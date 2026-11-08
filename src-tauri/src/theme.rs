use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub available: bool,
    pub name: String,
    pub light: bool,
    pub colours: BTreeMap<String, String>,
    pub error: Option<String>,
}
#[derive(Default)]
pub struct Cache(Mutex<Option<(PathBuf, Option<(u64, u64, i64, i64)>, Theme)>>);
fn colour(value: &str) -> Option<String> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    (hex.len() == 6 && hex.bytes().all(|c| c.is_ascii_hexdigit())).then(|| format!("#{hex}"))
}
fn parse(data: &str) -> Result<Theme, String> {
    let json: serde_json::Value =
        serde_json::from_str(data).map_err(|_| "Invalid Caelestia scheme JSON")?;
    let values = json
        .get("colours")
        .and_then(|v| v.as_object())
        .ok_or("Caelestia scheme has no colours")?;
    let keys = [
        "primary",
        "onPrimary",
        "primaryContainer",
        "onPrimaryContainer",
        "surface",
        "onSurface",
        "surfaceContainerLow",
        "surfaceContainer",
        "surfaceContainerHigh",
        "onSurfaceVariant",
        "outline",
        "outlineVariant",
    ];
    let mut colours = BTreeMap::new();
    for key in keys {
        if let Some(value) = values.get(key).and_then(|v| v.as_str()).and_then(colour) {
            colours.insert(key.to_string(), value);
        }
    }
    if !["primary", "surface", "onSurface"]
        .iter()
        .all(|key| colours.contains_key(*key))
    {
        return Err("Caelestia scheme is missing core colours".into());
    }
    Ok(Theme {
        available: true,
        name: json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Caelestia")
            .chars()
            .take(100)
            .collect(),
        light: json.get("mode").and_then(|v| v.as_str()) == Some("light"),
        colours,
        error: None,
    })
}
pub fn read(path: &Path, cache: &Cache) -> Theme {
    let modified = fs::metadata(path)
        .ok()
        .map(|m| (m.ino(), m.len(), m.ctime(), m.ctime_nsec()));
    let Ok(mut stored) = cache.0.lock() else {
        return Theme::default();
    };
    if let Some((old_path, old_time, theme)) = stored.as_ref() {
        if old_path == path && modified.is_some() && *old_time == modified {
            return theme.clone();
        }
    }
    let result = (|| {
        let file = fs::File::open(path).map_err(|_| "Caelestia scheme not found".to_string())?;
        let mut data = String::new();
        file.take(65537)
            .read_to_string(&mut data)
            .map_err(|e| e.to_string())?;
        if data.len() > 65536 {
            return Err("Caelestia scheme exceeds 64 KB".into());
        }
        parse(&data)
    })();
    match result {
        Ok(theme) => {
            *stored = Some((path.to_path_buf(), modified, theme.clone()));
            theme
        }
        Err(error) => {
            // Scheme writers may truncate or replace the file before finishing.
            // Preserve the last usable palette, and retry even if metadata stays unchanged.
            let mut theme = stored
                .as_ref()
                .filter(|(old_path, _, _)| old_path == path)
                .map(|(_, _, theme)| theme.clone())
                .unwrap_or_default();
            theme.error = Some(error);
            *stored = Some((path.to_path_buf(), None, theme.clone()));
            theme
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_official_caelestia_shape() {
        let t=parse(r##"{"name":"Ocean","mode":"light","colours":{"primary":"00639b","surface":"f9f9ff","onSurface":"191c20","onPrimary":"ffffff"}}"##).unwrap();
        assert!(t.available && t.light);
        assert_eq!(t.colours["primary"], "#00639b");
    }
    #[test]
    fn rejects_css_and_incomplete_schemes() {
        assert!(colour("red; background:url(x)").is_none());
        assert!(colour("#123456").is_some());
        assert!(parse(r#"{"colours":{"primary":"bad"}}"#).is_err());
    }
    #[test]
    fn keeps_palette_during_partial_write_and_recovers() {
        let path =
            std::env::temp_dir().join(format!("afterglow-theme-{}.json", std::process::id()));
        let cache = Cache::default();
        let valid = r#"{"name":"Ocean","colours":{"primary":"00639b","surface":"f9f9ff","onSurface":"191c20"}}"#;
        fs::write(&path, valid).unwrap();
        assert!(read(&path, &cache).available);
        fs::write(&path, r#"{"name":"#).unwrap();
        let partial = read(&path, &cache);
        assert!(partial.available && partial.error.is_some());
        assert_eq!(partial.colours["primary"], "#00639b");
        fs::remove_file(&path).unwrap();
        assert!(read(&path, &cache).available);
        fs::write(&path, valid.replace("00639b", "123456")).unwrap();
        let recovered = read(&path, &cache);
        assert_eq!(recovered.colours["primary"], "#123456");
        assert!(recovered.error.is_none());
        fs::remove_file(path).unwrap();
    }
}
