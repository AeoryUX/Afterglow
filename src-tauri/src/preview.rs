use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
pub struct Preview {
    pub cache: PathBuf,
    pub generation: Arc<AtomicU64>,
}
impl Preview {
    pub fn new(cache: PathBuf) -> Result<Self, String> {
        let dir = crate::private_fs::Directory::private(&cache).map_err(|e| e.to_string())?;
        // Only this dedicated cache holds temporary preview files; discard leftovers from a prior run.
        if let Ok(entries) = std::fs::read_dir(&cache) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name == "preview.webm" || name.starts_with(".preview-") {
                        dir.remove(name);
                    }
                }
            }
        }
        Ok(Self {
            cache,
            generation: Arc::new(AtomicU64::new(0)),
        })
    }
    pub fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Ok(dir) = crate::private_fs::Directory::open(&self.cache) {
            dir.remove("preview.webm");
        }
    }
}

/// Transfer only the bounded, private preview, never arbitrary paths or original recordings.
pub fn read_bytes(cache: &std::path::Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = crate::private_fs::Directory::open(cache)
        .and_then(|d| d.read("preview.webm"))
        .map_err(|e| e.to_string())?;
    const LIMIT: u64 = 32 * 1024 * 1024;
    if file.metadata().map_err(|e| e.to_string())?.len() > LIMIT {
        return Err("Preview exceeds the 32 MiB playback limit".into());
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT || !bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        return Err("Preview is empty, incomplete, or too large".into());
    }
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_transfer_rejects_symlinks_and_oversized_files() {
        let root =
            std::env::temp_dir().join(crate::private_fs::temporary_name("preview-bytes").unwrap());
        std::fs::create_dir(&root).unwrap();
        let path = root.join("preview.webm");
        std::fs::write(&path, [0x1a, 0x45, 0xdf, 0xa3, 0]).unwrap();
        assert_eq!(read_bytes(&root).unwrap().len(), 5);
        let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(32 * 1024 * 1024 + 1).unwrap();
        assert!(read_bytes(&root).is_err());
        drop(f);
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", &path).unwrap();
        assert!(read_bytes(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
