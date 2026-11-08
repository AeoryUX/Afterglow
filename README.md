# Afterglow 0.1.1 alpha

NVIDIA-first gameplay clipping for Linux. Target: CachyOS + Hyprland + Caelestia, starting with a GTX 1660 Super. Rust/Tauri manages the desktop app, GPU Screen Recorder handles the rolling recording, and FFmpeg handles clip edits. AMD and Intel support are next, after NVIDIA validation.

## What works in this alpha

- Start/stop a 30, 60, 90, or 120-second instant-replay buffer.
- H.264, 8–20 Mbps (12 Mbps default), 30 or 60 FPS, at the chosen display's **native resolution**.
- Default system audio and optional default microphone.
- Save a replay from the app or a Hyprland shortcut.
- Browse and search local MP4 clips, preview, rename, and open the library folder.
- Export a trimmed copy with frame-accurate FFmpeg cuts; keep the original.
- Monitor suggestions, NVIDIA/Wayland/backend diagnostics, capture logs, and local preferences.
- One app instance. Stopping/closing the app stops its recorder; Linux parent-death signaling also stops capture if the app crashes.

This is an early alpha. Real NVIDIA/Hyprland capture, audio synchronization, and game performance must still be tested on the target machine. The cloud build machine has no NVIDIA GPU or Hyprland. Passing software tests does not verify hardware capture.

There is no game detection, automatic startup, uploading, or AMD/Intel support yet. Clip covers are placeholders, not generated thumbnails. MP4 playback depends on your installed WebKit/GStreamer codecs.

## RAM target and game mode

This build targets **under 1 GB of app + capture RAM**. This is a measured target and an automatic recovery threshold, not a universal hard allocation limit. Actual NVIDIA/Wayland capture still needs measurement on your machine.

- The default video replay buffer is 60 seconds at 12 Mbps, approximately 90 MB of encoded video. Bitrate and duration combinations cannot exceed 180 MB of encoded video; audio, capture surfaces, the driver, and the interface add overhead.
- Click **Game mode** after starting replay. The library window and its web process close while recording and hotkeys continue. Use the tray to save/stop/quit, or reopen the library from the tray or by launching Afterglow again. An ordinary window close still exits and stops capture.
- While replay is active, measured usage above 850 MB automatically closes the library into game mode. If the library keeps closing, stop replay from the tray, reopen it, and lower the bitrate/duration before restarting.
- The RAM readout samples Afterglow and its descendants, including its recorder and playback helpers. It uses proportional resident memory when readable to avoid counting shared pages twice, with an RSS estimate as fallback. It does not represent GPU VRAM, all system cache, or allocations in external shared desktop/kernel services.
- Clip previews load only when opened and are unloaded when closed. WebKit uses its document-viewer cache policy; game mode clears the cache and terminates the library's web process.

In a native cloud test with a simulated recorder holding a real resident 90 MB buffer, usage measured approximately **469 MB with the library open** and **247 MB in game mode**. Saving still worked in game mode, and relaunch restored the interface. These numbers are not a GTX 1660 Super recording benchmark. The raw test report is provided with the downloads.

For the least packaging overhead on CachyOS, use the native Arch package. Prefer normal FUSE execution for the AppImage. Extract-and-run can consume substantial extra memory when its temporary directory is on tmpfs; extracting once to disk avoids repeated temporary copies.

If you require a kernel-enforced ceiling, quit any existing instance first, then an optional systemd user scope can request one:

```bash
systemd-run --user --scope -p MemoryHigh=800000000 -p MemoryMax=1000000000 -p MemorySwapMax=0 afterglow
```

That limits memory charged to the scope. The kernel can terminate Afterglow at the limit, losing any unsaved replay. This command requires a working systemd user manager and has not been exercised on the cloud build host.

## Caelestia appearance sync

**Follow Caelestia** is the default appearance setting. Afterglow reads `$XDG_STATE_HOME/caelestia/scheme.json`, falling back to `~/.local/state/caelestia/scheme.json`, using Caelestia's documented `name`, `mode`, and Material 3 `colours` structure. Accent, surfaces, text, borders, and light/dark mode follow the active scheme.

The interface checks for updates during its normal status refresh (about every 2.5 seconds), with modification-time caching. There is no extra theme watcher process, no network request, and no change to your Caelestia configuration. While the library is closed, theme polling stops; reopening reads the current scheme.

If the scheme is missing, incomplete, or invalid, Afterglow falls back to its built-in dark theme. You can choose **Afterglow dark** or **Afterglow light** in Capture setup instead. The native test verified an on-disk Caelestia scheme change from a blue dark palette to a rose light palette without restarting the app.

Source format references: [Caelestia Colours.qml](https://github.com/caelestia-dots/shell/blob/main/services/Colours.qml) and [Paths.qml](https://github.com/caelestia-dots/shell/blob/main/utils/Paths.qml).

## AppImage (simplest launch)

Install the native capture/edit tools once on CachyOS:

```bash
paru -S --needed gpu-screen-recorder ffmpeg
```

A working NVIDIA driver is also required. The AppImage includes GTK/WebKit and its playback libraries, but keeps capture/driver tools on the system so they match your driver installation. It targets current Linux x86_64 with glibc 2.39 or newer.

```bash
chmod +x Afterglow-0.1.1-x86_64.AppImage
./Afterglow-0.1.1-x86_64.AppImage
```

If FUSE is unavailable, use:

```bash
./Afterglow-0.1.1-x86_64.AppImage --appimage-extract-and-run
```

Keep the AppImage at a stable path, such as `~/Applications/Afterglow-0.1.1-x86_64.AppImage`, then add this to your Hyprland user configuration:

```ini
bind = SUPER, F8, exec, ~/Applications/Afterglow-0.1.1-x86_64.AppImage --save-replay
```

The app and replay buffer must already be running. No separate helper installation is needed for this binding.

## CachyOS / Arch package with paru

Install the native capture dependency and then the downloaded local package:

```bash
paru -S --needed gpu-screen-recorder
paru -U ./afterglow-bin-0.1.1-1-x86_64.pkg.tar.zst
```

This registers Afterglow with pacman and adds it to the application launcher. It can be removed with `paru -Rns afterglow-bin`; clips and preferences stay in your user directories. Its desktop-library and FFmpeg dependencies are declared in the package metadata.

You can also build the binary package locally with `packaging/arch/PKGBUILD`: place the matching native release tarball alongside it and run `makepkg -si` (install any AUR dependencies first) or `paru -Bi .` from that recipe directory. The recipe checks the release archive's SHA-256.

There is no public AUR entry yet, so `paru -S afterglow-bin` is not available. Publishing requires a public release URL and an AUR maintainer account; the provided recipe currently uses a local archive.

## Install a prebuilt Linux x86_64 release

The archive contains the executable, an installer, an icon, and the shortcut helper. It does **not** bundle GPU Screen Recorder, FFmpeg, or system GTK/WebKit libraries.

On CachyOS, install runtime dependencies through your package manager:

```bash
sudo pacman -S --needed webkit2gtk-4.1 gtk3 libayatana-appindicator ffmpeg gst-libav gst-plugins-good
```

Install the native `gpu-screen-recorder` CLI from your configured repositories or your preferred AUR helper if it is unavailable there. Check `nvidia-smi` works with your installed NVIDIA driver. Follow the recorder package's KMS/capture permission instructions; run Afterglow as your normal desktop user.

Extract the release and install it:

```bash
tar -xzf afterglow-0.1.1-linux-x86_64.tar.gz
cd afterglow-0.1.1-linux-x86_64
./install.sh
```

Launch **Afterglow** from your application launcher, or:

```bash
~/.local/bin/afterglow
```

You can also run `./afterglow` directly from the extracted folder. The prebuilt executable dynamically links Linux system libraries; it is intended for a current x86_64 CachyOS installation, not every Linux distribution. This build requires glibc 2.39 or newer. If a library is missing, inspect `ldd ./afterglow` and install the matching runtime package.

The installer writes only into `~/.local` by default, replacing previous Afterglow files there on updates. To use a different user-owned prefix, set `AFTERGLOW_PREFIX`. No root access is needed for the installer. Recorded clips are never removed by installation.

## First clip

1. Open **Capture setup**. Pick your monitor name from the suggestions or run `hyprctl monitors` and enter it (e.g. `DP-1` or `HDMI-A-1`).
2. Choose replay duration, frame rate, and audio sources. Save preferences.
3. Click **Start replay**. If capture exits, read **Capture setup → Capture log**.
4. Let a few seconds accumulate, play, then click **Save clip** or use the configured shortcut.
5. Open the clip in the library. Rename it, or set start/end times in seconds and export a trimmed copy.

Saving is asynchronous. The app reports when the recorder prints a saved MP4 path; large files can take time to write. Settings changed during recording apply after restarting replay; the active recording settings remain displayed until then. The default buffer uses roughly 90 MB per 60 seconds at 12 Mbps, plus audio and overhead. Bitrate/duration combinations are capped at 180 MB of encoded video.

Trim exports use CPU H.264 (`libx264`, two encoder threads) and AAC audio, independently of NVENC replay capture. All audio tracks are included when present. Trimming may affect game performance on a busy CPU. Failed exports are removed; originals stay intact. Use the timestamps from the preview to choose the range.

## Hyprland / Caelestia shortcut

The release installer adds `~/.local/bin/afterglow-save`. Add this to the **user override** configuration that your Caelestia/Hyprland setup sources:

```ini
bind = SUPER, F8, exec, ~/.local/bin/afterglow-save
```

Run `hyprctl reload`. Configuration paths vary by Caelestia version; keep its generated files intact and resolve any existing Super+F8 binding first.

Alternatively bind `afterglow --save-replay` when the executable is installed on PATH.

The shortcut works while Afterglow and its replay buffer are running. It verifies the saved PID belongs to your user and is a GPU Screen Recorder process, then sends SIGUSR1 only to that process. It does not signal all recorders. The shortcut is not registered automatically by the app.

## Storage and privacy

Clips stay in your XDG Videos directory under `Afterglow`, falling back to `~/Videos/Afterglow` when the Videos directory is not configured. The library folder is displayed in the app and accessible with **Open folder**. Preferences are stored locally in the webview. Capture stderr is saved in the platform application log directory and shown in Capture setup.

The selected display includes other windows and desktop notifications. Playback uses a loopback-only HTTP stream with a random per-session access token and byte-range support for seeking. The stream exposes only regular MP4 files in the library. Nothing is uploaded.

## Troubleshooting

Run these in a terminal inside your desktop session:

```bash
nvidia-smi
gpu-screen-recorder --info
gpu-screen-recorder --list-capture-options
gpu-screen-recorder --list-audio-devices
```

- **Capture exits:** read the in-app log. Check monitor names, driver compatibility, and the recorder package's KMS permission setup.
- **No microphone:** check the default input in your desktop audio settings. The recorder receives microphone and system audio as separate inputs/tracks; your player may select only one track.
- **MP4 won't preview:** install the relevant GStreamer codecs (on Arch, typically `gst-libav` and `gst-plugins-good`) or use Open folder and play the file in your preferred player.
- **Clip cannot be trimmed yet:** wait for saving to finish, then retry. `ffprobe` needs a finalized MP4.
- **Rename fails:** filenames must end in `.mp4` and cannot contain slashes or control characters. Existing clips are never overwritten.
- **No capture in browser:** `npm run dev` is an interface preview only. Use the desktop app to capture gameplay.

Upstream capture documentation: https://git.dec05eba.com/gpu-screen-recorder/about/

## Build from source

On CachyOS, install Rust/Cargo, Node.js/npm, and native build requirements:

```bash
sudo pacman -S --needed base-devel rust nodejs npm pkgconf webkit2gtk-4.1 libayatana-appindicator openssl librsvg
npm ci
npm run tauri dev
```

Build and package an executable:

```bash
npm run tauri build -- --no-bundle
./scripts/package-release.sh
```

The release archive and SHA-256 checksum are written to `releases/`. For an AppImage, run `npm run package:appimage` on a Linux build machine with the native build dependencies installed. To generate a local Arch package, update the PKGBUILD checksum after packaging the native release, then run `scripts/package-arch.sh` (requires bsdtar/libarchive and zstd). `npm run build` alone builds only the interface. Rust and npm lockfiles are included.

Run backend checks, including a real generated-video trim test:

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --include-ignored
```

The media test requires FFmpeg with libx264 and FFprobe. It generates a short test video, exports a 1.5-second range, checks its duration, and confirms the original is unchanged.

## NVIDIA hardware acceptance

Before calling this hardware-ready, verify on the target CachyOS machine:

1. Capture a game, save an MP4, and verify frame rate, resolution, and audio sync.
2. Test system audio, microphone, both, and no audio.
3. Save while a game has focus; confirm unrelated recorders are unaffected.
4. Stop/restart, close/reopen, and verify the app's recorder exits correctly.
5. Compare game frame rate, memory, and encoder use with replay on/off.

Only after these pass should NVIDIA support be considered validated. Then add AMD/VA-API, followed by Intel/VA-API.

## License and AUR publishing

Afterglow is GPL-3.0-only; see COPYING and LICENSE.md. The `packaging/aur` directory contains the public-release PKGBUILD and publishing guide. Its dependencies include GPU Screen Recorder, FFmpeg, GTK/WebKit, playback/tray libraries, and nvidia-utils, so a published `paru -S afterglow-bin` entry can install the app dependencies in one operation. The correct NVIDIA kernel driver remains part of the existing CachyOS setup. The entry is not live until the release is hosted and the AUR repository is pushed.

## AUR registration temporarily closed: signed repository alternative

New AUR account registration is temporarily closed. A signed pacman repository is prepared as an alternative, so after one-time repository setup users can still run `paru -S afterglow-bin`. See packaging/repository/HOSTING.md. The repository is not hosted yet; it needs the AeoryUX/afterglow GitHub repository and release assets first. No AUR account is needed for this alternative.
