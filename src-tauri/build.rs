fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "status",
            "start_replay",
            "save_replay",
            "stop_replay",
            "list_clips",
            "rename_clip",
            "clip_duration",
            "clip_media_url",
            "trim_clip",
            "open_library",
            "capture_options",
            "game_mode",
            "get_preferences",
            "save_preferences",
            "apply_shortcuts",
        ]),
    ))
    .expect("Cannot build application permissions");
}
