#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod clips;
mod media;
mod memory;
mod system;
mod theme;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{Emitter, Manager, State};

#[derive(Default)]
struct Recorder(
    Mutex<Option<Child>>,
    std::sync::OnceLock<(String, bool, bool)>,
    AtomicBool,
);
#[derive(Default)]
struct ActiveSettings(Mutex<Option<Settings>>);
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    #[serde(default = "default_bitrate")]
    bitrate: u32,
    seconds: u32,
    fps: u32,
    monitor: String,
    system_audio: bool,
    microphone: bool,
}
fn default_bitrate() -> u32 {
    12
}
#[derive(Default)]
struct GameMode(AtomicBool);
#[derive(Default)]
struct MemoryState(Mutex<memory::Sample>);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    stopped_intentionally: bool,
    memory_target_bytes: u64,
    memory: memory::Sample,
    theme: theme::Theme,
    active_settings: Option<Settings>,
    ffmpeg_available: bool,
    running: bool,
    gpu: String,
    recorder_available: bool,
    wayland: bool,
    output_dir: String,
    log: String,
}
fn output_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let p = app
        .path()
        .video_dir()
        .or_else(|_| app.path().home_dir().map(|home| home.join("Videos")))
        .map_err(|e| e.to_string())?
        .join("Afterglow");
    fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    Ok(p)
}
fn runtime_dir() -> Result<PathBuf, String> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("XDG_RUNTIME_DIR is missing; run inside your desktop session".into())
}
fn log_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let p = app.path().app_log_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    Ok(p.join("capture.log"))
}
fn gpu() -> String {
    system::command("nvidia-smi")
        .args(["--query-gpu=name", "--format=csv,noheader"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default()
}
fn available() -> bool {
    system::command("gpu-screen-recorder")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}
fn is_running(slot: &mut Option<Child>) -> Result<bool, String> {
    if let Some(child) = slot.as_mut() {
        if child.try_wait().map_err(|e| e.to_string())?.is_none() {
            return Ok(true);
        }
        *slot = None;
        if let Ok(p) = runtime_dir() {
            let _ = fs::remove_file(p.join("afterglow-recorder.pid"));
        }
    }
    Ok(false)
}
#[tauri::command]
async fn status(app: tauri::AppHandle) -> Result<Status, String> {
    tauri::async_runtime::spawn_blocking(move || {
        status_inner(&app, app.state::<Recorder>(), app.state::<ActiveSettings>())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn status_inner(
    app: &tauri::AppHandle,
    state: State<Recorder>,
    active: State<ActiveSettings>,
) -> Result<Status, String> {
    let running = {
        let mut slot = state.0.lock().map_err(|e| e.to_string())?;
        is_running(&mut slot)?
    };
    let log = (|| -> std::io::Result<String> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = fs::File::open(log_path(app).map_err(std::io::Error::other)?)?;
        let size = file.metadata()?.len();
        file.seek(SeekFrom::Start(size.saturating_sub(16384)))?;
        let mut bytes = Vec::new();
        file.take(16384).read_to_end(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    })()
    .unwrap_or_default();
    let log = log
        .lines()
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let detected = state.1.get_or_init(|| {
        (
            gpu(),
            available(),
            system::command("ffmpeg")
                .arg("-version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
        )
    });
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let state_dir = std::env::var_os("XDG_STATE_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"));
    let theme = theme::read(
        &state_dir.join("caelestia/scheme.json"),
        &app.state::<theme::Cache>(),
    );
    Ok(Status {
        stopped_intentionally: state.2.load(Ordering::SeqCst),
        memory_target_bytes: memory::RAM_TARGET,
        memory: app
            .state::<MemoryState>()
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .clone(),
        theme,
        active_settings: active.0.lock().map_err(|e| e.to_string())?.clone(),
        ffmpeg_available: detected.2,
        running,
        gpu: detected.0.clone(),
        recorder_available: detected.1,
        wayland: std::env::var_os("WAYLAND_DISPLAY").is_some(),
        output_dir: output_dir(&app)?.display().to_string(),
        log,
    })
}
fn capture_args(s: &Settings, output: &str) -> Result<Vec<String>, String> {
    if ![30, 60, 90, 120].contains(&s.seconds) || ![30, 60].contains(&s.fps) {
        return Err("Unsupported replay length or frame rate".into());
    }
    if ![8, 12, 16, 20].contains(&s.bitrate) || u64::from(s.bitrate) * u64::from(s.seconds) > 1440 {
        return Err("Replay buffer budget is 180 MB: reduce bitrate or replay length".into());
    }
    if s.monitor.is_empty()
        || s.monitor.len() > 80
        || !s
            .monitor
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
    {
        return Err("Choose a valid monitor name, such as DP-1".into());
    }
    let mut args = vec![
        "-w".into(),
        s.monitor.clone(),
        "-f".into(),
        s.fps.to_string(),
        "-r".into(),
        s.seconds.to_string(),
        "-c".into(),
        "mp4".into(),
        "-k".into(),
        "h264".into(),
        "-bm".into(),
        "cbr".into(),
        "-q".into(),
        (s.bitrate * 1000).to_string(),
        "-o".into(),
        output.into(),
    ];
    if s.system_audio {
        args.extend(["-a".into(), "default_output".into()]);
    }
    if s.microphone {
        args.extend(["-a".into(), "default_input".into()]);
    }
    Ok(args)
}
#[tauri::command]
async fn start_replay(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        start_replay_inner(
            &app,
            app.state::<Recorder>(),
            app.state::<ActiveSettings>(),
            settings,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
fn start_replay_inner(
    app: &tauri::AppHandle,
    state: State<Recorder>,
    active: State<ActiveSettings>,
    settings: Settings,
) -> Result<(), String> {
    let mut slot = state.0.lock().map_err(|e| e.to_string())?;
    if is_running(&mut slot)? {
        return Err("Replay is already running".into());
    }
    if gpu().is_empty() {
        return Err("NVIDIA GPU not detected by nvidia-smi. Check your driver.".into());
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return Err("This prototype targets a Wayland desktop session".into());
    }
    let args = capture_args(&settings, &output_dir(&app)?.to_string_lossy())?;
    let pid_path = runtime_dir()?.join("afterglow-recorder.pid");
    // Refuse a second app instance while its recorder is alive.
    if let Ok(old) = fs::read_to_string(&pid_path) {
        if let Ok(pid) = old.trim().parse::<u32>() {
            if fs::read(format!("/proc/{pid}/comm"))
                .map(|b| b.starts_with(b"gpu-screen-reco"))
                .unwrap_or(false)
            {
                return Err("Another Afterglow recorder is running".into());
            }
        }
    }
    let log = fs::File::create(log_path(&app)?).map_err(|e| e.to_string())?;
    let mut command = system::command("gpu-screen-recorder");
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(log);
    let parent = std::process::id() as i32;
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGINT) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(std::io::Error::other(
                    "Afterglow exited during capture startup",
                ));
            }
            Ok(())
        });
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Cannot launch gpu-screen-recorder: {e}"))?;
    if let Err(e) = fs::write(pid_path, child.id().to_string()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e.to_string());
    }
    std::thread::sleep(std::time::Duration::from_millis(800));
    if let Some(exit) = child.try_wait().map_err(|e| e.to_string())? {
        let _ = fs::remove_file(runtime_dir()?.join("afterglow-recorder.pid"));
        return Err(format!(
            "Capture exited ({exit}). Check Capture setup → Capture log."
        ));
    }
    if let Some(stdout) = child.stdout.take() {
        let handle = app.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if line.trim().ends_with(".mp4") {
                    let _ = handle.emit("clip-saved", line);
                }
            }
        });
    }
    *active.0.lock().map_err(|e| e.to_string())? = Some(settings);
    state.2.store(false, Ordering::SeqCst);
    *slot = Some(child);
    Ok(())
}
fn signal(state: &Recorder, sig: i32) -> Result<(), String> {
    let mut slot = state.0.lock().map_err(|e| e.to_string())?;
    if !is_running(&mut slot)? {
        return Err("Replay is not running. Check the capture log.".into());
    }
    let pid = slot.as_ref().unwrap().id();
    if sig == libc::SIGINT {
        state.2.store(true, Ordering::SeqCst);
    }
    if unsafe { libc::kill(pid as i32, sig) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}
#[tauri::command]
fn save_replay(state: State<Recorder>) -> Result<(), String> {
    signal(&state, libc::SIGUSR1)
}
#[tauri::command]
async fn stop_replay(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Recorder>();
        signal(&state, libc::SIGINT)?;
        let mut slot = state.0.lock().map_err(|e| e.to_string())?;
        for _ in 0..50 {
            if !is_running(&mut slot)? {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Err("Recorder is taking longer to stop. Wait and check the capture log.".into())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn list_clips(app: tauri::AppHandle) -> Result<Vec<clips::Clip>, String> {
    clips::list(&output_dir(&app)?)
}
#[tauri::command]
fn rename_clip(app: tauri::AppHandle, name: String, new_name: String) -> Result<(), String> {
    clips::rename(&output_dir(&app)?, &name, &new_name)
}
#[tauri::command]
fn clip_media_url(
    app: tauri::AppHandle,
    media: State<media::MediaServer>,
    name: String,
) -> Result<String, String> {
    clips::clip_path(&output_dir(&app)?, &name)?;
    Ok(media.url(&name))
}
#[tauri::command]
async fn clip_duration(app: tauri::AppHandle, name: String) -> Result<f64, String> {
    let path = clips::clip_path(&output_dir(&app)?, &name)?;
    tauri::async_runtime::spawn_blocking(move || clips::duration(&path))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn trim_clip(
    app: tauri::AppHandle,
    name: String,
    start: f64,
    end: f64,
) -> Result<String, String> {
    let root = output_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || clips::trim(&root, &name, start, end))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn open_library(app: tauri::AppHandle) -> Result<(), String> {
    system::command("xdg-open")
        .arg(output_dir(&app)?)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
async fn capture_options() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let output = system::command("gpu-screen-recorder")
            .arg("--list-capture-options")
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|l| {
                let name = l.split('|').next()?.trim();
                if name == "window" || name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn save_from_cli() -> Result<(), String> {
    use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
    let pid: i32 = fs::read_to_string(runtime_dir()?.join("afterglow-recorder.pid"))
        .map_err(|_| "Afterglow replay is not running")?
        .trim()
        .parse()
        .map_err(|_| "Invalid recorder PID")?;
    if pid <= 0 {
        return Err("Invalid recorder PID".into());
    }
    let process = PathBuf::from(format!("/proc/{pid}"));
    if fs::metadata(&process).map_err(|e| e.to_string())?.uid() != unsafe { libc::getuid() } {
        return Err("Recorder belongs to another user".into());
    }
    let args = fs::read(process.join("cmdline")).map_err(|e| e.to_string())?;
    let executable =
        std::ffi::OsStr::from_bytes(args.split(|b| *b == 0).next().unwrap_or_default());
    if std::path::Path::new(executable).file_name()
        != Some(std::ffi::OsStr::new("gpu-screen-recorder"))
    {
        return Err("Saved PID is not GPU Screen Recorder".into());
    }
    if unsafe { libc::kill(pid, libc::SIGUSR1) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    println!("Replay save requested");
    Ok(())
}
fn reduce_web_cache(window: &tauri::WebviewWindow) {
    use webkit2gtk::{WebContextExt, WebViewExt};
    let _ = window.with_webview(|webview| {
        if let Some(context) = webview.inner().context() {
            context.set_cache_model(webkit2gtk::CacheModel::DocumentViewer);
        }
    });
}
fn show_main(app: &tauri::AppHandle) -> Result<(), String> {
    let window = if let Some(window) = app.get_webview_window("main") {
        window
    } else {
        let config = app
            .config()
            .app
            .windows
            .first()
            .ok_or("Window configuration is missing")?;
        tauri::WebviewWindowBuilder::from_config(app, config)
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?
    };
    app.state::<GameMode>().0.store(false, Ordering::SeqCst);
    reduce_web_cache(&window);
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}
#[tauri::command]
fn game_mode(app: tauri::AppHandle) -> Result<(), String> {
    use webkit2gtk::{WebContextExt, WebViewExt};
    let state = app.state::<Recorder>();
    {
        let mut slot = state.0.lock().map_err(|e| e.to_string())?;
        if !is_running(&mut slot)? {
            return Err("Start replay before entering game mode".into());
        }
    }
    let window = app
        .get_webview_window("main")
        .ok_or("Library window is already closed")?;
    app.state::<GameMode>().0.store(true, Ordering::SeqCst);
    let _ = window.with_webview(|webview| {
        let inner = webview.inner();
        inner.stop_loading();
        if let Some(context) = inner.context() {
            context.clear_cache();
        }
        inner.terminate_web_process();
    });
    if let Err(error) = window.destroy() {
        app.state::<GameMode>().0.store(false, Ordering::SeqCst);
        return Err(error.to_string());
    }
    Ok(())
}
struct TrayControls {
    save: tauri::menu::MenuItem<tauri::Wry>,
    stop: tauri::menu::MenuItem<tauri::Wry>,
}
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
    };
    let open = MenuItem::with_id(app, "open", "Open Afterglow", true, None::<&str>)?;
    let save = MenuItem::with_id(app, "save", "Save replay", false, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Stop replay", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Afterglow", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &save, &stop, &quit])?;
    let mut builder = TrayIconBuilder::with_id("afterglow")
        .menu(&menu)
        .tooltip("Afterglow replay");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                if let Err(e) = show_main(app) {
                    eprintln!("{e}");
                }
            }
            "save" => {
                if let Err(e) = signal(&app.state::<Recorder>(), libc::SIGUSR1) {
                    eprintln!("{e}");
                }
            }
            "stop" => {
                if let Err(e) = signal(&app.state::<Recorder>(), libc::SIGINT) {
                    eprintln!("{e}");
                }
            }
            "quit" => {
                app.state::<GameMode>().0.store(false, Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;
    app.manage(TrayControls { save, stop });
    Ok(())
}
fn start_memory_monitor(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut last_running = None;
        loop {
            let sample = memory::sample(std::process::id());
            let over_budget = sample.bytes >= memory::GAME_MODE_THRESHOLD;
            if let Ok(mut stored) = app.state::<MemoryState>().0.lock() {
                *stored = sample;
            }
            let running = app
                .state::<Recorder>()
                .0
                .lock()
                .map(|mut slot| is_running(&mut slot).unwrap_or(false))
                .unwrap_or(false);
            if last_running != Some(running) {
                if let Some(controls) = app.try_state::<TrayControls>() {
                    let _ = controls.save.set_enabled(running);
                    let _ = controls.stop.set_enabled(running);
                }
                last_running = Some(running);
            }
            if running && over_budget && app.get_webview_window("main").is_some() {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    let _ = game_mode(handle);
                });
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    });
}
fn main() {
    if std::env::args().skip(1).any(|arg| arg == "--save-replay") {
        if let Err(error) = save_from_cli() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .setup(|app| {
            let root = output_dir(app.handle()).map_err(std::io::Error::other)?;
            app.manage(media::MediaServer::start(root).map_err(std::io::Error::other)?);
            setup_tray(app.handle())?;
            if let Some(window) = app.get_webview_window("main") {
                reduce_web_cache(&window);
            }
            start_memory_monitor(app.handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = show_main(app);
        }))
        .manage(Recorder::default())
        .manage(ActiveSettings::default())
        .manage(GameMode::default())
        .manage(MemoryState::default())
        .manage(theme::Cache::default())
        .invoke_handler(tauri::generate_handler![
            status,
            start_replay,
            save_replay,
            stop_replay,
            list_clips,
            rename_clip,
            clip_duration,
            clip_media_url,
            trim_clip,
            open_library,
            capture_options,
            game_mode
        ])
        .build(tauri::generate_context!())
        .expect("Could not launch Afterglow")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
                if code.is_none() && app.state::<GameMode>().0.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<Recorder>();
                if let Ok(mut slot) = state.0.lock() {
                    if let Some(mut child) = slot.take() {
                        unsafe {
                            libc::kill(child.id() as i32, libc::SIGINT);
                        }
                        for _ in 0..30 {
                            if child.try_wait().ok().flatten().is_some() {
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        if child.try_wait().ok().flatten().is_none() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                    }
                }
                if let Ok(p) = runtime_dir() {
                    let _ = fs::remove_file(p.join("afterglow-recorder.pid"));
                }
            }
        });
}
#[cfg(test)]
mod tests {
    use super::*;
    fn settings() -> Settings {
        Settings {
            bitrate: 12,
            seconds: 60,
            fps: 60,
            monitor: "DP-1".into(),
            system_audio: true,
            microphone: false,
        }
    }
    #[test]
    fn validates_settings() {
        let mut s = settings();
        s.seconds = 0;
        assert!(capture_args(&s, "/tmp/clips").is_err());
        s = settings();
        s.monitor = "DP-1; touch /tmp/oops".into();
        assert!(capture_args(&s, "/tmp/clips").is_err());
    }
    #[test]
    fn uses_h264_and_audio() {
        let args = capture_args(&settings(), "/tmp/my clips").unwrap();
        assert!(args.windows(2).any(|x| x == ["-k", "h264"]));
        assert!(args.windows(2).any(|x| x == ["-a", "default_output"]));
        assert_eq!(args.iter().filter(|x| *x == "/tmp/my clips").count(), 1);
    }
    #[test]
    fn caps_replay_memory_and_migrates_older_preferences() {
        let mut s = settings();
        s.bitrate = 20;
        s.seconds = 120;
        assert!(capture_args(&s, "/tmp/clips").is_err());
        s.bitrate = 12;
        assert!(capture_args(&s, "/tmp/clips").is_ok());
        let old: Settings = serde_json::from_str(
            r#"{"seconds":60,"fps":60,"monitor":"DP-1","systemAudio":true,"microphone":false}"#,
        )
        .unwrap();
        assert_eq!(old.bitrate, 12);
    }
}
