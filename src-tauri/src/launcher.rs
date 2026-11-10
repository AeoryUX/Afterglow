use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub fn doctor() {
    println!("Afterglow {}", env!("CARGO_PKG_VERSION"));
    if let Ok(path) = std::env::current_exe() {
        println!("Executable: {}", path.display());
    }
    println!(
        "Wayland session: {}",
        std::env::var_os("WAYLAND_DISPLAY").is_some()
    );
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|p| p.join(".local/share")));
    describe(
        Path::new("/usr/share/applications/dev.afterglow.desktop.desktop"),
        "System launcher",
    );
    if let Some(data) = data {
        describe(
            &data.join("applications/dev.afterglow.desktop.desktop"),
            "User launcher override",
        );
    }
    if let Ok(version) = crate::control::send("version") {
        println!("Running backend: Afterglow {version}");
    } else {
        println!("Running backend: not running or an older control protocol");
    }
    println!("A user launcher override takes precedence over the system launcher.");
    for tool in ["nvidia-smi", "gpu-screen-recorder", "ffmpeg"] {
        let args = if tool == "nvidia-smi" {
            vec!["--query-gpu=name", "--format=csv,noheader"]
        } else if tool == "ffmpeg" {
            vec!["-version"]
        } else {
            vec!["--version"]
        };
        match crate::system::command(tool).args(args).output() {
            Ok(output) => println!(
                "{tool}: {} {}",
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" | "),
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
            Err(error) => println!("{tool}: {error}"),
        };
    }
    for codec in ["avdec_h264", "h264parse", "vp8dec", "matroskademux"] {
        match crate::system::command("gst-inspect-1.0")
            .args(["--exists", codec])
            .output()
        {
            Ok(out) => println!(
                "Playback {codec}: {}",
                if out.status.success() {
                    "available"
                } else {
                    "missing or could not load"
                }
            ),
            Err(_) => {
                println!("Playback plugins: gst-inspect-1.0 is unavailable; plugin checks skipped");
                break;
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let data = std::env::var_os("XDG_DATA_HOME")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(home).join(".local/share"));
        for name in ["capture.log", "events.log"] {
            let path = data.join("dev.afterglow.desktop/logs").join(name);
            if let Ok(mut file) = crate::private_fs::Directory::open(path.parent().unwrap())
                .and_then(|d| d.read(name))
            {
                use std::io::{Read, Seek, SeekFrom};
                let size = file.metadata().map(|m| m.len()).unwrap_or(0);
                let _ = file.seek(SeekFrom::Start(size.saturating_sub(65536)));
                let mut text = String::new();
                let _ = file.take(65536).read_to_string(&mut text);
                let lines = text
                    .lines()
                    .filter(|v| !v.contains("gsr info: update fps:"))
                    .collect::<Vec<_>>();
                println!(
                    "Recent {name}:\n{}",
                    lines
                        .into_iter()
                        .rev()
                        .take(20)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
        }
    }
}
fn describe(path: &Path, label: &str) {
    match fs::metadata(path) {
        Ok(m) => println!(
            "{label}: {} (mode {:04o}, readable by other users: {})",
            path.display(),
            m.permissions().mode() & 0o777,
            m.permissions().mode() & 0o004 != 0
        ),
        Err(_) => println!("{label}: absent ({})", path.display()),
    }
}
