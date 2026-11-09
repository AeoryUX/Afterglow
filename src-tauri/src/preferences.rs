use crate::{private_fs::Directory, Settings};
use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path, sync::Mutex};
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Shortcut {
    pub modifiers: Vec<String>,
    pub key: String,
}
impl Default for Shortcut {
    fn default() -> Self {
        Self {
            modifiers: vec!["SUPER".into()],
            key: "F8".into(),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Shortcuts {
    pub enabled: bool,
    pub save: Shortcut,
    pub toggle: Shortcut,
    pub show: Shortcut,
}
impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            enabled: false,
            save: Shortcut::default(),
            toggle: Shortcut {
                key: "F9".into(),
                ..Shortcut::default()
            },
            show: Shortcut {
                key: "F10".into(),
                ..Shortcut::default()
            },
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub close_to_tray: bool,
    pub start_with_system: bool,
    pub show_window_on_login: bool,
    pub clip_sound: bool,
    pub auto_start_replay: bool,
    pub startup_policy_version: u32,
    pub start_in_tray: bool,
    pub storage_limit_mb: u32,
    pub shortcuts: Shortcuts,
    pub capture: Option<Settings>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            start_with_system: false,
            show_window_on_login: false,
            clip_sound: true,
            auto_start_replay: true,
            startup_policy_version: 1,
            start_in_tray: false,
            storage_limit_mb: 0,
            shortcuts: Shortcuts::default(),
            capture: None,
        }
    }
}
pub struct PreferencesState(pub Mutex<Preferences>);
pub fn validate(p: &Preferences) -> Result<(), String> {
    if p.storage_limit_mb != 0 && !(100..=2_000_000).contains(&p.storage_limit_mb) {
        return Err("Choose a capacity between 0.1 GB and 2,000 GB, or unlimited".into());
    }
    crate::hotkeys::validate(&p.shortcuts)?;
    if let Some(capture) = &p.capture {
        crate::capture_args(capture, "library")?;
    }
    Ok(())
}
pub fn load(path: &Path) -> Result<Preferences, String> {
    if !path.exists() {
        return Ok(Preferences::default());
    }
    let mut text = String::new();
    Directory::open(path.parent().ok_or("Missing preferences directory")?)
        .and_then(|d| d.read("preferences.json"))
        .and_then(|f| f.take(65537).read_to_string(&mut text))
        .map_err(|e| e.to_string())?;
    if text.len() > 65536 {
        return Err("Preferences are too large".into());
    }
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Cannot read preferences: {e}"))?;
    let needs_migration = value.get("startupPolicyVersion").is_none();
    let mut p: Preferences =
        serde_json::from_value(value).map_err(|e| format!("Cannot read preferences: {e}"))?;
    if needs_migration {
        p.auto_start_replay = true;
        p.startup_policy_version = 1;
    }
    validate(&p)?;
    Ok(p)
}
pub fn store(path: &Path, p: &Preferences) -> Result<(), String> {
    validate(p)?;
    let dir = path.parent().ok_or("Missing preferences directory")?;
    std::fs::create_dir_all(dir.parent().ok_or("Missing configuration directory")?)
        .map_err(|e| e.to_string())?;
    Directory::private(dir)
        .and_then(|d| {
            d.atomic_write(
                "preferences.json",
                &serde_json::to_vec_pretty(p).map_err(std::io::Error::other)?,
            )
        })
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_are_private_and_round_trip() {
        let root =
            std::env::temp_dir().join(crate::private_fs::temporary_name("settings").unwrap());
        let path = root.join("preferences.json");
        let mut prefs = Preferences::default();
        prefs.storage_limit_mb = 5000;
        store(&path, &prefs).unwrap();
        assert_eq!(load(&path).unwrap().storage_limit_mb, 5000);
        prefs.auto_start_replay = false;
        store(&path, &prefs).unwrap();
        assert!(!load(&path).unwrap().auto_start_replay);
        let mut old = serde_json::to_value(&prefs).unwrap();
        old.as_object_mut().unwrap().remove("startupPolicyVersion");
        std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        assert!(load(&path).unwrap().auto_start_replay);
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn invalid_capacity_is_rejected() {
        let mut p = Preferences::default();
        p.storage_limit_mb = 50;
        assert!(validate(&p).is_err());
        p.storage_limit_mb = 0;
        assert!(validate(&p).is_ok());
    }
}
