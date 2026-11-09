use serde::Serialize;
use std::{fs, path::Path, time::UNIX_EPOCH};

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
pub fn open_clip(root: &Path, name: &str) -> Result<fs::File, String> {
    validate_name(name)?;
    crate::private_fs::Directory::open(root)
        .and_then(|dir| dir.read(name))
        .map_err(|e| e.to_string())
}
pub fn list(root: &Path) -> Result<Vec<Clip>, String> {
    let mut clips = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let e = entry.map_err(|e| e.to_string())?;
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let Some(name) = e.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(file) = open_clip(root, &name) else {
            continue;
        };
        let m = file.metadata().map_err(|e| e.to_string())?;
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
    validate_name(name)?;
    validate_name(new_name)?;
    let dir = crate::private_fs::Directory::open(root).map_err(|e| e.to_string())?;
    // Rename is anchored to this directory and never follows a source symlink.
    dir.read(name).map_err(|e| e.to_string())?;
    if name == new_name {
        return Ok(());
    }
    dir.rename(name, new_name, true)
        .map_err(|e| format!("Cannot rename clip: {e}"))
}
pub fn delete(root: &Path, name: &str) -> Result<(), String> {
    validate_name(name)?;
    let dir = crate::private_fs::Directory::open(root).map_err(|e| e.to_string())?;
    dir.read(name).map_err(|e| e.to_string())?;
    dir.remove_checked(name).map_err(|e| e.to_string())
}
pub fn is_processing() -> bool {
    MEDIA_JOB.try_lock().is_err()
}
static MEDIA_JOB: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub fn duration(root: &Path, name: &str) -> Result<f64, String> {
    let _job = MEDIA_JOB
        .try_lock()
        .map_err(|_| "Another video operation is running. Try again when it finishes.")?;
    probe(open_clip(root, name)?)
}
fn probe(source: fs::File) -> Result<f64, String> {
    let mut command = crate::processing::command("ffprobe", source);
    command.args([
        "-v",
        "error",
        "-protocol_whitelist",
        "file",
        "-f",
        "mov",
        "-show_entries",
        "format=duration",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
        "/proc/self/fd/0",
    ]);
    let out = crate::processing::run(command, std::time::Duration::from_secs(10))?;
    let duration: f64 = String::from_utf8_lossy(&out)
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
#[cfg(test)]
pub fn trim(root: &Path, name: &str, start: f64, end: f64) -> Result<String, String> {
    trim_limited(root, name, start, end, None)
}
#[cfg(test)]
pub fn trim_limited(
    root: &Path,
    name: &str,
    start: f64,
    end: f64,
    max_output_bytes: Option<u64>,
) -> Result<String, String> {
    trim_with_mode(root, name, start, end, max_output_bytes, true)
}
pub fn trim_with_mode(
    root: &Path,
    name: &str,
    start: f64,
    end: f64,
    max_output_bytes: Option<u64>,
    precise: bool,
) -> Result<String, String> {
    use std::{
        io::{Seek, SeekFrom},
        process::Stdio,
    };
    let _job = MEDIA_JOB
        .try_lock()
        .map_err(|_| "Another video operation is running. Try again when it finishes.")?;
    let mut source = open_clip(root, name)?;
    validate_trim(
        start,
        end,
        probe(source.try_clone().map_err(|e| e.to_string())?)?,
    )?;
    source.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let dir = crate::private_fs::Directory::open(root).map_err(|e| e.to_string())?;
    let temporary = crate::private_fs::temporary_name("trim").map_err(|e| e.to_string())?;
    let output_name = format!("{}.mp4", temporary.trim_start_matches('.'));
    let output = dir.create(&temporary).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut command = crate::processing::command("ffmpeg", source);
        command.stdout(Stdio::from(output));
        if let Some(limit) = max_output_bytes {
            use std::os::unix::process::CommandExt;
            unsafe {
                command.pre_exec(move || {
                    let limit = limit.min(1024 * 1024 * 1024);
                    let cap = libc::rlimit {
                        rlim_cur: limit,
                        rlim_max: limit,
                    };
                    if libc::setrlimit(libc::RLIMIT_FSIZE, &cap) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        command.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-protocol_whitelist",
            "file",
            "-threads",
            "2",
            "-ss",
            &start.to_string(),
            "-f",
            "mov",
            "-i",
            "/proc/self/fd/0",
            "-t",
            &(end - start).to_string(),
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
        ]);
        if precise {
            command.args([
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-tune",
                "zerolatency",
                "-crf",
                "18",
                "-threads",
                "2",
                "-filter_threads",
                "1",
                "-c:a",
                "aac",
            ]);
        } else {
            command.args(["-c", "copy", "-avoid_negative_ts", "make_zero"]);
        }
        command.args(["-f", "mp4", "-movflags", "+faststart", "/proc/self/fd/1"]);
        crate::processing::run(command, std::time::Duration::from_secs(600))?;
        dir.rename(&temporary, &output_name, true)
            .map_err(|e| e.to_string())?;
        Ok(output_name)
    })();
    dir.remove(&temporary);
    result
}
pub fn compatible_preview(
    root: &Path,
    name: &str,
    cache: &Path,
    cancelled: impl Fn() -> bool,
) -> Result<(), String> {
    use std::{os::unix::process::CommandExt, process::Stdio};
    let _job = MEDIA_JOB
        .try_lock()
        .map_err(|_| "Another video operation is running. Try again when it finishes.")?;
    let source = open_clip(root, name)?;
    let dir = crate::private_fs::Directory::private(cache).map_err(|e| e.to_string())?;
    let temp = crate::private_fs::temporary_name("preview").map_err(|e| e.to_string())?;
    let output = dir.create(&temp).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut command = crate::processing::command("ffmpeg", source);
        command.stdout(Stdio::from(output));
        unsafe {
            command.pre_exec(|| {
                let cap = libc::rlimit {
                    rlim_cur: 64 * 1024 * 1024,
                    rlim_max: 64 * 1024 * 1024,
                };
                if libc::setrlimit(libc::RLIMIT_FSIZE, &cap) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        command.args(["-hide_banner","-v","error","-nostdin","-y","-protocol_whitelist","file","-threads","2","-f","mov","-i","/proc/self/fd/0","-map","0:v:0","-map","0:a:0?","-vf","scale='min(1280,iw)':'min(720,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2,setsar=1,fps=30,format=yuv420p","-c:v","libvpx","-deadline","realtime","-cpu-used","8","-threads","2","-b:v","1500k","-crf","12","-lag-in-frames","0","-auto-alt-ref","0","-c:a","libopus","-b:a","96k","-f","webm","/proc/self/fd/1"]);
        crate::processing::run_cancellable(
            command,
            std::time::Duration::from_secs(180),
            &cancelled,
        )?;
        if cancelled() {
            return Err("Preview cancelled".into());
        }
        dir.rename(&temp, "preview.webm", false)
            .map_err(|e| e.to_string())?;
        if cancelled() {
            dir.remove("preview.webm");
            return Err("Preview cancelled".into());
        }
        Ok(())
    })();
    dir.remove(&temp);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delete_removes_only_the_selected_regular_clip() {
        let root = std::env::temp_dir().join(crate::private_fs::temporary_name("delete").unwrap());
        fs::create_dir(&root).unwrap();
        fs::write(root.join("one.mp4"), b"one").unwrap();
        fs::write(root.join("two.mp4"), b"two").unwrap();
        std::os::unix::fs::symlink(root.join("two.mp4"), root.join("link.mp4")).unwrap();
        assert!(delete(&root, "link.mp4").is_err());
        assert!(delete(&root, "../two.mp4").is_err());
        delete(&root, "one.mp4").unwrap();
        assert!(!root.join("one.mp4").exists());
        assert_eq!(fs::read(root.join("two.mp4")).unwrap(), b"two");
        assert!(delete(&root, "one.mp4").is_err());
        fs::remove_dir_all(root).unwrap();
    }
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
        assert!(open_clip(&root, "link.mp4").is_err());
        assert_eq!(list(&root).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod media_tests {
    use super::*;
    use std::time::SystemTime;
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
                "testsrc2=size=1920x1080:rate=60",
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
        assert!((duration(&root, &output).unwrap() - 1.5).abs() < 0.15);
        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(list(&root).unwrap().len(), 2);
        let cache = root.join("cache");
        fs::create_dir(&cache).unwrap();
        compatible_preview(&root, "original.mp4", &cache, || false).unwrap();
        assert!(fs::metadata(cache.join("preview.webm")).unwrap().len() > 1000);
        let result = crate::system::command("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=codec_name,width,height",
                "-of",
                "csv=p=0",
            ])
            .arg(cache.join("preview.webm"))
            .output()
            .unwrap();
        let info = String::from_utf8_lossy(&result.stdout);
        assert!(info.contains("vp8,1280,720"), "{info}");
        fs::remove_file(cache.join("preview.webm")).unwrap();
        assert!(compatible_preview(&root, "original.mp4", &cache, || true)
            .unwrap_err()
            .contains("cancelled"));
        assert!(!cache.join("preview.webm").exists());
        assert_eq!(fs::read(&source).unwrap(), original);
        let fast = trim_with_mode(&root, "original.mp4", 1.0, 2.5, None, false).unwrap();
        assert!(duration(&root, &fast).unwrap() > 0.1);
        assert_eq!(fs::read(&source).unwrap(), original);
        fs::remove_file(root.join(fast)).unwrap();
        assert!(trim(&root, "original.mp4", 0.0, 100.0).is_err());
        assert!(trim_limited(&root, "original.mp4", 1.0, 2.5, Some(512)).is_err());
        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(list(&root).unwrap().len(), 2);
        assert!(!fs::read_dir(&root).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".trim-")));
        fs::remove_dir_all(root).unwrap();
    }
}
