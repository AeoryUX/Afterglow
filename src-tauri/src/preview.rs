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
