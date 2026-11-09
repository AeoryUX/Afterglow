# Distribution packaging validation

## Version 0.1.8 (2026-11-09)

The signed package is afterglow-bin-0.1.8-1-x86_64.pkg.tar.zst. The executable is built from the committed source with the vendored GLib safety fix and compiled static interface. Corresponding source includes the patched crate, original license, patch notes, audit script, and dependency review.

The native executable embeds the white tray icon. The clip-saved sound adds pipewire-audio for pw-play; the audio asset is embedded in the native executable. Hyprland supplies its optional hyprctl command.

The launcher remains mode 0644 with the same application identity. Application-created clips, logs, PID state and temporary exports are restricted separately; launcher discovery continues to work. The package also includes SECURITY.md under /usr/share/doc/afterglow.

Both native and Arch archive generation are exercised under umask 077. The actual package is extracted with original permissions and checked for launcher/icon/directory/executable modes, valid desktop metadata, desktop-file-utils dependency, and the security document. Pacman/libalpm reads package and repository metadata. Package/database signatures are verified with a separate public-key-only client. The public downloads are compared against the prepared artifacts after publishing.

The signed repository retains previous package files for clients using a cached database. The future AUR recipe uses the immutable v0.1.8-packages snapshot. An AUR entry has not been submitted.

Native WebKit testing uses the existing GTK/WebKit test payload with the new executable. Video processing runs the real host FFmpeg with the new restrictions. Capture and GPU detection are test fixtures, not hardware capture. The previous 0.1.1 AppImage was not rebuilt and does not contain this release's protections; the current update is the signed Arch package and native archive.

Full root installation on CachyOS, physical NVIDIA capture, and a live Caelestia shell are not available in this cloud environment. See VALIDATION.md and SECURITY.md for security and test boundaries.
