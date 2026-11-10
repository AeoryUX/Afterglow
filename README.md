# Afterglow 0.2.2 alpha

A local gameplay clipping app for Linux, targeting CachyOS + Hyprland + Caelestia. The current capture support requires a working NVIDIA driver. Afterglow uses GPU Screen Recorder for capture and FFmpeg for edits.

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

The package pulls in GPU Screen Recorder, FFmpeg, PipeWire audio tools, GTK3/WebKitGTK, Ayatana tray support, playback/parser plugins (including gst-plugins-base), xdg-utils, and NVIDIA user-space tools. Your existing NVIDIA kernel driver must already work with your CachyOS kernel. The package does not select a kernel module. This build targets current Linux x86_64 with glibc >= 2.39.

## Home, library, and desktop controls

**Home** opens first and contains replay controls, status, storage/RAM information and recent clips. **Clip library** is reserved for searching, sorting, previewing, renaming and trimming clips. **Keybinds** has its own active sidebar state. Valid key selections apply automatically and update the shortcut shown on Home. Failed changes keep an inline error and can be retried with Apply in Hyprland. The interface follows Caelestia's live palette, with rounded surfaces and glowing on/off switches. Built-in choices also include Forest dark/light, Graphite, Midnight blue, Lavender, Rose light and Sand light.

A short original tone plays after a saved clip is confirmed, including from the tray. Disable **Clip saved sound** in **Settings → Appearance & behavior** to mute it. Playback uses the installed PipeWire `pw-play` command; there is no additional persistent audio process.

## Use

Replay starts automatically when Afterglow opens, using saved capture settings (or the full display by default). The first upgrade from 0.1.4 enables this once; afterward your choice in **Settings → Appearance & behavior** is preserved. Open **Settings → Capture** to choose your monitor and restart replay to apply changes. Save a clip with **Save clip** or your configured shortcut; buffering alone does not create a library file. The default replay is 60 seconds at 12 Mbps, H.264, 60 FPS. Capture settings include Auto/Wayland/X11 session selection, native/1080p/720p resolution, cursor visibility, system audio and microphone. A session choice must match the current desktop; Xwayland inside Hyprland does not turn it into an X11 desktop.

**Keybinds** is available directly in the sidebar and as a Settings tab. Use the dropdowns or press your preferred key, and changes apply automatically. Select **None** for a single key such as F2 or `/`. Single keys trigger globally, including while typing. Conflicting bindings are preserved. Defaults are Super+F8 to save, Super+F9 to start/stop replay, and Super+F10 to open the library. Existing named bindings are checked before changes; conflicting actions are kept. Runtime bindings are reapplied while Afterglow runs, including after compositor reloads (within about 15 seconds). Shortcut editing does not rewrite your Hyprland configuration. Hyprland 0.57.1 uses Lua bindings through `hyprctl eval`; older Hyprlang sessions use `keyword bind`. For other desktops, the tab shows commands to assign in the desktop's shortcut settings:

```bash
afterglow --save-replay
afterglow --toggle-replay
afterglow --show
```

The **Clip storage** control at the bottom-left opens **Settings → Storage**. Choose a preset, custom capacity, or unlimited storage (the default). This is a total library budget. The next save reserves estimated video/audio space and pauses when it would exceed the budget. Existing clips are kept; there is no automatic deletion. A clip already being written can finish above the budget, so this is not a filesystem quota. Exports are limited to the remaining budget and an incomplete failed export is removed while its original is preserved.

**Settings → Appearance & behavior → Start with your desktop** enables login startup. **Open Afterglow window at login** chooses between a visible app window and starting quietly in the tray. The default is off for login startup; enabling it starts quietly unless the window option is also enabled. This runs after desktop sign-in, when screen capture is available. Replay still follows its separate “Start replay when Afterglow opens” setting.

Login setup writes an owned XDG autostart entry under `$XDG_CONFIG_HOME/autostart/dev.afterglow.desktop.desktop`. Managed graphical sessions refresh their systemd user configuration. Standalone Hyprland sessions use a marked startup block in the existing standard `hyprland.lua` or `hyprland.conf`; disabling removes the owned entry/block while preserving user content. Custom compositor config locations need a launcher for `afterglow --autostart`. Duplicate login requests do not open a hidden window.

A persistent capture worker keeps replay independent of the GUI and idle background task pools. Closing the window now keeps Afterglow in the tray by default, with the library web process released to save RAM. Replay and configured shortcuts keep working. The white monochrome tray menu offers Open Afterglow, Start replay, Save replay, Stop replay and Quit Afterglow. Reopen through the tray, shortcut, or app launcher. **Quit Afterglow** fully exits and stops capture. **Settings → Appearance & behavior** controls close-to-tray, replay on app startup, start-in-tray, and Caelestia/light/dark appearance. Replay must be active to save clips. The manual tray preference and desktop-login window preference are separate.

Each clip has **Play**, **Edit**, **Move to folder**, **Open externally**, and **Delete** actions. Delete asks for confirmation and permanently removes only the selected file. The player has explicit play/pause, ±5 seconds, seek, mute and volume controls, retry, and external opening. **Close clip** closes only its preview/editor. App window controls in the main header provide minimize to tray, fullscreen/restore and close; close follows the tray preference. Use **F11** to toggle app fullscreen and **Escape** to leave it. In the player, **Enlarge** fills the window and enters fullscreen; **Shrink** or Escape restores the player without closing the clip.

The app keeps WebKit’s platform rendering policy. The forced software GUI policy introduced in 0.2.1 was removed after reproducing a blank Wayland window. Preview playback uses software video presentation and prefers software H.264 decoding in WebKit. The package explicitly includes gst-plugins-base for the Opus decoder used by compatible previews, plus the H.264 parser plugin from gst-plugins-bad-libs. Play is available as soon as a media source is assigned, including while WebKit has loaded only metadata. If WebKit reports a network, unsupported-media or decode failure, Afterglow prepares a private temporary VP8/Opus preview, up to 720p30. **Compatible preview** can also be selected manually. The original MP4 is kept unchanged and remains the source for edits. Compatible previews play from a local Blob delivered through the native bridge, bypassing HTTP streaming. Conversion is limited to 32 MiB output, 180 seconds and the existing sampled processing RAM guard, and cancels when the editor closes. The binary transfer has its own 32 MiB read limit. Playback starts independently of the duration probe. The temporary preview is removed when closed and stale preview files are cleared on startup. Failure details remain visible in the player and recent playback diagnostics appear in `afterglow --doctor`.

Use the selection toggle on each library clip, or **Select visible**, then **Move to folder** or **Delete selected**. Bulk deletion lists the selected filenames and requires confirmation. Cancel keeps files. Partial failures show the files that could not be deleted and let you retry them; unselected clips are preserved. **Clear selection** resets the selection. Up to 500 clips can be selected per operation.

**Clip library → New folder** creates an in-app folder. Use the folder icon on a clip to move it between folders or choose **No folder**. Folder tiles filter the library; selected folders can be renamed or removed. Removing a folder keeps all its clips in All clips. This is local organization metadata: recordings stay in the same directory on disk. Renaming a clip keeps its folder and trimmed copies inherit the original’s folder.

The clip library also supports search, sort, rename, open-folder, and exporting a trimmed copy while preserving the original. **Fast** trim copies the encoded streams, keeping quality and using little processing memory; cuts can include extra time near a keyframe. Choose **Frame-accurate** for an exact cut using CPU encoding. Preview is unloaded during export. If export fails, the editor retains your range and displays the error so you can retry.

## Validation and limits

44 backend tests and 11 JavaScript tests and native UI/playback/trim/theme/background tests passed with a simulated recorder. An earlier 0.1.8 packaged test measured approximately 437 MB with the library open and 247 MB in the tray while the fixture held a resident 90 MB buffer. These figures are not a GTX 1660 Super recording benchmark or a universal RAM ceiling. Real NVIDIA/Wayland capture, permissions, audio sync, and game performance still require hardware testing.

The RAM target is below 1 GB, with a bounded encoded replay buffer, paused preview during export, and a sampled 384 MiB processing memory guard. High memory no longer automatically closes the window. Use **Game mode** or close to tray to release the library web process while recording continues. GPU VRAM, all filesystem cache, and external shared desktop/kernel allocations are outside that readout. See VALIDATION.md and PACKAGING-VALIDATION.md for the test boundaries.

Clips and preferences stay local. The selected display includes other windows and notifications. Nothing is uploaded by the app; playback uses a token-protected loopback stream.

## Security

The security protections introduced in 0.1.3 include private clip/log directories, safer file-descriptor access, restricted video processing, root refusal, a tighter app permission policy, and a GLib safety fix. See [SECURITY.md](SECURITY.md) for the implemented boundaries, dependency review, and remaining work.

## Troubleshoot

Version 0.1.2 fixed a package bug that made the system desktop entry readable only by root. Existing installations can update with `paru -Syu afterglow-bin` or `yay -Syu afterglow-bin`. Close and reopen the launcher after updating. If it is still missing, run `afterglow --doctor` to inspect launcher paths, permissions, and a possible user override. Do not run the app with sudo.

After upgrading, choose **Quit Afterglow** in the tray and reopen; closing the window keeps the old backend alive. Verify `afterglow --version` shows 0.2.2. Read Settings → Diagnostics → Capture log after a failed start or export. `afterglow --doctor` includes both the installed executable version and running backend version, recorder/FFmpeg/driver details, and recent app/capture logs. Playback errors include the media server request count, last status, origin and reason, without the secret URL token. Compatible-preview completion is logged, and playback diagnostics distinguish the original stream from the local preview. Codec checks load the full playback factories, including MP4 demuxing, AAC/Opus, conversion and audio output. Capture failures are also shown beside the replay controls. Check `nvidia-smi`, `gpu-screen-recorder --info`, and `gpu-screen-recorder --list-capture-options`. Follow the recorder package's KMS permission instructions and run Afterglow as your normal user. A clip must finish writing before FFprobe can read it for trimming.

## Build from source

```bash
sudo pacman -S --needed base-devel rust nodejs npm pkgconf webkit2gtk-4.1 libayatana-appindicator openssl librsvg
npm ci
npm run tauri dev
```

Hyprland shortcut setup uses the hyprctl already provided by Hyprland. It is optional for users assigning commands through another desktop.

Build a native executable with `npm run tauri build -- --no-bundle`. Run backend/media tests with `cargo test --manifest-path src-tauri/Cargo.toml -- --include-ignored` (FFmpeg with libx264/FFprobe required).

## Source, packages, and license

Application source is in this branch. Signed package files are in the [packages branch](https://github.com/AeoryUX/Afterglow/tree/packages); versioned native archives and corresponding source are under its `releases/` directory. See the [0.2.2 release](https://github.com/AeoryUX/Afterglow/releases/tag/v0.2.2) and [repository information](https://github.com/AeoryUX/Afterglow/releases/tag/arch-repository).

Afterglow's original code is GPL-3.0-only. See COPYING and LICENSE.md. Dependencies keep their respective licenses; notices are included in THIRD_PARTY_NOTICES.txt and package distributions.

Saved X11/Wayland capture preferences that conflict with the current desktop are reset to Auto on launch. Explicit incompatible choices are disabled in the session selector; backend validation still rejects incompatible manual requests. A regression test verifies the saved-setting repair.
