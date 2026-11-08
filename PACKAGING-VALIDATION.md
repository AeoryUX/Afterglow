# Distribution packaging validation

Afterglow 0.1.1 alpha now has an AppImage and an Arch binary package.

- AppImage: `Afterglow-0.1.1-x86_64.AppImage`, approximately 185 MiB. GTK3, WebKitGTK, playback libraries, and dependency notices are included. GPU Screen Recorder, FFmpeg, the NVIDIA driver, and the system's glibc remain host dependencies.
- Arch: `afterglow-bin-0.1.1-1-x86_64.pkg.tar.zst`, approximately 3.6 MiB. Installs the executable, shortcut helper, desktop entry, icon, README, and third-party notices. Host dependencies are recorded in `.PKGINFO`.
- Both target Linux x86_64 with glibc >= 2.39. These are local alpha packages, not signed public releases or a published AUR entry.

## Verified

1. All fifteen Rust backend/media tests pass, including the AppImage system-command environment guard.
2. The AppImage payload (AppDir) launches with its bundled GTK/WebKit stack, handles simulated capture/save/stop, runs host FFmpeg for trim/duration, and plays MP4 video to readyState 4.
3. The final AppImage itself passes the same native WebKit WebDriver flow using the runtime's extract-and-run mode. No FUSE device is available in the cloud environment, so FUSE mounting was not tested.
4. Both the payload and final AppImage's `--save-replay` command signal a user-owned GPU Screen Recorder executable fixture and reject a negative PID. This works without opening a desktop window.
5. Pacman 7/libalpm 15 accepts the Arch package metadata and lists its installed paths. The package has root ownership metadata, `.PKGINFO`, and a gzip-compressed `.MTREE`. Full installation on CachyOS was not performed in this Debian cloud environment.
6. Native release, Arch package, AppImage, and source ZIP have SHA-256 checksums.

The AppImage uses the official Tauri/linuxdeploy toolchain with GTK and GStreamer bundling. In this rootless build environment, a local configuration supplies the installed WebKit subprocess files and libav playback plugin from a sysroot. Those files are placed under standard paths inside the image. Unlike the earlier bare-executable cloud QA, this final AppImage test uses its own bundled WebKit helpers and libraries.

AppRun modifies several library/runtime environment variables. Afterglow clears those injected paths for external recording/driver/editing tools while preserving desktop-session variables and non-bundled search paths. This prevents its GUI bundle from becoming the driver or codec stack for GPU Screen Recorder or FFmpeg.

The recorder and GPU detection in cloud UI tests are fixtures. Actual NVIDIA recording, Wayland/KMS capture permissions, game performance, microphone/system-audio synchronization, and the real Hyprland binding still require the hardware acceptance checks in README.md.

## Local install

```bash
paru -S --needed gpu-screen-recorder ffmpeg
chmod +x Afterglow-0.1.1-x86_64.AppImage
./Afterglow-0.1.1-x86_64.AppImage
```

Alternatively:

```bash
paru -S --needed gpu-screen-recorder
paru -U ./afterglow-bin-0.1.1-1-x86_64.pkg.tar.zst
```

`paru -S afterglow-bin` is unavailable until a public release source and AUR entry are published. The provided PKGBUILD uses the matching native release archive locally and verifies its checksum.

Version 0.1.1 adds a tray, low-memory game mode, measured RAM display, a bounded replay bitrate/duration budget, and read-only Caelestia appearance synchronization. The native package now declares libayatana-appindicator; the AppImage includes its tray libraries. Memory results use a recorder fixture and do not establish a real GPU capture ceiling.

The final 0.1.1 AppImage extract-and-run test measured 419,015,680 bytes with the library and fixture recorder active, and 256,099,328 bytes in game mode. Caelestia live colour/light-mode changes, saving with the library closed, reopening by relaunch, playback, and the --save-replay ownership/PID guards passed. The resident recorder fixture held 90,000,000 bytes, and no physical GPU capture was exercised. See the accompanying afterglow-0.1.1-memory-test.json for the raw figures and accounting boundary.
