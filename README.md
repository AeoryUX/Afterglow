# Afterglow 0.1.4 alpha

A gameplay clipping app for Linux, targeting CachyOS + Hyprland and NVIDIA first. Afterglow provides replay controls and a local clip library, using GPU Screen Recorder for capture and FFmpeg for edits. AMD and Intel support follow NVIDIA hardware validation. The launcher fix preserves the existing application ID, clips, and settings.

## Install on CachyOS / Arch Linux

One-time setup and installation in one command:

```bash
curl -fsSL https://raw.githubusercontent.com/AeoryUX/Afterglow/packages/install-afterglow.sh -o afterglow-install.sh && sudo bash afterglow-install.sh --install
```

The download must complete successfully before the installer runs. You can review the downloaded script before running it with sudo.

After setup, either helper can install/manage the package:

| Package manager | Install command |
| --- | --- |
| paru | `paru -S afterglow-bin` |
| yay | `yay -S afterglow-bin` |
| pacman | `sudo pacman -S afterglow-bin` |

For repository setup without installing, use `sudo bash afterglow-install.sh --setup-only`.

This uses the project's signed pacman repository. An AUR account is not needed. AUR registration is temporarily closed; the AUR recipe remains available for future submission.

The setup pins publisher fingerprint `0BD7BEF64831B7CD5EA167C5952EE3DFB307F111`, verifies the repository signature, imports and locally trusts that key, adds `/etc/pacman.d/afterglow.conf`, and syncs only this repository. Package and database signatures are required. Keep your normal Arch/CachyOS system upgrades current.

The package pulls in GPU Screen Recorder, FFmpeg, GTK3/WebKitGTK, Ayatana tray support, playback plugins, xdg-utils, and NVIDIA user-space tools. Your existing NVIDIA kernel driver must already work with your CachyOS kernel. The package does not select a kernel module. This build targets current Linux x86_64 with glibc >= 2.39.

## Use

Launch Afterglow, open **Settings → Capture**, choose your monitor, and start replay. The default replay is 60 seconds at 12 Mbps, H.264, 60 FPS. Capture settings include Auto/Wayland/X11 session selection, native/1080p/720p resolution, cursor visibility, system audio and microphone. A session choice must match the current desktop; Xwayland inside Hyprland does not turn it into an X11 desktop.

**Keybinds** is available directly in the sidebar and as a Settings tab. Click a shortcut and press your preferred combination, then choose **Apply in Hyprland**. Defaults are Super+F8 to save, Super+F9 to start/stop replay, and Super+F10 to open the library. Existing named bindings are checked before changes; conflicting actions are kept. Runtime bindings are reapplied while Afterglow runs, including after compositor reloads (within about 15 seconds). No Hyprland configuration file is edited. For other desktops, the tab shows commands to assign in the desktop's shortcut settings:

```bash
afterglow --save-replay
afterglow --toggle-replay
afterglow --show
```

The **Clip storage** control at the bottom-left opens **Settings → Storage**. Choose a preset, custom capacity, or unlimited storage (the default). This is a total library budget. The next save reserves estimated video/audio space and pauses when it would exceed the budget. Existing clips are kept; there is no automatic deletion. A clip already being written can finish above the budget, so this is not a filesystem quota. Exports are limited to the remaining budget and an incomplete failed export is removed while its original is preserved.

A persistent capture worker keeps replay independent of the GUI and idle background task pools. Closing the window now keeps Afterglow in the tray by default, with the library web process released to save RAM. Replay and configured shortcuts keep working. The white monochrome tray menu offers Open Afterglow, Start replay, Save replay, Stop replay and Quit Afterglow. Reopen through the tray, shortcut, or app launcher. **Quit Afterglow** fully exits and stops capture. **Settings → Appearance & behavior** controls close-to-tray, replay on app startup, start-in-tray, and Caelestia/light/dark appearance. Replay must be active to save clips. App startup preferences do not add login autostart.

The clip library supports search, playback, rename, open-folder, and exporting a trimmed copy while preserving the original.

## Validation and limits

Thirty backend tests and native UI/playback/trim/theme/background tests passed with a simulated recorder. One packaged test measured approximately 410 MB with the library open and 247 MB in the tray while the fixture held a resident 90 MB buffer. These figures are not a GTX 1660 Super recording benchmark or a universal RAM ceiling. Real NVIDIA/Wayland capture, permissions, audio sync, and game performance still require hardware testing.

The RAM target is below 1 GB, with a bounded encoded replay buffer and automatic game-mode switching above 850 MB of measured app/process-tree memory. GPU VRAM, all filesystem cache, and external shared desktop/kernel allocations are outside that readout. See VALIDATION.md and PACKAGING-VALIDATION.md for the test boundaries.

Clips and preferences stay local. The selected display includes other windows and notifications. Nothing is uploaded by the app; playback uses a token-protected loopback stream.

## Security

The security protections introduced in 0.1.3 include private clip/log directories, safer file-descriptor access, restricted video processing, root refusal, a tighter app permission policy, and a GLib safety fix. See [SECURITY.md](SECURITY.md) for the implemented boundaries, dependency review, and remaining work.

## Troubleshoot

Version 0.1.2 fixed a package bug that made the system desktop entry readable only by root. Existing installations can update with `paru -Syu afterglow-bin` or `yay -Syu afterglow-bin`. Close and reopen the launcher after updating. If it is still missing, run `afterglow --doctor` to inspect launcher paths, permissions, and a possible user override. Do not run the app with sudo.

Read Settings → Diagnostics → Capture log after a failed start. Check `nvidia-smi`, `gpu-screen-recorder --info`, and `gpu-screen-recorder --list-capture-options`. Follow the recorder package's KMS permission instructions and run Afterglow as your normal user. A clip must finish writing before FFprobe can read it for trimming.

## Build from source

```bash
sudo pacman -S --needed base-devel rust nodejs npm pkgconf webkit2gtk-4.1 libayatana-appindicator openssl librsvg
npm ci
npm run tauri dev
```

Hyprland shortcut setup uses the hyprctl already provided by Hyprland. It is optional for users assigning commands through another desktop.

Build a native executable with `npm run tauri build -- --no-bundle`. Run backend/media tests with `cargo test --manifest-path src-tauri/Cargo.toml -- --include-ignored` (FFmpeg with libx264/FFprobe required).

## Source, packages, and license

Application source is in this branch. Signed package files are in the [packages branch](https://github.com/AeoryUX/Afterglow/tree/packages); versioned native archives and corresponding source are under its `releases/` directory. See the [0.1.4 release](https://github.com/AeoryUX/Afterglow/releases/tag/v0.1.4) and [repository information](https://github.com/AeoryUX/Afterglow/releases/tag/arch-repository).

Afterglow's original code is GPL-3.0-only. See COPYING and LICENSE.md. Dependencies keep their respective licenses; notices are included in THIRD_PARTY_NOTICES.txt and package distributions.
