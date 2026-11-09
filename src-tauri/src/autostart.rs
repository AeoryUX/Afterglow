//! Own login entry plus a marked startup hook for standalone Hyprland sessions.
use std::{
    fs,
    io::{self, Read},
    os::unix::fs::{DirBuilderExt, MetadataExt},
    path::{Path, PathBuf},
};
const ENTRY: &str = "dev.afterglow.desktop.desktop";
fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let dir = crate::private_fs::Directory::open(path.parent().unwrap())?;
    match dir.read(
        path.file_name()
            .unwrap()
            .to_str()
            .ok_or_else(|| io::Error::other("Invalid config filename"))?,
    ) {
        Ok(f) => {
            let m = f.metadata()?;
            if m.uid() != unsafe { libc::geteuid() } || m.nlink() != 1 {
                return Err(io::Error::other(
                    "Unsafe login config ownership or hard link",
                ));
            }
            let mut b = Vec::new();
            f.take(1024 * 1024 + 1).read_to_end(&mut b)?;
            if b.len() > 1024 * 1024 {
                return Err(io::Error::other("Login configuration is too large"));
            }
            Ok(Some(b))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
fn desktop_quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
            .replace('%', "%%")
    )
}
fn hook(text: &str, lua: bool, command: &str, enable: bool) -> io::Result<String> {
    let prefix = if lua { "--" } else { "#" };
    let begin = format!("{prefix} Afterglow login startup begin");
    let end = format!("{prefix} Afterglow login startup end");
    let mut clean = text.to_string();
    if let Some(start) = clean.find(&begin) {
        let tail = clean[start..].find(&end).ok_or_else(|| {
            io::Error::other("Incomplete Afterglow startup block; preserve it for review")
        })? + start
            + end.len();
        let tail = tail + usize::from(clean.as_bytes().get(tail) == Some(&b'\n'));
        clean.replace_range(start..tail, "");
    }
    if enable {
        let code = if lua {
            format!(
                "hl.on(\"hyprland.start\", function() hl.exec_cmd({}) end)",
                serde_json::to_string(command).unwrap()
            )
        } else {
            format!("exec-once = {command}")
        };
        if !clean.ends_with('\n') && !clean.is_empty() {
            clean.push('\n');
        }
        clean.push_str(&format!("{begin}\n{code}\n{end}\n"));
    }
    Ok(clean)
}
pub fn apply(
    root: &Path,
    exe: &Path,
    enabled: bool,
    hyprland: bool,
    lua: bool,
) -> Result<(), String> {
    apply_inner(root, exe, enabled, hyprland, lua)
        .map_err(|e| format!("Could not update login startup: {e}"))
}
fn apply_inner(
    root: &Path,
    exe: &Path,
    enabled: bool,
    hyprland: bool,
    lua: bool,
) -> io::Result<()> {
    let exe = exe
        .to_str()
        .ok_or_else(|| io::Error::other("Executable path must be UTF-8"))?;
    if !Path::new(exe).is_absolute() || exe.chars().any(char::is_control) {
        return Err(io::Error::other("Invalid executable path"));
    }
    fs::create_dir_all(root)?;
    let folder = root.join("autostart");
    match fs::DirBuilder::new().mode(0o700).create(&folder) {
        Ok(()) => (),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e),
    };
    let entry = folder.join(ENTRY);
    let old = read(&entry)?;
    if old
        .as_ref()
        .is_some_and(|b| !b.starts_with(b"# Managed by Afterglow\n"))
    {
        return Err(io::Error::other(
            "An existing login entry is not managed by Afterglow",
        ));
    }
    let new=enabled.then(||format!("# Managed by Afterglow\n[Desktop Entry]\nType=Application\nName=Afterglow\nExec={} --autostart\nTryExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",desktop_quote(exe),exe).into_bytes());
    let mut changes: Vec<(PathBuf, Option<Vec<u8>>, Option<Vec<u8>>)> = vec![(entry, old, new)];
    for is_lua in [false, true] {
        let path = root.join("hypr").join(if is_lua {
            "hyprland.lua"
        } else {
            "hyprland.conf"
        });
        if !path.parent().unwrap().exists() {
            continue;
        }
        // Follow a user-owned config symlink to its existing target, preserving the symlink itself.
        let path = if path.is_symlink() {
            path.canonicalize()?
        } else {
            path
        };
        let Some(old) = read(&path)? else { continue };
        let text = std::str::from_utf8(&old).map_err(io::Error::other)?;
        let new = hook(
            text,
            is_lua,
            &format!("{} --autostart", shell_quote(exe)),
            enabled && hyprland && is_lua == lua,
        )?
        .into_bytes();
        if old != new {
            changes.push((path, Some(old), Some(new)));
        }
    }
    for (i, (path, _, new)) in changes.iter().enumerate() {
        if let Err(e) = write(path, new.as_deref()) {
            for (p, old, _) in changes[..i].iter().rev() {
                let _ = write(p, old.as_deref());
            }
            return Err(e);
        }
    }
    Ok(())
}
fn write(path: &Path, data: Option<&[u8]>) -> io::Result<()> {
    let dir = crate::private_fs::Directory::open(path.parent().unwrap())?;
    let name = path
        .file_name()
        .unwrap()
        .to_str()
        .ok_or_else(|| io::Error::other("Invalid filename"))?;
    if let Some(data) = data {
        dir.atomic_write(name, data)
    } else {
        dir.remove_checked(name)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn login_registration_round_trip_preserves_foreign_config() {
        for lua in [false, true] {
            let root =
                std::env::temp_dir().join(crate::private_fs::temporary_name("login").unwrap());
            fs::create_dir_all(root.join("hypr")).unwrap();
            let config = root
                .join("hypr")
                .join(if lua { "hyprland.lua" } else { "hyprland.conf" });
            let original = if lua {
                "-- user config\n"
            } else {
                "# user config\n"
            };
            fs::write(&config, original).unwrap();
            apply(&root, Path::new("/usr/bin/afterglow"), true, true, lua).unwrap();
            apply(&root, Path::new("/usr/bin/afterglow"), true, true, lua).unwrap();
            let t = fs::read_to_string(&config).unwrap();
            assert_eq!(t.matches("login startup begin").count(), 1);
            assert!(t.contains(if lua { "hl.on" } else { "exec-once" }));
            let entry = fs::read_to_string(root.join("autostart").join(ENTRY)).unwrap();
            assert!(entry.contains("Exec=\"/usr/bin/afterglow\" --autostart"));
            apply(&root, Path::new("/usr/bin/afterglow"), false, true, lua).unwrap();
            assert!(!root.join("autostart").join(ENTRY).exists());
            assert_eq!(fs::read_to_string(config).unwrap().trim(), original.trim());
            fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    fn refuses_to_replace_foreign_login_entry() {
        let root = std::env::temp_dir().join(crate::private_fs::temporary_name("login").unwrap());
        fs::create_dir_all(root.join("autostart")).unwrap();
        let p = root.join("autostart").join(ENTRY);
        fs::write(&p, "foreign").unwrap();
        assert!(apply(&root, Path::new("/usr/bin/afterglow"), true, false, false).is_err());
        assert_eq!(fs::read_to_string(p).unwrap(), "foreign");
        fs::remove_dir_all(root).unwrap();
    }
}
