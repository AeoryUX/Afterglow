# Afterglow

An experimental NVIDIA-first Linux gameplay clipping app for CachyOS and Hyprland, with Caelestia colour syncing.

Afterglow uses GPU Screen Recorder for hardware-encoded instant replay and FFmpeg for clip edits. It includes a local clip library, a replay hotkey, low-memory game mode, a tray menu, and optional Caelestia appearance syncing.

## Distribution status

The first public release and AUR submission are being prepared. **`paru -S afterglow-bin` is not available yet.** Once the AUR package is published, paru will install the declared application dependencies automatically.

A working NVIDIA kernel driver matching your CachyOS kernel is required. The package will depend on NVIDIA user-space tools but will not choose a kernel module for your system.

## Validation

The Linux executable and AppImage have passed native UI, playback, trimming, theme-sync, and background-mode tests using a simulated recorder. Actual NVIDIA/Wayland capture, audio sync, and gaming performance still require hardware testing. RAM figures from the simulated tests are not a GPU recording benchmark or a universal memory ceiling.

## License

The application's release license is being selected. No open-source license is currently granted for the application. Third-party components retain their respective licenses.
