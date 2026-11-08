# Afterglow 0.1.2 alpha validation

Validated on 2026-11-08 in a Linux x86_64 cloud environment. The target desktop is CachyOS + Hyprland + Caelestia + NVIDIA GTX 1660 Super.

## Passed

- Vite production build and optimized, stripped Rust/Tauri release build.
- Sixteen Rust backend/media tests, including real FFmpeg trimming, original preservation, filename/path boundaries, HTTP byte ranges, capture settings, process-tree memory, and retaining/recovering a valid palette while a Caelestia scheme is partially written.
- Two JavaScript preference tests: corrupt/legacy settings recover safe defaults and excessive encoded replay budgets are bounded.
- Browser preview: capture guard, navigation, saved preferences, search, 900px/600px layouts, and no JavaScript errors.
- Browser with a simulated native bridge: start/save/stop, rename, trim, open-folder, delayed monitor detection, forced diagnostic refresh, and live capture controls on the setup page without discarding unsaved preferences.
- Native WebKitGTK test under Xvfb: actual compiled interface/Rust IPC, recorder start/save/stop, real MP4 duration/rename/trim/playback (readyState 4), live dark/light Caelestia schemes, partial scheme recovery, and setup controls following external replay state.
- Game mode tears down the web process, preserves recording and saving, and relaunch restores the library. The recorder fixture held a resident 90 MB buffer. Measured tree PSS: 404,080,640 bytes with the library and 246,157,312 bytes in game mode. These are fixture results, not a hardware benchmark or a universal ceiling.
- Headless --version, --help, --doctor, and --save-replay; the latter signals an owned recorder and rejects negative PIDs.
- Package regression checks under umask 077, desktop-file validation, pacman metadata, SHA-256 integrity, and package/repository signatures. See PACKAGING-VALIDATION.md.

## Limits

The native test substitutes nvidia-smi and GPU Screen Recorder with scratch fixtures. Real NVIDIA encoding, Wayland/KMS capture permissions, microphone/system-audio synchronization, game performance, and the actual Hyprland shortcut require target-machine testing. No physical audio device was tested. A live Caelestia shell was not available; its launcher and Quickshell's desktop-entry loader were reviewed from source.

Native tests use the existing bundled GTK/WebKit payload because the rootless cloud sysroot has relocated helper paths. WebKit's normal /var/tmp media cache was permitted for testing. The native release links the target machine's system libraries. Test fixtures and the private signing key are excluded from source and release archives.

## Build

Rust stable 1.99.1; Tauri 2.12.5; Vite 6.4.5. Release optimization level 1 with stripped symbols. Linux x86_64, glibc >= 2.39, GTK3/WebKitGTK 4.1. Cargo.lock and package-lock.json pin dependency resolution.
