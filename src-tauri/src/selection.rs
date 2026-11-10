//! Bounded multi-clip actions; invalid selections are rejected before mutation.
use serde::Serialize;
use std::{collections::HashSet, path::Path};
pub const LIMIT: usize = 500;
pub fn validate(names: &[String]) -> Result<(), String> {
    if names.is_empty() || names.len() > LIMIT {
        return Err(format!("Select between 1 and {LIMIT} clips"));
    }
    let mut seen = HashSet::new();
    for name in names {
        crate::clips::validate_name(name)?;
        if !seen.insert(name) {
            return Err("A clip was selected more than once".into());
        }
    }
    Ok(())
}
#[derive(Serialize)]
pub struct Failed {
    pub name: String,
    pub error: String,
}
#[derive(Default, Serialize)]
pub struct Deleted {
    pub deleted: Vec<String>,
    pub failed: Vec<Failed>,
}
pub fn delete(root: &Path, names: &[String]) -> Result<Deleted, String> {
    validate(names)?;
    let mut result = Deleted::default();
    for name in names {
        match crate::clips::delete(root, name) {
            Ok(()) => result.deleted.push(name.clone()),
            Err(error) => result.failed.push(Failed {
                name: name.clone(),
                error,
            }),
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_selection_deletes_nothing_and_partial_failure_is_explicit() {
        let root =
            std::env::temp_dir().join(crate::private_fs::temporary_name("selection").unwrap());
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("one.mp4"), b"one").unwrap();
        std::fs::write(root.join("two.mp4"), b"two").unwrap();
        std::os::unix::fs::symlink(root.join("two.mp4"), root.join("link.mp4")).unwrap();
        assert!(delete(&root, &["one.mp4".into(), "../two.mp4".into()]).is_err());
        assert!(root.join("one.mp4").exists());
        assert!(validate(&["one.mp4".into(), "one.mp4".into()]).is_err());
        assert!(validate(&[]).is_err());
        assert!(validate(&vec!["one.mp4".into(); 501]).is_err());
        let result = delete(&root, &["one.mp4".into(), "link.mp4".into()]).unwrap();
        assert_eq!(result.deleted, vec!["one.mp4"]);
        assert_eq!(result.failed[0].name, "link.mp4");
        assert_eq!(std::fs::read(root.join("two.mp4")).unwrap(), b"two");
        std::fs::remove_dir_all(root).unwrap();
    }
}
