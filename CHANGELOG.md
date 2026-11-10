# Changelog

## 0.2.3 — 2026-11-10

- Move playback and video-tool controls into the video as overlays, with pointer/focus visibility and keyboard access.
- Replace the corrupted VP8-compatible-preview path with normalized H.264 Constrained Baseline / AAC MP4; validate decoded frames against the source. Originals remain unchanged.
- Reconcile enlarged layout with external desktop fullscreen changes. Use serialized, idempotent fullscreen requests and track ownership so closing a clip preserves externally entered fullscreen.
- Guard rapid enlargement, close-during-transition and stale Retry results; keep playback status consistent with pause/play events.
- Keep the platform GUI renderer restored in 0.2.2.

44 backend tests, 15 JavaScript tests, browser interaction checks and native Wayland video/window-state checks pass using a simulated recorder. The user’s exact corrupted clip and physical Hyprland shortcut are not directly reproduced.

## 0.2.2 — 2026-11-10

- Remove the forced software GUI rendering policy introduced in 0.2.1. The 0.2.1 binary loaded its interface without painting it under Wayland; the platform renderer restores the visible window.
- Add gst-plugins-base explicitly to both package recipes, supplying the missing Opus decoder needed by compatible previews. Artifact checks now require this dependency.
- Add an optional native Wayland paint regression test that checks actual compositor pixels.

Validated with 44 Rust tests, 11 JavaScript tests, Wayland compositor screenshots and native MP4/VP8-Opus Blob playback. The recorder remains simulated.

## 0.2.1 — 2026-11-10

- Use explicit fullscreen/windowed actions for the app and add Enlarge/Shrink to the player. F11 toggles fullscreen; Escape shrinks the player before closing it.
- Remove the Unfiled library tile. Unassigned clips remain in All clips, and No folder removes assignments.
- Select clips individually or select visible clips, move selections to folders, clear selection and delete with filename confirmation. Report partial deletion and retry only failed items.
- Play compatible previews from bounded native binary IPC/local Blobs instead of HTTP; revoke Blob URLs on close/reset and preserve original MP4 files. Limit preview output/transfer to 32 MiB.
- Set software WebKit composition, serialize editor transitions, fix opening after a status refresh, start media independently of duration probing, and expand playback factory checks and preview-completion diagnostics.

44 Rust tests, 11 JavaScript tests, browser interactions and native managed-window/MP4/Blob WebM/multi-selection checks passed with a simulated recorder. The user’s exact target playback failure remains subject to on-device confirmation.

## 0.2.0 — 2026-11-10

- Replace app window buttons inside the clip editor with a dedicated Close clip action. Closing a preview keeps the main window and background replay alive.
- Enable Play before WebKit has buffered future frames; request autoplay immediately. Stop treating already-decoded/playing video as a loading timeout.
- Accept authenticated opaque/alternate Tauri media origins for WebKit compatibility. Network/decode/unsupported-source failures can trigger the compatible preview. Keep the secret-token, exact Host, filename and range checks.
- Add local clip folders: create, rename, move, filter and remove. Removing folders keeps clips; renames preserve assignments and trimmed copies inherit the source folder.
- Review metadata writes, preserve corrupt indexes, roll back clip renames on metadata failure, and retain exported files if assignment fails.
- Add media-request diagnostics without secret URLs and show the running backend version in --doctor. Normal preview cancellation is no longer logged as a failed conversion.

Validated with 41 Rust tests, nine JavaScript tests, browser interaction tests and native WebKit MP4/WebM/trim/background tests using a simulated recorder. Target NVIDIA/Hyprland playback still requires confirmation on the user’s machine.
