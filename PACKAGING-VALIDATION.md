# Distribution packaging validation

## Version 0.1.2 (2026-11-08)

The signed Arch/CachyOS package is `afterglow-bin-0.1.2-1-x86_64.pkg.tar.zst`. The native release archive contains the executable, shortcut helper, user installer, canonical desktop entry, icon, documentation, GPL license, and dependency notices. It targets Linux x86_64 with glibc >= 2.39.

The previous binary package installed a root-owned desktop entry with mode 0600. Quickshell's desktop-entry reader skips unreadable files, so Caelestia could not list it. The updated recipe installs the canonical entry explicitly with mode 0644, validates its contents, and declares desktop-file-utils for desktop database updates. The user installer also sets mode 0644. The filename remains dev.afterglow.desktop.desktop to preserve the existing dev.afterglow.desktop application identity.

Packaging is tested with umask 077, matching the restrictive environment that exposed the bug. `python3 tests/check-package.py PACKAGE` extracts the actual archive and checks desktop-file validation, launcher mode, traversable directory permissions, executable/icon modes, expected paths, and the desktop cache dependency. Pacman/libalpm reads the resulting package metadata. Package and repository signatures are verified with a separate public-key-only client.

The source and new native archive are published with this version in the packages branch. An immutable v0.1.2-packages tag pins the native download used by the future AUR recipe. This is a signed pacman repository; an AUR entry has not been submitted. Use the installer in README once, then `paru -Syu afterglow-bin` or `yay -Syu afterglow-bin` for this update.

## Native test boundary

The new executable was tested in the existing bundled GTK/WebKit test payload under a virtual X display. It exercised real Rust IPC, playback, FFmpeg trimming, theme synchronization, and background lifecycle with a test-only recorder. The payload's private runtime paths were relocated for the build environment. Test executables and private signing material are excluded from downloads.

The earlier 0.1.1 AppImage passed extract-and-run testing. A new 0.1.2 AppImage was not built for this package fix. The current update is distributed through the signed Arch repository and native archive.

Real NVIDIA/Wayland capture, KMS permissions, game performance, physical audio output, and the actual Hyprland shortcut remain hardware acceptance work. The Caelestia/Quickshell launcher source was reviewed, but a live Caelestia desktop was not available here.
