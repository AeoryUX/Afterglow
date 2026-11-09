//! Runtime Hyprland bindings. Never rewrite compositor configuration or replace other apps' shortcuts.
use crate::preferences::{Shortcut, Shortcuts};
use serde_json::Value;
use std::sync::Mutex;
#[derive(Default)]
pub struct HotkeyState {
    pub installed: Mutex<Vec<(Shortcut, String)>>,
    pub message: Mutex<String>,
}
pub fn mask(s: &Shortcut) -> u64 {
    s.modifiers
        .iter()
        .map(|m| match m.as_str() {
            "SUPER" => 64,
            "CTRL" => 4,
            "ALT" => 8,
            "SHIFT" => 1,
            _ => 0,
        })
        .fold(0, |a, b| a | b)
}
fn valid_key(k: &str) -> bool {
    (k.len() == 1 && k.bytes().all(|c| c.is_ascii_alphanumeric()))
        || k.strip_prefix('F')
            .and_then(|s| s.parse::<u8>().ok())
            .is_some_and(|n| (1..=24).contains(&n))
        || [
            "space",
            "Return",
            "Tab",
            "Home",
            "End",
            "Insert",
            "Delete",
            "Pause",
            "Print",
            "Left",
            "Right",
            "Up",
            "Down",
            "Page_Up",
            "Page_Down",
        ]
        .contains(&k)
}
pub fn validate(s: &Shortcuts) -> Result<(), String> {
    let keys = [&s.save, &s.toggle, &s.show];
    for key in keys {
        if !valid_key(&key.key)
            || key.modifiers.is_empty()
            || key.modifiers.len() > 4
            || key
                .modifiers
                .iter()
                .any(|m| !["SUPER", "CTRL", "ALT", "SHIFT"].contains(&m.as_str()))
        {
            return Err(
                "Choose a letter, number, F1–F24 or navigation key with Super, Ctrl, Alt or Shift"
                    .into(),
            );
        }
    }
    for i in 0..keys.len() {
        for j in i + 1..keys.len() {
            if same(keys[i], keys[j]) {
                return Err("Each action needs a different shortcut".into());
            }
        }
    }
    Ok(())
}
fn same(a: &Shortcut, b: &Shortcut) -> bool {
    mask(a) == mask(b) && a.key.eq_ignore_ascii_case(&b.key)
}
fn combo(k: &Shortcut) -> String {
    format!("{},{}", k.modifiers.join(" "), k.key)
}
fn query() -> Result<Vec<Value>, String> {
    let out=crate::system::command("hyprctl").args(["-j","binds"]).output().map_err(|_|"Hyprland shortcut setup requires hyprctl. Use the commands below in your desktop's shortcut settings.")?;
    if !out.status.success() {
        return Err("Could not connect to Hyprland. Set these shortcuts in your desktop's keyboard settings instead.".into());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}
fn matches(v: &Value, k: &Shortcut) -> bool {
    v["modmask"].as_u64() == Some(mask(k))
        && v["key"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case(&k.key))
}
fn owned(v: &Value, command: &str, flag: &str) -> bool {
    v["dispatcher"].as_str() == Some("exec")
        && v["arg"].as_str().is_some_and(|arg| {
            arg == command
                || arg == format!("afterglow {flag}")
                || arg == format!("/usr/bin/afterglow {flag}")
        })
}
fn keyword(key: &str, value: &str) -> Result<(), String> {
    let out = crate::system::command("hyprctl")
        .args(["keyword", key, value])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "ok" {
        Ok(())
    } else {
        Err(format!(
            "Hyprland could not apply shortcut: {}",
            String::from_utf8_lossy(&out.stdout).trim()
        ))
    }
}
pub fn apply(state: &HotkeyState, shortcuts: &Shortcuts) -> Result<(), String> {
    validate(shortcuts)?;
    let mut installed = state.installed.lock().map_err(|e| e.to_string())?;
    if !shortcuts.enabled && installed.is_empty() {
        return Ok(());
    }
    let mut current = query()?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let quoted = format!("'{}'", executable.to_string_lossy().replace('\'', "'\\''"));
    let desired = if shortcuts.enabled {
        vec![
            (
                shortcuts.save.clone(),
                format!("{quoted} --save-replay"),
                "--save-replay",
            ),
            (
                shortcuts.toggle.clone(),
                format!("{quoted} --toggle-replay"),
                "--toggle-replay",
            ),
            (shortcuts.show.clone(), format!("{quoted} --show"), "--show"),
        ]
    } else {
        vec![]
    };
    // Check every requested combination before changing any binding.
    for (key, command, flag) in &desired {
        if current
            .iter()
            .any(|v| matches(v, key) && !owned(v, command, flag))
        {
            return Err(format!("{} is already used by another action. Choose a different shortcut; the existing action was kept.",combo(key)));
        }
    }
    let mut added = Vec::new();
    for (key, command, flag) in &desired {
        if current
            .iter()
            .any(|v| matches(v, key) && owned(v, command, flag))
        {
            continue;
        }
        if let Err(error) = keyword("bind", &format!("{},exec,{}", combo(key), command)) {
            for (key, _) in &added {
                let _ = keyword("unbind", &combo(key));
            }
            return Err(error);
        }
        added.push((key.clone(), command.clone()));
    }
    // Remove only bindings this process installed, after checking for foreign bindings again.
    current = query()?;
    for (key, command) in installed.iter() {
        if desired.iter().any(|(new, _, _)| same(key, new)) {
            continue;
        }
        let found = current
            .iter()
            .filter(|v| matches(v, key))
            .collect::<Vec<_>>();
        if !found.is_empty()
            && found.iter().all(|v| {
                v["dispatcher"].as_str() == Some("exec") && v["arg"].as_str() == Some(command)
            })
        {
            keyword("unbind", &combo(key))?;
        }
    }
    // Adopt equivalent existing Afterglow bindings without duplicating them.
    *installed = desired
        .into_iter()
        .map(|(k, c, _)| {
            let existing = current
                .iter()
                .find(|v| matches(v, &k) && v["dispatcher"].as_str() == Some("exec"))
                .and_then(|v| v["arg"].as_str());
            (k, existing.unwrap_or(&c).to_string())
        })
        .collect();
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shortcut_validation_rejects_injection_and_duplicates() {
        let mut s = Shortcuts::default();
        assert!(validate(&s).is_ok());
        s.save.key = "F8; touch bad".into();
        assert!(validate(&s).is_err());
        s = Shortcuts::default();
        s.toggle = s.save.clone();
        assert!(validate(&s).is_err());
    }
    #[test]
    fn detects_foreign_bindings() {
        let k = Shortcut::default();
        let v =
            serde_json::json!({"modmask":64,"key":"F8","dispatcher":"exec","arg":"some-other-app"});
        assert!(matches(&v, &k));
        assert!(!owned(
            &v,
            "'/usr/bin/afterglow' --save-replay",
            "--save-replay"
        ));
    }
}
