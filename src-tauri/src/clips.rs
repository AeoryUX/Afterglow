use serde::Serialize;
use std::{
    ffi::CString,
    fs,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub modified: u64,
}

pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 180
        || name.starts_with('.')
        || !name.ends_with(".mp4")
        || name.chars().any(|c| c.is_control() || "/\\".contains(c))
    {
        return Err("Use an MP4 filename without slashes, hidden prefixes, or control characters (maximum 180 bytes)".into());
    }
    Ok(())
}
pub fn clip_path(root: &Path, name: &str) -> Result<PathBuf, String> {
    validate_name(name)?;
    let path = root.join(name);
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() {
        return Err("Only regular clips in the library can be edited".into());
    }
    Ok(path)
}
pub fn list(root: &Path) -> Result<Vec<Clip>, String> {
    let mut clips = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let e = entry.map_err(|e| e.to_string())?;
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if !e.file_type().map_err(|e| e.to_string())?.is_file()
            || e.path().extension().and_then(|s| s.to_str()) != Some("mp4")
        {
            continue;
        }
        let m = e.metadata().map_err(|e| e.to_string())?;
        if m.len() == 0 {
            continue;
        }
        clips.push(Clip {
            name: e.file_name().to_string_lossy().into_owned(),
            path: e.path().display().to_string(),
            bytes: m.len(),
            modified: m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });
    }
    clips.sort_by_key(|c| std::cmp::Reverse(c.modified));
    Ok(clips)
}
pub fn rename(root: &Path, name: &str, new_name: &str) -> Result<(), String> {
    let old = clip_path(root, name)?;
    validate_name(new_name)?;
    if name == new_name {
        return Ok(());
    }
    let old = CString::new(old.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    let new =
        CString::new(root.join(new_name).as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    // Linux atomic rename that cannot overwrite another clip.
    if unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            old.as_ptr(),
            libc::AT_FDCWD,
            new.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    } != 0
    {
        return Err(format!(
            "Cannot rename clip: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}
pub fn duration(path: &Path) -> Result<f64, String> {
    let out = crate::system::command("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .map_err(|e| format!("Cannot run ffprobe (install ffmpeg): {e}"))?;
    if !out.status.success() {
        return Err("Clip is still being written or cannot be read by FFprobe".into());
    }
    let duration: f64 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .map_err(|_| "Cannot read clip duration")?;
    if !duration.is_finite() || duration <= 0.0 {
        return Err("Clip has no valid duration".into());
    }
    Ok(duration)
}
pub fn validate_trim(start: f64, end: f64, length: f64) -> Result<(), String> {
    if !start.is_finite()
        || !end.is_finite()
        || !length.is_finite()
        || start < 0.0
        || end - start < 0.1
        || end > length + 0.05
    {
        return Err("Choose a range of at least 0.1 seconds within the clip".into());
    }
    Ok(())
}
pub fn trim(root: &Path, name: &str, start: f64, end: f64) -> Result<String, String> {
    let source = clip_path(root, name)?;
    validate_trim(start, end, duration(&source)?)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let output_name = format!("trim-{stamp}.mp4");
    let output = root.join(format!(".trim-{stamp}.mp4"));
    let log = root.join(format!(".trim-{stamp}.log"));
    let stderr = fs::File::create(&log).map_err(|e| e.to_string())?;
    // Re-encode for frame-accurate cuts. Capture continues using NVENC independently.
    let result = crate::system::command("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-n",
            "-ss",
            &start.to_string(),
            "-i",
        ])
        .arg(&source)
        .args([
            "-t",
            &(end - start).to_string(),
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-crf",
            "18",
            "-threads",
            "2",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
        ])
        .arg(&output)
        .stderr(stderr)
        .stdout(std::process::Stdio::null())
        .status();
    let details = fs::read_to_string(&log).unwrap_or_default();
    let _ = fs::remove_file(log);
    match result {
        Ok(status) if status.success() => {
            fs::rename(&output, root.join(&output_name)).map_err(|e| e.to_string())?;
            Ok(output_name)
        }
        result => {
            let _ = fs::remove_file(output);
            Err(format!(
                "Trim failed. Check FFmpeg/libx264 installation. {} {}",
                result.err().map(|e| e.to_string()).unwrap_or_default(),
                details.chars().take(1000).collect::<String>()
            ))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_path_escape() {
        for name in [
            "../clip.mp4",
            "/tmp/test.mp4",
            ".hidden.mp4",
            "a\\b.mp4",
            "x\n.mp4",
            "x.mkv",
            "",
        ] {
            assert!(validate_name(name).is_err(), "{name}");
        }
        assert!(validate_name("A great play.mp4").is_ok());
    }
    #[test]
    fn validates_trim_range() {
        assert!(validate_trim(1.0, 3.0, 5.0).is_ok());
        for (a, b) in [
            (-1.0, 2.0),
            (4.0, 3.0),
            (0.0, 6.0),
            (f64::NAN, 2.0),
            (0.0, f64::INFINITY),
            (1.0, 1.05),
        ] {
            assert!(validate_trim(a, b, 5.0).is_err());
        }
    }
    #[test]
    fn rename_never_overwrites_and_rejects_symlink() {
        let root = std::env::temp_dir().join(format!("afterglow-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("first.mp4"), b"first").unwrap();
        fs::write(root.join("second.mp4"), b"second").unwrap();
        assert!(rename(&root, "first.mp4", "second.mp4").is_err());
        assert_eq!(fs::read(root.join("second.mp4")).unwrap(), b"second");
        rename(&root, "first.mp4", "renamed.mp4").unwrap();
        assert!(!root.join("first.mp4").exists());
        std::os::unix::fs::symlink(root.join("second.mp4"), root.join("link.mp4")).unwrap();
        assert!(clip_path(&root, "link.mp4").is_err());
        assert_eq!(list(&root).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod media_tests {
    use super::*;
    #[test]
    #[ignore = "requires ffmpeg with libx264 and ffprobe"]
    fn exports_a_precise_trim_and_preserves_source() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("afterglow-media-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("original.mp4");
        let status = crate::system::command("ffmpeg")
            .args([
                "-v",
                "error",
                "-nostdin",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=320x180:rate=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000",
                "-t",
                "4",
                "-c:v",
                "libx264",
                "-threads",
                "2",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ])
            .arg(&source)
            .status()
            .unwrap();
        assert!(status.success());
        let original = fs::read(&source).unwrap();
        let output = trim(&root, "original.mp4", 1.0, 2.5).unwrap();
        assert!((duration(&root.join(output)).unwrap() - 1.5).abs() < 0.15);
        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(list(&root).unwrap().len(), 2);
        assert!(trim(&root, "original.mp4", 0.0, 100.0).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
