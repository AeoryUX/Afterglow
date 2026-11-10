#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod activity;
mod autostart;
mod clips;
mod control;
mod folders;
mod hotkeys;
mod launcher;
mod media;
mod memory;
mod preferences;
mod preview;
mod private_fs;
mod processing;
mod selection;
mod sound;
mod storage;
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
    Mutex<Option<(std::time::Instant, (String, bool, bool))>>,
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
    #[serde(default)]
    session: String,
    #[serde(default = "default_true")]
    show_cursor: bool,
    #[serde(default)]
    resolution: String,
}
fn default_true() -> bool {
    true
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            bitrate: 12,
            seconds: 60,
            fps: 60,
            monitor: "screen".into(),
            system_audio: true,
            microphone: false,
            session: "auto".into(),
            show_cursor: true,
            resolution: "native".into(),
        }
    }
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
    tray_initialized: bool,
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
    session: String,
    storage: storage::Storage,
    preferences: preferences::Preferences,
    shortcut_status: String,
    background: bool,
    starting: bool,
    capture_error: Option<String>,
}
fn output_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let p = app
        .path()
        .video_dir()
        .or_else(|_| app.path().home_dir().map(|home| home.join("Videos")))
        .map_err(|e| e.to_string())?
        .join("Afterglow");
    fs::create_dir_all(p.parent().ok_or("Missing video directory")?).map_err(|e| e.to_string())?;
    private_fs::Directory::private(&p).map_err(|e| e.to_string())?;
    Ok(p)
}
fn runtime_dir() -> Result<PathBuf, String> {
    use std::os::unix::fs::MetadataExt;
    let path = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("XDG_RUNTIME_DIR is missing; run inside your desktop session")?;
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !path.is_absolute()
        || !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(
            "XDG_RUNTIME_DIR must be a private directory owned by your desktop user".into(),
        );
    }
    Ok(path)
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
async fn status(app: tauri::AppHandle, force: Option<bool>) -> Result<Status, String> {
    tauri::async_runtime::spawn_blocking(move || {
        status_inner(
            &app,
            app.state::<Recorder>(),
            app.state::<ActiveSettings>(),
            force.unwrap_or(false),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
fn status_inner(
    app: &tauri::AppHandle,
    state: State<Recorder>,
    active: State<ActiveSettings>,
    force: bool,
) -> Result<Status, String> {
    let starting = app
        .state::<activity::Activity>()
        .starting
        .load(Ordering::SeqCst);
    let running = if starting {
        false
    } else {
        let mut slot = state.0.lock().map_err(|e| e.to_string())?;
        is_running(&mut slot)?
    };
    let log = activity::log_tail(app);
    let detected = {
        let mut cached = state.1.lock().map_err(|e| e.to_string())?;
        if force
            || cached
                .as_ref()
                .map(|(time, _)| time.elapsed() > std::time::Duration::from_secs(30))
                .unwrap_or(true)
        {
            let info = (
                gpu(),
                available(),
                system::command("ffmpeg")
                    .arg("-version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false),
            );
            *cached = Some((std::time::Instant::now(), info));
        }
        cached.as_ref().unwrap().1.clone()
    };
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let state_dir = std::env::var_os("XDG_STATE_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"));
    let theme = theme::read(
        &state_dir.join("caelestia/scheme.json"),
        &app.state::<theme::Cache>(),
    );
    let preferences = app
        .state::<preferences::PreferencesState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let settings = active
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .or_else(|| preferences.capture.clone())
        .unwrap_or_default();
    let storage = storage::snapshot(&output_dir(app)?, preferences.storage_limit_mb, &settings)?;
    Ok(Status {
        starting,
        capture_error: app
            .state::<activity::Activity>()
            .capture_error
            .lock()
            .map_err(|e| e.to_string())?
            .clone(),
        background: app.state::<GameMode>().0.load(Ordering::SeqCst),
        session: detected_session(),
        storage,
        preferences,
        shortcut_status: app
            .state::<hotkeys::HotkeyState>()
            .message
            .lock()
            .map_err(|e| e.to_string())?
            .clone(),
        tray_initialized: app.try_state::<TrayControls>().is_some(),
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
    if !["", "auto", "wayland", "x11"].contains(&s.session.as_str())
        || !["", "native", "1920x1080", "1280x720"].contains(&s.resolution.as_str())
    {
        return Err("Choose a supported session and resolution".into());
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
        "-ac".into(),
        "aac".into(),
        "-v".into(),
        "no".into(),
        "-bm".into(),
        "cbr".into(),
        "-q".into(),
        (s.bitrate * 1000).to_string(),
        "-o".into(),
        output.into(),
    ];
    args.extend([
        "-cursor".into(),
        if s.show_cursor { "yes" } else { "no" }.into(),
    ]);
    if !s.resolution.is_empty() && s.resolution != "native" {
        args.extend(["-s".into(), s.resolution.clone()]);
    }
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
type CaptureRequest = (Settings, std::sync::mpsc::Sender<Result<(), String>>);
#[derive(Default)]
struct CaptureWorker(Mutex<Option<std::sync::mpsc::SyncSender<CaptureRequest>>>);
fn start_capture_worker(app: &tauri::AppHandle) -> Result<(), String> {
    let (sender, receiver) = std::sync::mpsc::sync_channel::<CaptureRequest>(4);
    let handle = app.clone();
    std::thread::Builder::new()
        .name("afterglow-capture".into())
        .spawn(move || {
            while let Ok((settings, reply)) = receiver.recv() {
                let result = spawn_capture(
                    &handle,
                    handle.state::<Recorder>(),
                    handle.state::<ActiveSettings>(),
                    settings,
                );
                let _ = reply.send(result);
            }
        })
        .map_err(|e| e.to_string())?;
    *app.state::<CaptureWorker>()
        .0
        .lock()
        .map_err(|e| e.to_string())? = Some(sender);
    Ok(())
}
fn start_replay_inner(
    app: &tauri::AppHandle,
    _state: State<Recorder>,
    _active: State<ActiveSettings>,
    settings: Settings,
) -> Result<(), String> {
    // PR_SET_PDEATHSIG follows the spawning Linux thread. Keep that thread alive
    // across closed-window sessions instead of using a retiring async-pool worker.
    let activity = app.state::<activity::Activity>();
    if activity.starting.swap(true, Ordering::SeqCst) {
        return Err("Replay is already starting".into());
    }
    if let Ok(mut error) = activity.capture_error.lock() {
        *error = None;
    }
    activity::event(app, "Starting replay");
    let result = (|| {
        let sender = app
            .state::<CaptureWorker>()
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or("Capture service is not ready")?;
        let (reply, result) = std::sync::mpsc::channel();
        sender
            .try_send((settings, reply))
            .map_err(|_| "Capture service is busy; try again in a moment")?;
        result.recv().map_err(|_| "Capture service stopped")?
    })();
    activity.starting.store(false, Ordering::SeqCst);
    match &result {
        Ok(()) => activity::event(
            app,
            "Replay buffer started. Use Save clip or your shortcut to keep a clip.",
        ),
        Err(error) => activity::capture_error(app, error.clone()),
    };
    result
}
fn spawn_capture(
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
    let mode = resolve_session(&settings.session, &detected_session())?;
    let args = capture_args(&settings, &output_dir(&app)?.to_string_lossy())?;
    let pid_path = runtime_dir()?.join("afterglow-recorder.pid");
    // Refuse a second app instance while its recorder is alive.
    if let Ok(old) = fs::read_to_string(&pid_path) {
        if let Ok(pid) = old.trim().parse::<u32>() {
            if fs::read(format!("/proc/{pid}/comm"))
                .map(|b| b.starts_with(b"gpu-screen-reco"))
                .unwrap_or(false)
                && fs::read_to_string(format!("/proc/{pid}/stat"))
                    .ok()
                    .and_then(|v| {
                        v.rsplit_once(") ")
                            .map(|(_, tail)| !tail.starts_with(['Z', 'X']))
                    })
                    .unwrap_or(false)
            {
                return Err("Another Afterglow recorder is running".into());
            }
        }
    }
    let path = log_path(&app)?;
    let log = private_fs::Directory::private(path.parent().ok_or("Missing log directory")?)
        .and_then(|dir| dir.private_log("capture.log"))
        .map_err(|e| e.to_string())?;
    let mut command = system::command("gpu-screen-recorder");
    if mode == "x11" {
        command.env_remove("WAYLAND_DISPLAY");
    }
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
    if let Err(e) = private_fs::Directory::open(pid_path.parent().unwrap()).and_then(|dir| {
        dir.atomic_write("afterglow-recorder.pid", child.id().to_string().as_bytes())
    }) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e.to_string());
    }
    std::thread::sleep(std::time::Duration::from_millis(800));
    if let Some(exit) = child.try_wait().map_err(|e| e.to_string())? {
        let _ = fs::remove_file(runtime_dir()?.join("afterglow-recorder.pid"));
        return Err(format!(
            "Capture exited ({exit}). {}",
            activity::log_tail(app)
        ));
    }
    if let Some(stdout) = child.stdout.take() {
        let handle = app.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if line.trim().ends_with(".mp4") {
                    handle
                        .state::<SavePending>()
                        .0
                        .store(false, Ordering::SeqCst);
                    let _ = handle.emit("clip-saved", line);
                    if handle
                        .state::<preferences::PreferencesState>()
                        .0
                        .lock()
                        .map(|p| p.clip_sound)
                        .unwrap_or(false)
                    {
                        sound::play(handle.clone());
                    }
                }
            }
        });
    }
    *active.0.lock().map_err(|e| e.to_string())? = Some(settings.clone());
    if let Ok(mut p) = app.state::<preferences::PreferencesState>().0.lock() {
        p.capture = Some(settings);
        let _ = preferences::store(&preferences_path(app)?, &p);
    }
    state.2.store(false, Ordering::SeqCst);
    app.state::<SavePending>().0.store(false, Ordering::SeqCst);
    *slot = Some(child);
    Ok(())
}
#[derive(Default)]
struct SavePending(AtomicBool, Mutex<Option<std::time::Instant>>);
fn preferences_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|p| p.join("preferences.json"))
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn set_editor_active(app: tauri::AppHandle, active: bool) {
    if !active {
        app.state::<preview::Preview>().cancel();
    }
    app.state::<activity::Activity>()
        .editor
        .store(active, Ordering::SeqCst);
}
#[tauri::command]
fn get_preferences(app: tauri::AppHandle) -> Result<preferences::Preferences, String> {
    app.state::<preferences::PreferencesState>()
        .0
        .lock()
        .map(|p| p.clone())
        .map_err(|e| e.to_string())
}
fn sync_autostart(app: &tauri::AppHandle, p: &preferences::Preferences) -> Result<(), String> {
    let config = app.path().config_dir().map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let managed = system::command("systemctl")
        .args(["--user", "is-active", "graphical-session.target"])
        .output()
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "active");
    let hypr = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() && !managed;
    autostart::apply(
        &config,
        &exe,
        p.start_with_system,
        hypr,
        hypr && hotkeys::uses_lua(),
    )?;
    if managed {
        let result = system::command("systemctl")
            .args(["--user", "daemon-reload"])
            .output()
            .map_err(|e| e.to_string())?;
        if !result.status.success() {
            return Err(format!(
                "Could not refresh desktop login startup: {}",
                String::from_utf8_lossy(&result.stderr)
            ));
        }
    }
    Ok(())
}
#[tauri::command]
fn save_preferences(
    app: tauri::AppHandle,
    mut prefs: preferences::Preferences,
) -> Result<preferences::Preferences, String> {
    let state = app.state::<preferences::PreferencesState>();
    let mut current = state.0.lock().map_err(|e| e.to_string())?;
    // Shortcuts are changed through their explicit Apply/Remove action.
    prefs.shortcuts = current.shortcuts.clone();
    let login_changed = prefs.start_with_system != current.start_with_system
        || prefs.show_window_on_login != current.show_window_on_login;
    if login_changed {
        if let Err(e) = sync_autostart(&app, &prefs) {
            let _ = sync_autostart(&app, &current);
            return Err(e);
        }
    }
    if let Err(e) = preferences::store(&preferences_path(&app)?, &prefs) {
        if login_changed {
            let _ = sync_autostart(&app, &current);
        }
        return Err(e);
    }
    *current = prefs.clone();
    Ok(prefs)
}
#[tauri::command]
async fn apply_shortcuts(
    app: tauri::AppHandle,
    shortcuts: preferences::Shortcuts,
) -> Result<preferences::Preferences, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<preferences::PreferencesState>();
        let mut current = state.0.lock().map_err(|e| e.to_string())?;
        hotkeys::apply(&app.state::<hotkeys::HotkeyState>(), &shortcuts)?;
        let mut next = current.clone();
        next.shortcuts = shortcuts;
        preferences::store(&preferences_path(&app)?, &next)?;
        *current = next.clone();
        *app.state::<hotkeys::HotkeyState>()
            .message
            .lock()
            .map_err(|e| e.to_string())? = if next.shortcuts.enabled {
            "Shortcuts are active in Hyprland, including while Afterglow is in the tray."
        } else {
            "Afterglow shortcuts removed."
        }
        .into();
        Ok(next)
    })
    .await
    .map_err(|e| e.to_string())?
}
fn refresh_hotkeys(app: &tauri::AppHandle) {
    let state = app.state::<preferences::PreferencesState>();
    let prefs = state.0.lock().map(|p| p.clone());
    if let Ok(prefs) = prefs {
        if prefs.shortcuts.enabled {
            let result = hotkeys::apply(&app.state::<hotkeys::HotkeyState>(), &prefs.shortcuts);
            if let Ok(mut message) = app.state::<hotkeys::HotkeyState>().message.lock() {
                *message = match result {
                    Ok(()) => "Shortcuts active in Hyprland".into(),
                    Err(e) => e,
                };
            }
        }
    }
}
fn request_save(app: &tauri::AppHandle) -> Result<(), String> {
    let p = app
        .state::<preferences::PreferencesState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let settings = app
        .state::<ActiveSettings>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .or(p.capture)
        .unwrap_or_default();
    storage::guard(&output_dir(app)?, p.storage_limit_mb, &settings)?;
    let pending = app.state::<SavePending>();
    if pending.0.swap(true, Ordering::SeqCst) {
        return Err("A clip is already being saved. Wait for it to finish.".into());
    }
    if let Err(e) = signal(&app.state::<Recorder>(), libc::SIGUSR1) {
        pending.0.store(false, Ordering::SeqCst);
        return Err(e);
    }
    *pending.1.lock().map_err(|e| e.to_string())? = Some(std::time::Instant::now());
    Ok(())
}
fn toggle_replay(app: &tauri::AppHandle) -> Result<(), String> {
    let running = {
        let recorder = app.state::<Recorder>();
        let mut slot = recorder.0.lock().map_err(|e| e.to_string())?;
        is_running(&mut slot)?
    };
    if running {
        signal(&app.state::<Recorder>(), libc::SIGINT)
    } else {
        start_saved_replay(app)
    }
}
fn start_saved_replay(app: &tauri::AppHandle) -> Result<(), String> {
    let settings = app
        .state::<preferences::PreferencesState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .capture
        .clone()
        .unwrap_or_default();
    start_replay_inner(
        app,
        app.state::<Recorder>(),
        app.state::<ActiveSettings>(),
        settings,
    )
}

fn detected_session() -> String {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        "wayland"
    } else if std::env::var_os("DISPLAY").is_some() {
        "x11"
    } else {
        "none"
    }
    .into()
}
fn repair_saved_session(settings: &mut Settings, actual: &str) -> bool {
    if ["wayland", "x11"].contains(&actual)
        && ["wayland", "x11"].contains(&settings.session.as_str())
        && settings.session != actual
    {
        settings.session = "auto".into();
        true
    } else {
        false
    }
}
fn resolve_session(preferred: &str, actual: &str) -> Result<String, String> {
    let selected = if preferred.is_empty() || preferred == "auto" {
        actual
    } else {
        preferred
    };
    if !["wayland", "x11"].contains(&selected) {
        return Err("Open Afterglow inside a Wayland or X11 desktop session".into());
    }
    if selected != actual {
        return Err(format!("You are currently in a {actual} session. Choose Auto or {actual}; changing this setting does not switch your desktop session."));
    }
    Ok(selected.into())
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
fn save_replay(app: tauri::AppHandle) -> Result<(), String> {
    request_save(&app)
}
#[tauri::command]
async fn stop_replay(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Recorder>();
        signal(&state, libc::SIGINT)?;
        for _ in 0..50 {
            {
                let mut slot = state.0.lock().map_err(|e| e.to_string())?;
                if !is_running(&mut slot)? {
                    return Ok(());
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Err("Recorder is taking longer to stop. Wait and check the capture log.".into())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn get_folders(app: tauri::AppHandle) -> Result<folders::Data, String> {
    app.state::<folders::Folders>().snapshot()
}
#[tauri::command]
fn create_folder(app: tauri::AppHandle, name: String) -> Result<folders::Data, String> {
    app.state::<folders::Folders>().create(&name)
}
#[tauri::command]
fn rename_folder(app: tauri::AppHandle, id: String, name: String) -> Result<folders::Data, String> {
    app.state::<folders::Folders>().rename(&id, &name)
}
#[tauri::command]
fn remove_folder(app: tauri::AppHandle, id: String) -> Result<folders::Data, String> {
    app.state::<folders::Folders>().remove(&id)
}
#[tauri::command]
fn assign_clip_folder(
    app: tauri::AppHandle,
    name: String,
    folder_id: Option<String>,
) -> Result<folders::Data, String> {
    clips::open_clip(&output_dir(&app)?, &name)?;
    app.state::<folders::Folders>()
        .assign(&name, folder_id.as_deref())
}
#[tauri::command]
fn assign_clips_folder(
    app: tauri::AppHandle,
    names: Vec<String>,
    folder_id: Option<String>,
) -> Result<folders::Data, String> {
    selection::validate(&names)?;
    let root = output_dir(&app)?;
    for name in &names {
        clips::open_clip(&root, name)?;
    }
    app.state::<folders::Folders>()
        .assign_many(&names, folder_id.as_deref())
}
#[tauri::command]
fn delete_clips(app: tauri::AppHandle, names: Vec<String>) -> Result<selection::Deleted, String> {
    let result = selection::delete(&output_dir(&app)?, &names)?;
    if !result.deleted.is_empty() {
        if let Err(error) = app.state::<folders::Folders>().forget_many(&result.deleted) {
            activity::event(
                &app,
                &format!("Selected clips deleted; folder cleanup deferred: {error}"),
            );
        }
    }
    activity::event(
        &app,
        &format!(
            "Selected clips: {} deleted, {} failed",
            result.deleted.len(),
            result.failed.len()
        ),
    );
    Ok(result)
}
#[tauri::command]
async fn preview_bytes(app: tauri::AppHandle) -> Result<tauri::ipc::Response, String> {
    let cache = app.state::<preview::Preview>().cache.clone();
    tauri::async_runtime::spawn_blocking(move || {
        preview::read_bytes(&cache).map(tauri::ipc::Response::new)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn list_clips(app: tauri::AppHandle) -> Result<Vec<clips::Clip>, String> {
    clips::list(&output_dir(&app)?)
}
#[tauri::command]
fn delete_clip(app: tauri::AppHandle, name: String) -> Result<(), String> {
    clips::delete(&output_dir(&app)?, &name)?;
    if let Err(e) = app.state::<folders::Folders>().assign(&name, None) {
        activity::event(&app, &format!("Clip deleted; folder cleanup deferred: {e}"));
    }
    activity::event(&app, &format!("Deleted clip: {name}"));
    Ok(())
}
#[tauri::command]
fn rename_clip(app: tauri::AppHandle, name: String, new_name: String) -> Result<(), String> {
    app.state::<folders::Folders>()
        .renamed_clip(&output_dir(&app)?, &name, &new_name)
}
#[tauri::command]
fn clip_media_url(
    app: tauri::AppHandle,
    media: State<media::MediaServer>,
    name: String,
) -> Result<String, String> {
    clips::open_clip(&output_dir(&app)?, &name)?;
    Ok(media.url(&name))
}
#[tauri::command]
async fn clip_duration(app: tauri::AppHandle, name: String) -> Result<f64, String> {
    let root = output_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || clips::duration(&root, &name))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn trim_clip(
    app: tauri::AppHandle,
    name: String,
    start: f64,
    end: f64,
    precise: Option<bool>,
) -> Result<String, String> {
    let root = output_dir(&app)?;
    let prefs = app
        .state::<preferences::PreferencesState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let room = storage::export_room(&root, prefs.storage_limit_mb)?;
    tauri::async_runtime::spawn_blocking(move || {
        let result =
            clips::trim_with_mode(&root, &name, start, end, room, precise.unwrap_or(false));
        match &result {
            Ok(new) => {
                if let Err(e) = app.state::<folders::Folders>().inherit(&name, new) {
                    activity::event(
                        &app,
                        &format!("Trim exported; folder assignment failed: {e}"),
                    );
                }
                activity::event(&app, "Trimmed copy exported");
            }
            Err(error) => activity::event(&app, &format!("Trim export failed: {error}")),
        };
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum WindowAction {
    Minimize,
    Maximize,
    Fullscreen,
    Windowed,
    Close,
    Drag,
    State,
}
#[tauri::command]
fn window_action(app: tauri::AppHandle, action: WindowAction) -> Result<bool, String> {
    let window = app
        .get_webview_window("main")
        .ok_or("App window is not open")?;
    match action {
        WindowAction::Minimize => {
            if app.try_state::<TrayControls>().is_some() {
                background_library(&app)?;
            } else {
                window.minimize().map_err(|e| e.to_string())?;
            }
        }
        WindowAction::Fullscreen => {
            window
                .set_fullscreen(!window.is_fullscreen().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
        WindowAction::Windowed => {
            window.set_fullscreen(false).map_err(|e| e.to_string())?;
        }
        WindowAction::Maximize => {
            if window.is_maximized().map_err(|e| e.to_string())? {
                window.unmaximize()
            } else {
                window.maximize()
            }
            .map_err(|e| e.to_string())?;
        }
        WindowAction::Close => {
            window.close().map_err(|e| e.to_string())?;
        }
        WindowAction::Drag => {
            window.start_dragging().map_err(|e| e.to_string())?;
        }
        WindowAction::State => (),
    }
    Ok(window.is_fullscreen().unwrap_or(false))
}
#[tauri::command]
fn report_playback_error(
    app: tauri::AppHandle,
    name: String,
    detail: String,
) -> Result<(), String> {
    clips::validate_name(&name)?;
    activity::event(
        &app,
        &format!(
            "Playback {}: {}; HTTP {}",
            name,
            detail.chars().take(1200).collect::<String>(),
            app.state::<media::MediaServer>().diagnostics()
        ),
    );
    Ok(())
}
#[tauri::command]
async fn prepare_preview(app: tauri::AppHandle, name: String) -> Result<String, String> {
    clips::validate_name(&name)?;
    let root = output_dir(&app)?;
    let preview = app.state::<preview::Preview>();
    let generation = preview.generation.clone();
    let cache = preview.cache.clone();
    let ticket = generation.fetch_add(1, Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn_blocking(move || {
        let result = clips::compatible_preview(&root, &name, &cache, || {
            generation.load(Ordering::SeqCst) != ticket
        });
        if let Err(e) = &result {
            if e != "Preview cancelled" {
                activity::event(&app, &format!("Compatible preview: {e}"));
            }
        }
        result?;
        activity::event(&app, "Compatible preview ready");
        Ok(app.state::<media::MediaServer>().preview_url())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn open_clip_external(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let root = output_dir(&app)?;
    clips::open_clip(&root, &name)?;
    tauri::async_runtime::spawn_blocking(move || {
        let result = system::command("xdg-open")
            .arg(root.join(name))
            .stdin(Stdio::null())
            .output()
            .map_err(|e| e.to_string())?;
        if result.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Could not open clip externally: {}",
                String::from_utf8_lossy(&result.stderr)
            ))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn open_library(app: tauri::AppHandle) -> Result<(), String> {
    let path = output_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let output = system::command("xdg-open")
            .arg(path)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Could not open the clip folder: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    })
    .await
    .map_err(|e| e.to_string())?
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
    use std::{
        io::Read,
        os::fd::{AsRawFd, FromRawFd, OwnedFd},
    };
    let runtime = runtime_dir()?;
    let mut text = String::new();
    private_fs::Directory::open(&runtime)
        .and_then(|dir| dir.read("afterglow-recorder.pid"))
        .and_then(|file| file.take(32).read_to_string(&mut text))
        .map_err(|_| "Afterglow replay is not running")?;
    let pid: i32 = text.trim().parse().map_err(|_| "Invalid recorder PID")?;
    if pid <= 0 {
        return Err("Invalid recorder PID".into());
    }
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32;
    if fd < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let pidfd = unsafe { OwnedFd::from_raw_fd(fd) };
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
    if unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            pidfd.as_raw_fd(),
            libc::SIGUSR1,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    println!("Replay save requested");
    Ok(())
}
fn reduce_web_cache(window: &tauri::WebviewWindow) {
    use webkit2gtk::{SettingsExt, WebContextExt, WebViewExt};
    let _ = window.with_webview(|webview| {
        if let Some(settings) = webview.inner().settings() {
            settings.set_enable_media(true);
            settings.set_enable_fullscreen(true);
            settings
                .set_hardware_acceleration_policy(webkit2gtk::HardwareAccelerationPolicy::Never);
        }
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
    if activity::editor_busy(&app) {
        return Err("Finish or close the clip editor before entering game mode".into());
    }
    let state = app.state::<Recorder>();
    {
        let mut slot = state.0.lock().map_err(|e| e.to_string())?;
        if !is_running(&mut slot)? {
            return Err("Start replay before entering game mode".into());
        }
    }
    background_library(&app)
}
fn background_library(app: &tauri::AppHandle) -> Result<(), String> {
    app.state::<preview::Preview>().cancel();
    app.state::<activity::Activity>()
        .editor
        .store(false, Ordering::SeqCst);
    use webkit2gtk::{WebContextExt, WebViewExt};
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
    start: tauri::menu::MenuItem<tauri::Wry>,
    save: tauri::menu::MenuItem<tauri::Wry>,
    stop: tauri::menu::MenuItem<tauri::Wry>,
}
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
    };
    let open = MenuItem::with_id(app, "open", "Open Afterglow", true, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "Start replay", true, None::<&str>)?;
    let save = MenuItem::with_id(app, "save", "Save replay", false, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Stop replay", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Afterglow", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &start, &save, &stop, &quit])?;
    let builder = TrayIconBuilder::with_id("afterglow")
        .menu(&menu)
        .tooltip("Afterglow · Replay idle")
        .icon(tauri::image::Image::new(
            include_bytes!("../icons/tray.rgba"),
            64,
            64,
        ));
    builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                if let Err(e) = show_main(app) {
                    eprintln!("{e}");
                }
            }
            "start" => {
                let handle = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let _ = start_saved_replay(&handle);
                });
            }
            "save" => {
                if let Err(e) = request_save(app) {
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
    app.manage(TrayControls { start, save, stop });
    Ok(())
}
fn start_memory_monitor(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut last_running = None;
        let mut checks = 0;
        loop {
            checks += 1;
            if checks % 5 == 0 {
                refresh_hotkeys(&app);
            }
            if let Ok(mut time) = app.state::<SavePending>().1.lock() {
                if time
                    .as_ref()
                    .is_some_and(|t| t.elapsed() > std::time::Duration::from_secs(30))
                {
                    app.state::<SavePending>().0.store(false, Ordering::SeqCst);
                    *time = None;
                }
            }
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
            if last_running == Some(true)
                && !running
                && !app.state::<Recorder>().2.load(Ordering::SeqCst)
                && !app
                    .state::<activity::Activity>()
                    .starting
                    .load(Ordering::SeqCst)
            {
                activity::capture_error(&app,"Replay stopped unexpectedly. Open Settings → Diagnostics for the recorder error.".into());
            }
            if last_running != Some(running) {
                if let Some(controls) = app.try_state::<TrayControls>() {
                    let _ = controls.start.set_enabled(!running);
                    let _ = controls.save.set_enabled(running);
                    if let Some(tray) = app.tray_by_id("afterglow") {
                        let _ = tray.set_tooltip(Some(if running {
                            "Afterglow · Replay active"
                        } else {
                            "Afterglow · Replay idle"
                        }));
                    }
                    let _ = controls.stop.set_enabled(running);
                }
                last_running = Some(running);
            }
            if over_budget && activity::editor_busy(&app) {
                let _ = app.emit("memory-pressure", ());
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    });
}
fn initialize_security(uid: u32) -> Result<(), String> {
    if uid == 0 {
        return Err("Run Afterglow as your normal desktop user, without sudo.".into());
    }
    Ok(())
}
fn main() {
    // The capture subprocess has its own GPU path. WebKit preview uses software presentation
    // to avoid NVIDIA/Wayland zero-copy video surfaces that can remain black.
    if std::env::var_os("WEBKIT_GST_DISABLE_GL_SINK").is_none() {
        std::env::set_var("WEBKIT_GST_DISABLE_GL_SINK", "1");
    }
    if std::env::var_os("GST_PLUGIN_FEATURE_RANK").is_none() {
        std::env::set_var("GST_PLUGIN_FEATURE_RANK", "avdec_h264:300,avdec_aac:300");
    }

    // Apply before creating any WebKit, capture, or editing threads/processes.
    if let Err(error) = initialize_security(unsafe { libc::geteuid() }) {
        eprintln!("{error}");
        std::process::exit(1);
    }
    unsafe {
        libc::umask(0o077);
    }

    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("Afterglow {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if std::env::args().any(|arg| arg == "--doctor") {
        launcher::doctor();
        return;
    }
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!("Afterglow: launch the app, or use --show, --save-replay, --doctor, --version");
        return;
    }
    if std::env::args().skip(1).any(|arg| arg == "--toggle-replay") {
        match control::send("toggle") {
            Ok(message) => println!("{message}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args().skip(1).any(|arg| arg == "--show") && control::send("show").is_ok() {
        return;
    }
    if std::env::args().skip(1).any(|arg| arg == "--save-replay") {
        let result = if control::socket_path().is_ok_and(|p| p.exists()) {
            control::send("save").map(|message| println!("{message}"))
        } else {
            save_from_cli()
        };
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .plugin(tauri::plugin::Builder::<tauri::Wry>::new("navigation-policy").on_navigation(|_,url| {
            (url.scheme()=="tauri" && url.host_str()==Some("localhost")) ||
            (cfg!(debug_assertions) && url.scheme()=="http" && url.host_str()==Some("localhost") && url.port()==Some(1420))
        }).build())
        .setup(|app| {
            let mut prefs=preferences::load(&preferences_path(app.handle()).map_err(std::io::Error::other)?).unwrap_or_else(|error|{eprintln!("{error}");preferences::Preferences::default()});
            if let Some(capture)=prefs.capture.as_mut() {if repair_saved_session(capture,&detected_session()) {activity::event(app.handle(),"Saved capture session did not match the desktop; reset to Auto");}}
            let _=preferences::store(&preferences_path(app.handle()).map_err(std::io::Error::other)?,&prefs);
            let login=std::env::args().any(|a|a=="--autostart");
            if login&&!prefs.start_with_system {app.handle().exit(0);return Ok(());}
            let start_hidden=if login{!prefs.show_window_on_login}else{prefs.start_in_tray};
            if start_hidden {if let Some(window)=app.get_webview_window("main"){let _=window.hide();}}
            app.manage(preferences::PreferencesState(Mutex::new(prefs.clone())));
            app.manage(folders::Folders::new(preferences_path(app.handle()).map_err(std::io::Error::other)?.with_file_name("folders.json")));
            let root = output_dir(app.handle()).map_err(std::io::Error::other)?;
            let cache=app.path().app_cache_dir()?.join("previews");fs::create_dir_all(cache.parent().unwrap())?;
            app.manage(preview::Preview::new(cache.clone()).map_err(std::io::Error::other)?);
            app.manage(media::MediaServer::start(root,cache).map_err(std::io::Error::other)?);
            if let Err(error)=setup_tray(app.handle()) {eprintln!("Tray initialization failed: {error}. The app remains usable; relaunch to restore the library from game mode.");}
            if let Some(window) = app.get_webview_window("main") {
                reduce_web_cache(&window);
            }
            start_capture_worker(app.handle()).map_err(std::io::Error::other)?;
            control::start(app.handle().clone()).map_err(std::io::Error::other)?;
            refresh_hotkeys(app.handle());
            if prefs.auto_start_replay {let handle=app.handle().clone();tauri::async_runtime::spawn_blocking(move||{let _=start_saved_replay(&handle);});}
            if start_hidden&&!std::env::args().any(|a|a=="--show")&&app.try_state::<TrayControls>().is_some(){let handle=app.handle().clone();let _=app.run_on_main_thread(move||{let _=background_library(&handle);});}
            start_memory_monitor(app.handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if args.iter().any(|a|a=="--autostart") {return;}
            let _ = show_main(app);
        }))
        .manage(Recorder::default())
        .manage(ActiveSettings::default())
        .manage(GameMode::default())
        .manage(MemoryState::default())
        .manage(theme::Cache::default())
        .manage(hotkeys::HotkeyState::default())
        .manage(SavePending::default())
        .manage(CaptureWorker::default())
        .manage(activity::Activity::default())
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
            game_mode, get_preferences,save_preferences,apply_shortcuts,set_editor_active,window_action,assign_clips_folder,delete_clips,preview_bytes,report_playback_error,prepare_preview,open_clip_external,delete_clip,get_folders,create_folder,rename_folder,remove_folder,assign_clip_folder
        ])
        .build(tauri::generate_context!())
        .expect("Could not launch Afterglow")
        .run(|app, event| {
            if let tauri::RunEvent::WindowEvent{label,event:tauri::WindowEvent::CloseRequested{api,..},..}=&event {
                let close_to_tray=app.state::<preferences::PreferencesState>().0.lock().map(|p|p.close_to_tray).unwrap_or(false);
                if label=="main"&&close_to_tray&&app.try_state::<TrayControls>().is_some() {api.prevent_close();let handle=app.clone();let _=app.run_on_main_thread(move||{let _=background_library(&handle);});}
            }
            if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
                if code.is_none() && app.state::<GameMode>().0.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
            if let tauri::RunEvent::Exit = event {
                control::remove();
                if let Some(keys)=app.try_state::<hotkeys::HotkeyState>() {let _=hotkeys::apply(&keys,&preferences::Shortcuts::default());}
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
            ..Settings::default()
        }
    }
    #[test]
    fn patched_glib_string_iterator_is_safe() {
        use webkit2gtk::glib::variant::ToVariant;
        let value = vec!["one", "two", "three", "four"].to_variant();
        let mut iter = value.array_iter_str().unwrap();
        assert_eq!(iter.next(), Some("one"));
        assert_eq!(iter.next_back(), Some("four"));
        assert_eq!(iter.nth(1), Some("three"));
        assert_eq!(value.array_iter_str().unwrap().last(), Some("four"));
    }
    #[test]
    fn refuses_root_execution() {
        assert!(initialize_security(0).is_err());
        assert!(initialize_security(1000).is_ok());
    }
    #[test]
    fn saved_session_repairs_after_changing_desktops() {
        let mut settings = Settings::default();
        settings.session = "x11".into();
        assert!(repair_saved_session(&mut settings, "wayland"));
        assert_eq!(settings.session, "auto");
        settings.session = "wayland".into();
        assert!(!repair_saved_session(&mut settings, "wayland"));
        assert!(!repair_saved_session(&mut settings, "none"));
    }
    #[test]
    fn session_choice_matches_the_desktop() {
        assert_eq!(resolve_session("auto", "wayland").unwrap(), "wayland");
        assert_eq!(resolve_session("x11", "x11").unwrap(), "x11");
        assert!(resolve_session("x11", "wayland").is_err());
        assert!(resolve_session("wayland", "x11").is_err());
        assert!(resolve_session("auto", "none").is_err());
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
        assert!(args.windows(2).any(|x| x == ["-cursor", "yes"]));
        let mut scaled = settings();
        scaled.resolution = "1280x720".into();
        scaled.show_cursor = false;
        let scaled_args = capture_args(&scaled, "out").unwrap();
        assert!(scaled_args.windows(2).any(|x| x == ["-s", "1280x720"]));
        assert!(scaled_args.windows(2).any(|x| x == ["-cursor", "no"]));
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
