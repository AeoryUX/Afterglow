# Afterglow 0.1.4 alpha validation

Validated on 2026-11-09 in a Linux x86_64 cloud environment. The target desktop is CachyOS + Hyprland + Caelestia + NVIDIA GTX 1660 Super.

## Passed

- Vite production build and optimized, stripped Rust/Tauri release build.
- Thirty optimized Rust backend/media tests, including real FFmpeg trimming, original preservation, filename/path boundaries, HTTP byte ranges, capture settings, process-tree memory, and retaining/recovering a valid palette while a Caelestia scheme is partially written.
- Three JavaScript preference/key-recorder tests: corrupt/legacy settings recover safe defaults and excessive encoded replay budgets are bounded.
- Browser preview: capture guard, navigation, saved preferences, search, 900px/600px layouts, and no JavaScript errors.
- Browser with a simulated native bridge: start/save/stop, rename, trim, open-folder, delayed monitor detection, forced diagnostic refresh, and live capture controls on the setup page without discarding unsaved preferences.
- Native WebKitGTK test under Xvfb: actual compiled interface/Rust IPC, recorder start/save/stop, real MP4 duration/rename/trim/playback (readyState 4), live dark/light Caelestia schemes, partial scheme recovery, and setup controls following external replay state.
- Game mode tears down the web process, preserves recording and saving, and relaunch restores the library. The recorder fixture held a resident 90 MB buffer. Measured tree PSS: 410,335,232 bytes with the library and 247,202,816 bytes in game mode. These are fixture results, not a hardware benchmark or a universal ceiling.
- Headless --version, --help, --doctor, and --save-replay; the latter signals an owned recorder and rejects negative PIDs.
- Package regression checks under umask 077, desktop-file validation, pacman metadata, SHA-256 integrity, and package/repository signatures. See PACKAGING-VALIDATION.md.

## Settings and tray regression checks

The browser flow checks the direct Keybinds and bottom-left Storage navigation, accessible tab keyboard navigation, shortcut recording, storage presets/custom/unlimited controls, saved capture/session/resolution/cursor/behavior choices, and responsive layouts. The mocked native bridge preserves the existing start/save/stop/rename/trim/folder flows and unsaved capture drafts during status polling.

Native WebKit tests use the compiled UI and real Rust IPC. A test-only hyprctl models binding queries and mutations; tests verify applying, repeated application without duplicates, rebinding, and preserving an existing foreign shortcut. A wrong session selection is rejected before capture starts. Private preferences round-trip, valid capacity bounds, and the replay buffer budget have backend tests.

The recorder is spawned by a persistent capture worker so its parent-death signal remains tied to a live thread throughout background recording. A WM_DELETE_WINDOW client message exercises the actual close event under Xvfb. The app and simulated recorder remain alive, the web process is released, the recorder survives 20 seconds with no UI (past async-pool idle retirement), the exact configured shortcut command saves a clip while closed, and --show reopens the library. --toggle-replay stops and restarts capture through the local control channel. A fresh native X11-mode launch verifies saved capture flags, automatic replay, start-in-tray, explicit reopen, and close-to-quit behavior. A storage budget blocks saves before creating files; limited exports fail cleanly without altering the original.

These tests simulate the recorder and Hyprland IPC; they do not establish real compositor hotkey delivery, a visible Caelestia tray host, or NVIDIA capture performance. The white tray asset is verified as transparent RGBA with white RGB pixels. Capture cursor and scale options were checked against the GPU Screen Recorder 6.1.7 package usage.

## Security regression checks

The optimized backend suite includes private file/directory permissions, symlink/hard-link rejection without altering the target, FIFO rejection without blocking, stable inode reads after a filename swap, blocked sockets, blocked process creation, bounded diagnostic output, timeout cleanup, root refusal, and the backported GLib iterator fix. The HTTP tests reject foreign Origin/Host headers, wrong tokens, traversal, and clip symlinks. Real host FFmpeg still exports the correct trim while preserving the source.

Native WebKit tests confirm mode 0700 for the library, mode 0600 for recorded clips, PID state and exports, denied broad native monitor access, blocked external navigation, and normal MP4 playback under the production CSP. The save command still signals an owned recorder and rejects a negative PID after switching to pidfds.

npm audit reports no advisories in the current lockfile. OSV reports the backported GLib issue under two identifiers and four unmaintained transitive-crate warnings; the audit displays the documented exceptions and fails on new/unreviewed issues, expired review, or a changed backport checksum. See SECURITY.md and dependency-review.json. These scans cover the pinned app dependencies, not every system package.

## Limits

The native test substitutes nvidia-smi and GPU Screen Recorder with scratch fixtures. Real NVIDIA encoding, Wayland/KMS capture permissions, microphone/system-audio synchronization, game performance, and the actual Hyprland shortcut require target-machine testing. No physical audio device was tested. A live Caelestia shell was not available; its launcher and Quickshell's desktop-entry loader were reviewed from source.

Native tests use the existing bundled GTK/WebKit payload because the rootless cloud sysroot has relocated helper paths. WebKit's normal /var/tmp media cache was permitted for testing. The native release links the target machine's system libraries. Test fixtures and the private signing key are excluded from source and release archives.

## Build

Rust stable 1.99.1; Tauri 2.12.5; Vite 6.4.5. Release optimization level 1 with stripped symbols. Linux x86_64, glibc >= 2.39, GTK3/WebKitGTK 4.1. Cargo.lock and package-lock.json pin dependency resolution.
