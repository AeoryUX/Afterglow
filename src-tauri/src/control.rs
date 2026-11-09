//! Same-user control channel remains available with no webview running.
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
    time::Duration,
};
fn check_peer(socket: &UnixStream) -> Result<(), String> {
    use std::os::fd::AsRawFd;
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut credentials as *mut _ as *mut libc::c_void,
            &mut size,
        )
    } != 0
        || credentials.uid != unsafe { libc::geteuid() }
    {
        return Err("Afterglow control is available only to the current desktop user".into());
    }
    Ok(())
}
pub fn socket_path() -> Result<PathBuf, String> {
    Ok(crate::runtime_dir()?.join("afterglow-control.sock"))
}
pub fn send(action: &str) -> Result<String, String> {
    let mut socket = UnixStream::connect(socket_path()?)
        .map_err(|_| "Afterglow is not running. Open the app first.".to_string())?;
    check_peer(&socket)?;
    socket
        .set_read_timeout(Some(Duration::from_secs(15)))
        .map_err(|e| e.to_string())?;
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    writeln!(socket, "{action}").map_err(|e| e.to_string())?;
    let mut answer = String::new();
    BufReader::new(socket)
        .take(4096)
        .read_line(&mut answer)
        .map_err(|e| e.to_string())?;
    if let Some(value) = answer.strip_prefix("ok ") {
        Ok(value.trim().into())
    } else {
        Err(answer
            .strip_prefix("error ")
            .unwrap_or(&answer)
            .trim()
            .into())
    }
}
pub fn start(app: tauri::AppHandle) -> Result<(), String> {
    let path = socket_path()?;
    if path.exists() {
        if UnixStream::connect(&path).is_ok() {
            return Err("Another Afterglow control service is running".into());
        }
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    let listener = UnixListener::bind(&path).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        for connection in listener.incoming() {
            let Ok(mut socket) = connection else {
                break;
            };
            if check_peer(&socket).is_err() {
                continue;
            }
            let _ = socket.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = socket.set_write_timeout(Some(Duration::from_secs(2)));
            let mut action = String::new();
            let result = (|| {
                BufReader::new(&mut socket)
                    .take(64)
                    .read_line(&mut action)
                    .map_err(|e| e.to_string())?;
                match action.trim() {
                    "save" => crate::request_save(&app).map(|_| "Clip save requested".to_string()),
                    "toggle" => crate::toggle_replay(&app).map(|_| "Replay toggled".into()),
                    "show" => {
                        let handle = app.clone();
                        app.run_on_main_thread(move || {
                            let _ = crate::show_main(&handle);
                        })
                        .map_err(|e| e.to_string())?;
                        Ok("Library opened".into())
                    }
                    _ => Err("Unknown Afterglow command".into()),
                }
            })();
            let answer = match result {
                Ok(value) => format!("ok {value}"),
                Err(error) => format!("error {}", error.replace(['\n', '\r'], " ")),
            };
            let _ = writeln!(socket, "{answer}");
        }
    });
    Ok(())
}
use std::io::Read;
pub fn remove() {
    if let Ok(path) = socket_path() {
        let _ = std::fs::remove_file(path);
    }
}
