# Afterglow security

Version 0.2.2 retains the 0.1.3 protections and adds an owner-only local control channel for background save/toggle/show actions. The socket is mode 0600 inside a private runtime directory, and both ends validate SO_PEERCRED. New saves use the same storage-budget checks from the UI, tray and shortcuts. Legacy --save-replay pidfd fallback remains available for an older running version.

Preferences are stored atomically as mode 0600 in the app configuration directory. Shortcut inputs are validated against allowed modifiers and key names; the launched command is the shell-quoted current executable with a fixed action flag. Hyprland bindings are checked for collisions. Runtime binding changes use either the legacy keyword API or Lua API; Lua bindings retain their own handles so removal targets only Afterglow bindings. No general shell-execution API is exposed to the frontend.

Storage capacity is a save budget, not a kernel quota. It does not delete existing clips. In-flight recorder output can finish above the budget; trimmed exports have an additional output-size limit based on remaining capacity.


## Version 0.1.3 protections

Afterglow runs as the desktop user and refuses effective UID 0. Capture privileges remain with GPU Screen Recorder's supported helper. Afterglow never starts the graphical app with sudo.

The app applies umask 077 before creating threads or subprocesses. Its clip and log directories are owned by the user and mode 0700; new files are mode 0600. Existing clip files are protected by the enclosing private directory. The public desktop launcher remains mode 0644 so launchers can discover it. Files are not encrypted.

Clip reads open regular files with O_NOFOLLOW and O_NONBLOCK relative to an open directory descriptor. Playback and video tools use those descriptors, rather than checking a path and reopening it later. Export temporary files use cryptographically random names and O_EXCL, then rename without overwriting existing clips. Log opening rejects symlinks, hard links, and files owned by another user. Recorder PID writes replace the state file atomically; the save command uses a Linux pidfd to avoid PID reuse between checking and signaling. The shortcut helper delegates to that same command.

FFmpeg and FFprobe run from /usr/bin with a cleared environment, explicit local-file protocols and the MOV/MP4 demuxer. A Linux x86_64 seccomp filter denies network socket operations, process creation (threads remain allowed), ptrace/process-memory access and io_uring setup. PR_SET_NO_NEW_PRIVS prevents privilege elevation. Processing is refused if these restrictions cannot be applied. Jobs die when Afterglow exits.

Only one probe/export/preview conversion job runs at a time. Probe timeout is 10 seconds; export timeout and CPU limit are 10 minutes. Each tool has a sampled 384 MiB resident-memory guard (PSS when available, RSS fallback), a 1 GiB output-file limit, disabled core dumps, and bounded captured diagnostic output. The monitor checks about every 100 ms and terminates only the processing child on excess; transient growth between samples is possible. This replaces the 2 GiB virtual-address cap, which could reject legitimate FFmpeg library mappings. Fast trim copies streams; frame-accurate trim uses two encoding threads and low-latency settings. The original is preserved and incomplete output removed on failure. These are processing limits, not a promise that total resident RAM always stays below 1 GB. No persistent extra service is introduced.

The Tauri capability grants only event listen/unlisten and the enumerated application commands to the local main window. External page navigation is denied. CSP restricts scripts to the bundled app and disables objects, base URLs, form submissions, and framing. The development server binds only to loopback.

Clip playback binds only to 127.0.0.1 and requires a random 256-bit per-launch token. Requests must use the exact loopback Host; ordinary foreign web Origin headers and duplicate Origin headers are rejected. Authenticated requests accept the app’s known custom-scheme origins and the opaque `null` origin emitted by some WebKit builds. A `null` origin alone does not authorize access: the exact Host and unpredictable token are required first. Clip path/name validation and no-follow file opening apply to streaming. Responses disable caching and content-type sniffing. The media endpoint does not offer upload or arbitrary filesystem APIs.

Keep system WebKitGTK, FFmpeg, GPU Screen Recorder and NVIDIA drivers current through normal system updates. App dependency scans do not audit those system packages.

Packages and repository databases are signed. The initial setup verifies the publisher fingerprint and database signature before adding repository configuration. Treat the downloaded setup script as trusted installation code and review it before running with sudo. The signing key is excluded from source, package downloads, and CI.

## Dependency review

The GLib Rust 0.18.5 used by the GTK3 stack has a two-line backport of RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g. Source, original license, and upstream references are in src-tauri/vendor/glib/AFTERGLOW-PATCH.md. The audit verifies the reviewed file's checksum, and an optimized regression test exercises the affected iterator.

Four maintenance warnings remain: proc-macro-error, zbus_names, zvariant, and zvariant_derive. These transitive components need upstream migrations. They are informational unmaintained-crate notices, not four newly identified exploitable vulnerabilities. The exact locked versions, reasons, and a review expiry are recorded in dependency-review.json. They remain visible in audit output.

Run `npm audit --audit-level=low` and `python3 scripts/audit-cargo.py`. The latter sends public dependency names and versions to OSV. CI runs both on main-branch pushes, pull requests, and weekly; new advisories, invalid backport checksums, expired reviews, or scan errors fail the checks. The checkout action is pinned to a verified commit, CI has read-only permissions, and checkout does not persist GitHub credentials. Dependabot checks npm/Cargo updates weekly.

## Remaining boundaries

The media-tool restrictions are not a full filesystem sandbox. A compromised native decoder could still access files readable by the same Linux user. Landlock is unavailable and nested namespace setup is blocked in this build environment; a verified filesystem sandbox remains follow-up work. Kernel restrictions do not eliminate decoder bugs or kernel vulnerabilities.

The localhost server limits concurrent served requests, but does not yet impose hard limits/timeouts on all connections before HTTP parsing. Same-account malicious programs can already read that account's clips, memory, or token. Root can access them too. The current changes primarily reduce accidental disclosure, unsafe path handling, and the impact of untrusted input.

Cloud tests use a simulated recorder. Actual NVIDIA/Wayland capture, privileged helper behavior, audio, and performance require testing on the target desktop. This document is a description of implemented protections and limits, not a security certification.

## Reporting

Use the repository's private vulnerability reporting feature for sensitive findings when available. Do not attach personal clips, credentials, or private signing material to public issues.

## Login startup in 0.1.9

Login startup is opt-in. The app writes only its owned XDG login entry and, for standalone Hyprland sessions, a marked startup block in an existing standard config. A user-owned config symlink is resolved to its existing target while preserving the symlink. Files must be regular, owned by the current user, bounded in size and not hard linked. Writes are atomic; multi-file changes roll back earlier changes if a later write fails. A foreign login entry using the same filename is preserved and reported. Disabling removes the app's registration and preserves foreign config content. The fixed autostart command uses the current executable; no arbitrary command comes from the frontend. No system/root service is installed.

Managed graphical sessions use the XDG entry and a systemd user daemon-reload to refresh autostart generation. Hidden and visible login launches share the existing single-instance backend. Stale login requests exit when startup is disabled, and duplicate login requests do not force a hidden window open.

## Playback, deletion and window actions in 0.1.9

Compatible previews use the same descriptor-based, network-restricted FFmpeg processing as exports, with a dedicated private cache, a 32 MiB output cap, a 180-second timeout and a cancellation generation checked by the child monitor. In 0.2.1, the player reads the completed, private preview through a narrow binary IPC command and creates a local video/webm Blob. The read accepts only the fixed preview filename, rejects symlinks and bounds metadata/read length to 32 MiB before transfer. Blob URLs are allowed only by media-src and revoked on close/reset/replacement. The existing token-protected preview endpoint remains available internally, with WebM MIME and byte ranges. CORS headers are returned only after the token/Host/origin checks, including the authenticated opaque-origin compatibility case described above. Closing the editor cancels conversion and removes its completed cache file; old preview leftovers are cleared on startup. Original clip bytes remain unchanged.

Delete accepts a validated library filename, verifies a regular non-symlink file and unlinks it relative to the opened library directory. The UI confirms the filename before permanent deletion; Cancel does not call the native delete command. External opening accepts only an existing validated library clip and invokes xdg-open without a shell. The window command uses a fixed action enum and targets only the app's own main window; broad window or filesystem permissions are not granted. Playback diagnostic input is bounded before logging.

## Local folders in 0.2.0

Folder metadata is a private, atomically written configuration file (folders.json, mode 0600). Folder names and clip filenames are validated, folder IDs are generated from random bytes, duplicate names are rejected, and counts/serialized size are bounded. File access continues through the existing validated clip API; folder labels never become filesystem paths. Removing a folder only changes this metadata. A clip rename rolls back if saving its new assignment fails. Corrupt metadata is preserved and reported, rather than silently overwritten; capture and playback remain available. Exports keep their output if folder assignment fails and log the warning.

Playback diagnostics report bounded HTTP request metadata without the token or full stream URL. The same-user control socket now exposes the running app version to --doctor. Closing a clip has a dedicated action and no app-window controls inside its dialog.

## Selection and fullscreen in 0.2.1

Selections contain 1–500 distinct, validated clip filenames. An invalid selection is rejected before mutation. Bulk folder moves check all clips exist before saving one atomic index update. Bulk deletion calls the existing regular-file/no-follow deletion logic for each selected name and returns explicit success/failure lists; it preserves unselected clips and never reports a partial failure as full success. The UI confirms filenames before deletion and retries only failed items.

Fullscreen and windowed actions are fixed enum values applied only to Afterglow’s own window. Player enlargement remembers whether it entered fullscreen and restores that state when shrunk or closed. The player uses explicit app controls rather than WebKit’s native fullscreen button. The 0.2.1 forced software WebKit composition policy was reverted in 0.2.2 after reproducing a blank Wayland window. GUI composition follows WebKit’s platform policy; the existing software media presentation/decoder preferences and GPU recorder are retained. Editor activation/deactivation is serialized to prevent rapid close/reopen from cancelling a new preview.
