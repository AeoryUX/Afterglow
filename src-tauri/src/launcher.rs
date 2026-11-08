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
    println!("A user launcher override takes precedence over the system launcher.");
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
