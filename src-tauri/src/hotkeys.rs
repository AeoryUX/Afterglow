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
fn canonical_key(k: &str) -> &str {
    if k == "/" {
        "slash"
    } else {
        k
    }
}
fn valid_key(k: &str) -> bool {
    (k.len() == 1 && k.bytes().all(|c| c.is_ascii_alphanumeric()))
        || k.strip_prefix('F')
            .and_then(|s| s.parse::<u8>().ok())
            .is_some_and(|n| (1..=24).contains(&n))
        || [
            "slash",
            "/",
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
            || key.modifiers.len() > 4
            || key
                .modifiers
                .iter()
                .any(|m| !["SUPER", "CTRL", "ALT", "SHIFT"].contains(&m.as_str()))
        {
            return Err(
                "Choose a letter, number, F1–F24, slash or navigation key, optionally with a modifier"
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
    mask(a) == mask(b) && canonical_key(&a.key).eq_ignore_ascii_case(canonical_key(&b.key))
}
fn combo(k: &Shortcut) -> String {
    format!("{},{}", k.modifiers.join(" "), canonical_key(&k.key))
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
            .is_some_and(|s| canonical_key(s).eq_ignore_ascii_case(canonical_key(&k.key)))
}
fn marker(command: &str) -> String {
    format!("Afterglow:{}", command)
}
fn owned(v: &Value, command: &str, flag: &str) -> bool {
    v["description"].as_str() == Some(marker(command).as_str())
        || (v["dispatcher"].as_str() == Some("exec")
            && v["arg"].as_str().is_some_and(|arg| {
                arg == command
                    || arg == format!("afterglow {flag}")
                    || arg == format!("/usr/bin/afterglow {flag}")
            }))
}
#[derive(Clone, Copy)]
enum Backend {
    Legacy,
    Lua,
}
pub fn uses_lua() -> bool {
    matches!(backend(), Backend::Lua)
}
fn backend() -> Backend {
    if crate::system::command("hyprctl")
        .args(["eval", "assert(type(hl.bind) == 'function')"])
        .output()
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "ok")
    {
        Backend::Lua
    } else {
        Backend::Legacy
    }
}
fn lua_key(k: &Shortcut) -> String {
    let mut parts = k.modifiers.clone();
    parts.push(canonical_key(&k.key).into());
    parts.join(" + ")
}
fn lua_script(k: &Shortcut, command: Option<&str>) -> String {
    let id = serde_json::to_string(&format!("{}:{}", mask(k), canonical_key(&k.key))).unwrap();
    let base=format!("_G.__afterglow_desktop_bindings_v1 = _G.__afterglow_desktop_bindings_v1 or {{}}; local t = _G.__afterglow_desktop_bindings_v1; local id = {id}; if t[id] then t[id]:remove(); t[id] = nil end;");
    if let Some(command) = command {
        format!(
            "{base} t[id] = hl.bind({}, hl.dsp.exec_cmd({}), {{description = {}}});",
            serde_json::to_string(&lua_key(k)).unwrap(),
            serde_json::to_string(command).unwrap(),
            serde_json::to_string(&marker(command)).unwrap()
        )
    } else {
        base
    }
}
fn mutate(mode: Backend, k: &Shortcut, command: Option<&str>) -> Result<(), String> {
    let args = match mode {
        Backend::Legacy => vec![
            "keyword".into(),
            if command.is_some() { "bind" } else { "unbind" }.into(),
            command
                .map(|c| format!("{},exec,{c}", combo(k)))
                .unwrap_or_else(|| combo(k)),
        ],
        Backend::Lua => vec!["eval".into(), lua_script(k, command)],
    };
    let out = crate::system::command("hyprctl")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "ok" {
        Ok(())
    } else {
        Err(format!(
            "Hyprland could not apply shortcut: {} {}",
            String::from_utf8_lossy(&out.stdout).trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}
pub fn apply(state: &HotkeyState, shortcuts: &Shortcuts) -> Result<(), String> {
    validate(shortcuts)?;
    let mut installed = state.installed.lock().map_err(|e| e.to_string())?;
    if !shortcuts.enabled && installed.is_empty() {
        return Ok(());
    }
    let mode = backend();
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
        if let Err(error) = mutate(mode, key, Some(command)) {
            for (key, _) in &added {
                let _ = mutate(mode, key, None);
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
        if !found.is_empty() && found.iter().all(|v| owned(v, command, "--unrecognized")) {
            mutate(mode, key, None)?;
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
    fn single_keys_and_slash_are_valid_and_conflicts_are_detected() {
        let mut s = Shortcuts::default();
        s.save = Shortcut {
            modifiers: vec![],
            key: "F2".into(),
        };
        assert!(validate(&s).is_ok());
        assert_eq!(combo(&s.save), ",F2");
        s.save.key = "/".into();
        assert!(validate(&s).is_ok());
        assert_eq!(combo(&s.save), ",slash");
        assert!(matches(
            &serde_json::json!({"modmask":0,"key":"slash"}),
            &s.save
        ));
        s.toggle = Shortcut {
            modifiers: vec![],
            key: "slash".into(),
        };
        assert!(validate(&s).is_err());
    }
    #[test]
    fn lua_binding_uses_supported_api_and_quotes_commands() {
        let key = Shortcut {
            modifiers: vec![],
            key: "/".into(),
        };
        let code = lua_script(&key, Some("'/usr/bin/afterglow' --save-replay"));
        assert!(code.contains("hl.bind(\"slash\", hl.dsp.exec_cmd("));
        assert!(code.contains("description = \"Afterglow:"));
        assert!(lua_script(&key, None).contains(":remove()"));
        assert!(!lua_script(&key, None).contains("hl.unbind"));
        assert!(owned(
            &serde_json::json!({"dispatcher":"function","arg":"42","description":marker("cmd")}),
            "cmd",
            "--save-replay"
        ));
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
