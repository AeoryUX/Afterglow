use std::process::Command;

// AppRun injects its bundled GUI libraries into the environment. Native capture,
// driver tools, FFmpeg, and desktop utilities must use the host's matching stack.
pub fn command(program: &str) -> Command {
    make_command(
        program,
        std::env::var_os("APPIMAGE").is_some() || std::env::var_os("APPDIR").is_some(),
    )
}
fn make_command(program: &str, bundled: bool) -> Command {
    let mut command = Command::new(program);
    if bundled {
        if let Some(appdir) = std::env::var_os("APPDIR") {
            for key in ["PATH", "XDG_DATA_DIRS"] {
                if let Some(value) = std::env::var_os(key) {
                    let paths = std::env::split_paths(&value)
                        .filter(|path| !path.starts_with(std::path::Path::new(&appdir)))
                        .collect::<Vec<_>>();
                    if let Ok(value) = std::env::join_paths(paths) {
                        command.env(key, value);
                    }
                }
            }
        }
        for key in [
            "LD_LIBRARY_PATH",
            "LD_PRELOAD",
            "GTK_PATH",
            "GTK_EXE_PREFIX",
            "GTK_DATA_PREFIX",
            "GSETTINGS_SCHEMA_DIR",
            "GI_TYPELIB_PATH",
            "GIO_MODULE_DIR",
            "GDK_PIXBUF_MODULE_FILE",
            "GST_PLUGIN_PATH",
            "GST_PLUGIN_PATH_1_0",
            "GST_PLUGIN_SYSTEM_PATH",
            "GST_PLUGIN_SYSTEM_PATH_1_0",
            "GST_PLUGIN_SCANNER",
            "PYTHONHOME",
            "PYTHONPATH",
            "PERLLIB",
            "PERL5LIB",
            "QT_PLUGIN_PATH",
            "QML2_IMPORT_PATH",
            "GST_PLUGIN_SCANNER_1_0",
            "GST_PTP_HELPER_1_0",
            "APPDIR",
            "APPIMAGE",
        ] {
            command.env_remove(key);
        }
    }
    command
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_commands_do_not_inherit_bundle_libraries() {
        let command = make_command("gpu-screen-recorder", true);
        let removed = command
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(key, _)| key.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        for key in ["LD_LIBRARY_PATH", "LD_PRELOAD", "GST_PLUGIN_PATH", "APPDIR"] {
            assert!(removed.iter().any(|k| k == key));
        }
        for key in [
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "XDG_RUNTIME_DIR",
            "PULSE_SERVER",
        ] {
            assert!(!removed.iter().any(|k| k == key));
        }
        assert_eq!(
            make_command("gpu-screen-recorder", false)
                .get_envs()
                .count(),
            0
        );
    }
}
