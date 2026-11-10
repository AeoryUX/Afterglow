# Afterglow signed pacman repository

Stable server: https://raw.githubusercontent.com/AeoryUX/Afterglow/packages

One-time setup (review the downloaded script first):

```bash
curl -fL https://raw.githubusercontent.com/AeoryUX/Afterglow/packages/setup-afterglow-repo.sh -o setup-afterglow-repo.sh
sudo bash setup-afterglow-repo.sh --install
```

This is a signed pacman repository, separate from the AUR. No AUR account is needed. The setup pins the public signing key, verifies the database, locally trusts the publisher key, creates /etc/pacman.d/afterglow.conf with SigLevel=Required, adds one Include line to pacman.conf, and syncs only this repository. It does not install a kernel module or upgrade other repository databases.

Publisher fingerprint: 0BD7BEF64831B7CD5EA167C5952EE3DFB307F111

GPL source: https://github.com/AeoryUX/Afterglow

Versioned native archives and corresponding source are under releases/. Database and package signatures are provided. SHA256SUMS supplies additional file-integrity checks. The private signing key is excluded from this repository and all downloads.

Afterglow is an alpha. Real NVIDIA/Wayland capture and game performance still need hardware validation; cloud UI/RAM tests used a simulated recorder. A working NVIDIA kernel driver remains part of the existing system setup.

After initial setup, both `yay -S afterglow-bin` and `paru -S afterglow-bin` work. Use --setup-only to add the repository without installing.

Version 0.1.2 fixes launcher file permissions and Caelestia theme/update handling. Existing users can update with `paru -Syu afterglow-bin` or `yay -Syu afterglow-bin`. Close and reopen the launcher after updating.

Version 0.1.3 adds private files, restricted video processing, tighter native permissions, safer save signaling and a backported GLib fix. See https://github.com/AeoryUX/Afterglow/blob/main/SECURITY.md for protections and remaining boundaries.

Version 0.1.4 adds Settings tabs and an easy Hyprland keybind editor, total clip-library storage budgets, session/resolution/cursor controls, and a white tray icon. Closing keeps replay alive by default. Use Quit Afterglow in the tray, then reopen after upgrading. Details: https://github.com/AeoryUX/Afterglow/releases/tag/v0.1.4

Version 0.1.5 starts replay automatically, makes fast trimming the default, preserves the editor after failed exports, and keeps memory pressure from closing the window. Capture failures and export diagnostics are visible. Update with `paru -Syu afterglow-bin`, choose Quit Afterglow in the tray, then reopen. Details: https://github.com/AeoryUX/Afterglow/releases/tag/v0.1.5

Version 0.1.6 adds shortcut dropdowns, fixes WebKit key capture, keeps capture controls usable after library errors, and repairs saved X11/Wayland settings that conflict with the current desktop. Quit from the tray after upgrading and reopen. https://github.com/AeoryUX/Afterglow/releases/tag/v0.1.6

Version 0.1.7 separates Home and the clip library, highlights active Keybinds, adds a saved-clip sound with mute, supports single keys including F2 and slash, and refreshes the interface with Caelestia surfaces and glowing switches. Quit from the tray after upgrading and reopen. https://github.com/AeoryUX/Afterglow/releases/tag/v0.1.7


Version 0.1.8 adds desktop login startup and a window option, more built-in themes, automatic shortcut selection and Home sync, and compatibility with Hyprland 0.57.1 Lua bindings. Quit from the tray after upgrading and reopen. https://github.com/AeoryUX/Afterglow/releases/tag/v0.1.8

Version 0.1.9 adds explicit player controls, software video presentation, a compatible temporary preview, per-clip Play/Edit/Open/Delete actions, and app window buttons. Quit from the tray after upgrading and reopen. https://github.com/AeoryUX/Afterglow/releases/tag/v0.1.9

Version 0.2.0 fixes clip closing, improves metadata-only playback and media-origin compatibility, adds local clip folders, and expands playback/running-version diagnostics. Quit from the tray after upgrading and reopen. https://github.com/AeoryUX/Afterglow/releases/tag/v0.2.0
