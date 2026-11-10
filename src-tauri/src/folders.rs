//! Virtual clip folders: file locations and recorder output remain unchanged.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Folder {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Data {
    pub folders: Vec<Folder>,
    pub assignments: BTreeMap<String, String>,
}
pub struct Folders {
    path: PathBuf,
    data: Mutex<Result<Data, String>>,
}
fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|c| c.is_ascii_hexdigit())
}
fn label(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > 80
        || name.chars().any(|c| c.is_control() || "/\\".contains(c))
    {
        Err("Choose a folder name of 1–80 characters without slashes or control characters".into())
    } else {
        Ok(name.into())
    }
}
fn validate(d: &Data) -> Result<(), String> {
    if d.folders.len() > 256 || d.assignments.len() > 20_000 {
        return Err("Folder index limit reached".into());
    }
    for (i, f) in d.folders.iter().enumerate() {
        label(&f.name)?;
        if !valid_id(&f.id)
            || d.folders[..i]
                .iter()
                .any(|o| o.id == f.id || o.name.to_lowercase() == f.name.to_lowercase())
        {
            return Err("Invalid or duplicate folder".into());
        }
    }
    for (name, id) in &d.assignments {
        crate::clips::validate_name(name)?;
        if !d.folders.iter().any(|f| f.id == *id) {
            return Err("Clip refers to an unknown folder".into());
        }
    }
    Ok(())
}
impl Folders {
    pub fn new(path: PathBuf) -> Self {
        let data = load(&path);
        Self {
            path,
            data: Mutex::new(data),
        }
    }
    pub fn snapshot(&self) -> Result<Data, String> {
        self.data.lock().map_err(|e| e.to_string())?.clone()
    }
    fn write(&self, d: &Data) -> Result<(), String> {
        validate(d)?;
        let bytes = serde_json::to_vec_pretty(d).map_err(|e| e.to_string())?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err("Folder index is too large".into());
        }
        crate::private_fs::Directory::private(
            self.path
                .parent()
                .ok_or("Missing folder metadata directory")?,
        )
        .and_then(|dir| dir.atomic_write("folders.json", &bytes))
        .map_err(|e| e.to_string())
    }

    fn change(&self, f: impl FnOnce(&mut Data) -> Result<(), String>) -> Result<Data, String> {
        let mut slot = self.data.lock().map_err(|e| e.to_string())?;
        let mut next = slot.clone()?;
        f(&mut next)?;
        self.write(&next)?;
        *slot = Ok(next.clone());
        Ok(next)
    }
    pub fn create(&self, name: &str) -> Result<Data, String> {
        let name = label(name)?;
        let id = crate::private_fs::temporary_name("folder")
            .map_err(|e| e.to_string())?
            .trim_start_matches(".folder-")
            .to_owned();
        self.change(|d| {
            d.folders.push(Folder { id, name });
            Ok(())
        })
    }
    pub fn rename(&self, id: &str, name: &str) -> Result<Data, String> {
        let name = label(name)?;
        self.change(|d| {
            d.folders
                .iter_mut()
                .find(|f| f.id == id)
                .ok_or("Folder no longer exists")?
                .name = name;
            Ok(())
        })
    }
    pub fn remove(&self, id: &str) -> Result<Data, String> {
        self.change(|d| {
            if !d.folders.iter().any(|f| f.id == id) {
                return Err("Folder no longer exists".into());
            }
            d.folders.retain(|f| f.id != id);
            d.assignments.retain(|_, value| value != id);
            Ok(())
        })
    }
    pub fn assign(&self, name: &str, id: Option<&str>) -> Result<Data, String> {
        crate::clips::validate_name(name)?;
        self.change(|d| {
            d.assignments.remove(name);
            if let Some(id) = id {
                if !d.folders.iter().any(|f| f.id == id) {
                    return Err("Folder no longer exists".into());
                }
                d.assignments.insert(name.into(), id.into());
            }
            Ok(())
        })
    }
    pub fn renamed_clip(&self, root: &Path, old: &str, new: &str) -> Result<(), String> {
        let mut slot = self.data.lock().map_err(|e| e.to_string())?;
        let mut next = slot.clone()?;
        let folder = next.assignments.remove(old);
        next.assignments.remove(new);
        if let Some(id) = folder {
            next.assignments.insert(new.into(), id);
        }
        crate::clips::rename(root, old, new)?;
        if let Err(error) = self.write(&next) {
            let rollback = crate::clips::rename(root, new, old);
            return Err(if rollback.is_ok() {
                format!(
                    "Rename was rolled back because folder metadata could not be saved: {error}"
                )
            } else {
                format!(
                    "Clip was renamed to {new}, but folder metadata could not be saved: {error}"
                )
            });
        }
        *slot = Ok(next);
        Ok(())
    }
    pub fn inherit(&self, source: &str, new: &str) -> Result<(), String> {
        let id = self.snapshot()?.assignments.get(source).cloned();
        self.assign(new, id.as_deref()).map(|_| ())
    }
}
fn load(path: &Path) -> Result<Data, String> {
    let result =
        crate::private_fs::Directory::open(path.parent().ok_or("Missing config directory")?)
            .and_then(|dir| dir.read("folders.json"));
    let file = match result {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Data::default()),
        Err(e) => return Err(format!("Cannot read folder index: {e}")),
    };
    let mut bytes = Vec::new();
    file.take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Folder index is too large".into());
    }
    let d: Data = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Folder index needs repair; it was not overwritten: {e}"))?;
    validate(&d)?;
    Ok(d)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folders_round_trip_rename_inherit_and_remove_preserve_files() {
        let root = std::env::temp_dir().join(crate::private_fs::temporary_name("folders").unwrap());
        std::fs::create_dir(&root).unwrap();
        let path = root.join("folders.json");
        let store = Folders::new(path.clone());
        let data = store.create(" Ranked ").unwrap();
        let id = data.folders[0].id.clone();
        assert!(store.create("ranked").is_err());
        assert!(store.create("../bad").is_err());
        std::fs::write(root.join("clip.mp4"), b"original").unwrap();
        store.assign("clip.mp4", Some(&id)).unwrap();
        store.renamed_clip(&root, "clip.mp4", "new.mp4").unwrap();
        assert_eq!(store.snapshot().unwrap().assignments["new.mp4"], id);
        store.inherit("new.mp4", "trim.mp4").unwrap();
        assert_eq!(store.snapshot().unwrap().assignments["trim.mp4"], id);
        store.rename(&id, "Highlights").unwrap();
        assert_eq!(
            Folders::new(path).snapshot().unwrap().folders[0].name,
            "Highlights"
        );
        store.remove(&id).unwrap();
        assert!(store.snapshot().unwrap().assignments.is_empty());
        assert_eq!(std::fs::read(root.join("new.mp4")).unwrap(), b"original");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn metadata_failure_rolls_back_a_clip_rename() {
        let root = std::env::temp_dir()
            .join(crate::private_fs::temporary_name("folder-rollback").unwrap());
        std::fs::create_dir(&root).unwrap();
        let path = root.join("folders.json");
        let store = Folders::new(path.clone());
        store.create("Clips").unwrap();
        std::fs::write(root.join("one.mp4"), b"keep").unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(store
            .renamed_clip(&root, "one.mp4", "two.mp4")
            .unwrap_err()
            .contains("rolled back"));
        assert_eq!(std::fs::read(root.join("one.mp4")).unwrap(), b"keep");
        assert!(!root.join("two.mp4").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_metadata_is_preserved_and_not_overwritten() {
        let root = std::env::temp_dir().join(crate::private_fs::temporary_name("folders").unwrap());
        std::fs::create_dir(&root).unwrap();
        let path = root.join("folders.json");
        std::fs::write(&path, b"broken").unwrap();
        let store = Folders::new(path.clone());
        assert!(store.snapshot().is_err());
        assert!(store.create("New").is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"broken");
        std::fs::remove_dir_all(root).unwrap();
    }
}
