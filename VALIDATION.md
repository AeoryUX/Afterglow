# Afterglow 0.1.1 alpha validation

Validated on 2026-11-07 in a Debian x86_64 cloud build environment. The target desktop remains CachyOS + Hyprland + NVIDIA GTX 1660 Super.

## Passed

- Frontend production build (Vite).
- Native release build (Rust/Tauri 2), optimized and stripped. The release contains the compiled interface, not a Vite development server.
- Fifteen Rust tests: capture argument validation; safe filenames; rename collision protection; symlink rejection; trim ranges; real FFmpeg trim and original preservation; HTTP byte ranges; invalid range handling; token/path restrictions; safe system-tool environments in AppImage mode.
- Real media test: generated a 4-second H.264/AAC video, exported 1.0–2.5 seconds, checked output duration within 0.15 seconds, and confirmed the original stayed byte-for-byte identical.
- Browser preview: disabled recording outside Tauri, navigation, persistent preferences, search, and layouts at 900px/600px. No JavaScript errors.
- Browser with simulated native connection: start/save/stop, rename, trim, and open-folder interactions. No JavaScript errors.
- Compiled native desktop test using WebKitGTK WebDriver under a virtual X display: actual interface, Rust IPC, simulated recorder start/save/stop, actual saved fixture MP4, rename, FFmpeg trim, duration probing, library refresh, and native playback. Video reached readyState 4 with no media error through the loopback stream.
- Hotkey helper: rejects missing/invalid PIDs and unrelated processes; delivers SIGUSR1 to an owned recorder fixture.
- Release extraction and user-prefix installation; installed executable matches the built executable; desktop launcher and helper created.
- Release archive SHA-256 verification.

## Test boundaries

The native test substituted `nvidia-smi` and GPU Screen Recorder with test-only executables in the scratch workspace. It exercised the real capture process manager and signals, but did not record an actual GPU/display. Those substitutes are excluded from both release and source archives.

The cloud's GTK/WebKit dependencies were extracted into a local build sysroot. Its test-only WebKit library helper paths were relocated for this environment; those libraries are not shipped. Native playback QA needed permission for WebKit's normal `/var/tmp` media cache. The executable links the target machine's installed system libraries.

Real NVIDIA encoding, Wayland/KMS capture permissions, game performance, microphone/system-audio synchronization, and the actual Hyprland shortcut must still pass the hardware acceptance checklist in README.md. Audio output was not verified on a physical device. Do not describe hardware support as validated yet.

## Build details

Rust stable 1.99.1; Tauri 2.12.5; Vite 6.4.5. Release built with optimization level 1 and stripped symbols. Linux x86_64, glibc >= 2.39, dynamically linked GTK3/WebKitGTK 4.1. Cargo.lock and package-lock.json pin the dependency resolution.

## Packaging update

AppImage staging (AppDir) was tested with the compiled native UI and its bundled GTK/WebKit stack: simulated capture/save/stop, real host FFmpeg trimming, and native video playback reached readyState 4. The launcher environment is removed from external capture/driver/edit tools to keep those tools on their host libraries. The --save-replay command supports Hyprland keybinds directly. Real NVIDIA capture is still outside cloud validation.

## RAM and Caelestia update

The native packaged payload passed live Caelestia scheme changes, default 12 Mbps buffer validation, game-mode web-process teardown, saving without a library window, and reopening by relaunch. With a resident 90 MB recorder fixture, the measured process tree was approximately 469 MB with the UI and 247 MB in game mode. Physical NVIDIA capture remains unverified. See README.md for accounting limits and an optional systemd memory ceiling.
