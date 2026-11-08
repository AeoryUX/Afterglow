#!/usr/bin/env bash
# Install a prebuilt Afterglow release for the current user. Never run with sudo.
set -euo pipefail
if [[ $EUID == 0 ]]; then echo 'Run as your normal desktop user, without sudo.' >&2; exit 1; fi
release_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
[[ -x "$release_dir/afterglow" ]] || { echo 'Use this installer from an extracted prebuilt release.' >&2; exit 1; }
prefix="${AFTERGLOW_PREFIX:-$HOME/.local}"
install -Dm755 "$release_dir/afterglow" "$prefix/bin/afterglow"
install -Dm755 "$release_dir/afterglow-save" "$prefix/bin/afterglow-save"
install -Dm644 "$release_dir/icon.png" "$prefix/share/icons/hicolor/128x128/apps/dev.afterglow.desktop.png"
mkdir -p "$prefix/share/applications"
cat > "$prefix/share/applications/dev.afterglow.desktop.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Afterglow
Comment=Keep your gameplay highlights on Linux
Exec="$prefix/bin/afterglow"
Icon=$prefix/share/icons/hicolor/128x128/apps/dev.afterglow.desktop.png
Terminal=false
Categories=AudioVideo;Recorder;
GenericName=Gameplay Recorder
Keywords=Afterglow;Clips;Gameplay;Replay;Recording;Video;
StartupWMClass=dev.afterglow.desktop
DESKTOP
chmod 644 "$prefix/share/applications/dev.afterglow.desktop.desktop"
if command -v update-desktop-database >/dev/null; then update-desktop-database "$prefix/share/applications" || true; fi
printf 'Installed Afterglow. Open it from your launcher or run: %s/bin/afterglow\n' "$prefix"
printf 'For replay capture, install GPU Screen Recorder and verify nvidia-smi in your desktop session.\n'
printf 'Hyprland binding: bind = SUPER, F8, exec, %s/bin/afterglow-save\n' "$prefix"
