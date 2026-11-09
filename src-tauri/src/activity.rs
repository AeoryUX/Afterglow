use std::{
    io::{Read, Seek, SeekFrom, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::Manager;
#[derive(Default)]
pub struct Activity {
    pub editor: AtomicBool,
    pub starting: AtomicBool,
    pub capture_error: Mutex<Option<String>>,
}
pub fn editor_busy(app: &tauri::AppHandle) -> bool {
    app.state::<Activity>().editor.load(Ordering::SeqCst) || crate::clips::is_processing()
}
pub fn event(app: &tauri::AppHandle, message: &str) {
    let result = (|| -> std::io::Result<()> {
        let path = crate::log_path(app).map_err(std::io::Error::other)?;
        let dir = crate::private_fs::Directory::private(path.parent().unwrap())?;
        let mut file = dir.append_log("events.log")?;
        if file.metadata()?.len() > 262144 {
            file.set_len(0)?;
        }
        writeln!(file, "Afterglow: {}", message.replace(['\n', '\r'], " "))
    })();
    if let Err(error) = result {
        eprintln!("Cannot write app log: {error}");
    }
}
pub fn capture_error(app: &tauri::AppHandle, message: String) {
    if let Ok(mut error) = app.state::<Activity>().capture_error.lock() {
        *error = Some(message.clone());
    }
    event(app, &message);
    let _ = app.emit("capture-failed", message);
}
use tauri::Emitter;
pub fn log_tail(app: &tauri::AppHandle) -> String {
    let Ok(path) = crate::log_path(app) else {
        return String::new();
    };
    let Ok(dir) = crate::private_fs::Directory::open(path.parent().unwrap()) else {
        return String::new();
    };
    let mut sections = Vec::new();
    for name in ["capture.log", "events.log"] {
        let text = (|| -> std::io::Result<String> {
            let mut file = dir.read(name)?;
            let len = file.metadata()?.len();
            file.seek(SeekFrom::Start(len.saturating_sub(32768)))?;
            let mut data = Vec::new();
            file.take(32768).read_to_end(&mut data)?;
            Ok(String::from_utf8_lossy(&data).into_owned())
        })();
        if let Ok(text) = text {
            let lines = text
                .lines()
                .filter(|line| !line.contains("gsr info: update fps:"))
                .collect::<Vec<_>>();
            let tail = lines
                .into_iter()
                .rev()
                .take(25)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            if !tail.is_empty() {
                sections.push(format!("{name}:\n{tail}"));
            }
        }
    }
    sections.join("\n\n")
}
