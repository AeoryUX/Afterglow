//! File descriptors anchor operations to a directory, avoiding check/reopen races.
use std::{
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::Path,
};

pub struct Directory(File);
impl Directory {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(path)?;
        Ok(Self(file))
    }
    pub fn private(path: &Path) -> io::Result<Self> {
        match fs::DirBuilder::new().mode(0o700).create(path) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e),
        }
        let dir = Self::open(path)?;
        if dir.0.metadata()?.uid() != unsafe { libc::geteuid() } {
            return Err(io::Error::other(
                "Private directory belongs to another user",
            ));
        }
        dir.0.set_permissions(fs::Permissions::from_mode(0o700))?;
        Ok(dir)
    }
    fn name(name: &str) -> io::Result<CString> {
        if name.is_empty() || name == "." || name == ".." || name.contains('/') {
            return Err(io::Error::other("Invalid relative filename"));
        }
        CString::new(name).map_err(io::Error::other)
    }
    fn open_at(&self, name: &str, flags: i32) -> io::Result<File> {
        let name = Self::name(name)?;
        let fd = unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        if !file.metadata()?.is_file() {
            return Err(io::Error::other("Only regular files are accepted"));
        }
        Ok(file)
    }
    pub fn read(&self, name: &str) -> io::Result<File> {
        self.open_at(name, libc::O_RDONLY)
    }
    pub fn create(&self, name: &str) -> io::Result<File> {
        self.open_at(name, libc::O_RDWR | libc::O_CREAT | libc::O_EXCL)
    }
    pub fn private_log(&self, name: &str) -> io::Result<File> {
        let file = self.append_log(name)?;
        file.set_len(0)?;
        Ok(file)
    }
    pub fn append_log(&self, name: &str) -> io::Result<File> {
        let file = self.open_at(name, libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND)?;
        let meta = file.metadata()?;
        if meta.uid() != unsafe { libc::geteuid() } || meta.nlink() != 1 {
            return Err(io::Error::other("Unsafe log ownership or hard link"));
        }
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        Ok(file)
    }
    pub fn rename(&self, old: &str, new: &str, no_replace: bool) -> io::Result<()> {
        let (old, new) = (Self::name(old)?, Self::name(new)?);
        let rc = unsafe {
            libc::renameat2(
                self.0.as_raw_fd(),
                old.as_ptr(),
                self.0.as_raw_fd(),
                new.as_ptr(),
                if no_replace {
                    libc::RENAME_NOREPLACE
                } else {
                    0
                },
            )
        };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    pub fn remove_checked(&self, name: &str) -> io::Result<()> {
        let name = Self::name(name)?;
        if unsafe { libc::unlinkat(self.0.as_raw_fd(), name.as_ptr(), 0) } == 0 {
            Ok(())
        } else {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
    pub fn remove(&self, name: &str) {
        let _ = self.remove_checked(name);
    }
    pub fn atomic_write(&self, name: &str, data: &[u8]) -> io::Result<()> {
        let temporary = temporary_name("state")?;
        let mut file = self.create(&temporary)?;
        let result = (|| {
            file.write_all(data)?;
            self.rename(&temporary, name, false)
        })();
        self.remove(&temporary);
        result
    }
}
use std::os::unix::fs::DirBuilderExt;
pub fn temporary_name(prefix: &str) -> io::Result<String> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(format!(
        ".{prefix}-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_fifo_without_blocking() {
        let root = std::env::temp_dir().join(temporary_name("fifo").unwrap());
        let dir = Directory::private(&root).unwrap();
        let name = CString::new(root.join("pipe").as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(dir.read("pipe").is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn private_modes_and_link_defenses() {
        let root = std::env::temp_dir().join(temporary_name("private-fs").unwrap());
        let dir = Directory::private(&root).unwrap();
        assert_eq!(fs::metadata(&root).unwrap().mode() & 0o777, 0o700);
        dir.atomic_write("pid", b"123").unwrap();
        assert_eq!(
            fs::metadata(root.join("pid")).unwrap().mode() & 0o777,
            0o600
        );
        fs::write(root.join("original"), b"keep").unwrap();
        std::os::unix::fs::symlink(root.join("original"), root.join("link")).unwrap();
        assert!(dir.read("link").is_err());
        assert!(dir.private_log("link").is_err());
        fs::hard_link(root.join("original"), root.join("hardlink")).unwrap();
        assert!(dir.private_log("hardlink").is_err());
        assert_eq!(fs::read(root.join("original")).unwrap(), b"keep");
        assert!(dir.create("original").is_err());
        // An opened descriptor stays on the original inode after the name changes.
        let mut opened = dir.read("original").unwrap();
        fs::rename(root.join("original"), root.join("moved")).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", root.join("original")).unwrap();
        let mut text = String::new();
        std::io::Read::read_to_string(&mut opened, &mut text).unwrap();
        assert_eq!(text, "keep");
        fs::remove_dir_all(root).unwrap();
    }
}
