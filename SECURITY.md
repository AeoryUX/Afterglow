# Afterglow security

Version 0.1.4 retains the 0.1.3 protections and adds an owner-only local control channel for background save/toggle/show actions. The socket is mode 0600 inside a private runtime directory, and both ends validate SO_PEERCRED. New saves use the same storage-budget checks from the UI, tray and shortcuts. Legacy --save-replay pidfd fallback remains available for an older running version.

Preferences are stored atomically as mode 0600 in the app configuration directory. Shortcut inputs are validated against allowed modifiers and key names; the launched command is the shell-quoted current executable with a fixed action flag. Hyprland bindings are checked for collisions and its configuration files are not modified. No general shell-execution API is exposed to the frontend.

Storage capacity is a save budget, not a kernel quota. It does not delete existing clips. In-flight recorder output can finish above the budget; trimmed exports have an additional output-size limit based on remaining capacity.


## Version 0.1.3 protections

Afterglow runs as the desktop user and refuses effective UID 0. Capture privileges remain with GPU Screen Recorder's supported helper. Afterglow never starts the graphical app with sudo.

The app applies umask 077 before creating threads or subprocesses. Its clip and log directories are owned by the user and mode 0700; new files are mode 0600. Existing clip files are protected by the enclosing private directory. The public desktop launcher remains mode 0644 so launchers can discover it. Files are not encrypted.

Clip reads open regular files with O_NOFOLLOW and O_NONBLOCK relative to an open directory descriptor. Playback and video tools use those descriptors, rather than checking a path and reopening it later. Export temporary files use cryptographically random names and O_EXCL, then rename without overwriting existing clips. Log opening rejects symlinks, hard links, and files owned by another user. Recorder PID writes replace the state file atomically; the save command uses a Linux pidfd to avoid PID reuse between checking and signaling. The shortcut helper delegates to that same command.

FFmpeg and FFprobe run from /usr/bin with a cleared environment, explicit local-file protocols and the MOV/MP4 demuxer. A Linux x86_64 seccomp filter denies network socket operations, process creation (threads remain allowed), ptrace/process-memory access and io_uring setup. PR_SET_NO_NEW_PRIVS prevents privilege elevation. Processing is refused if these restrictions cannot be applied. Jobs die when Afterglow exits.

Only one probe/export job runs at a time. Probe timeout is 10 seconds; export timeout and CPU limit are 10 minutes. Each tool has a 2 GiB virtual address limit, a 1 GiB output-file limit, disabled core dumps, and bounded captured diagnostic output. These are processing limits, not a promise that total resident RAM always stays below 1 GB. No persistent extra service is introduced.

The Tauri capability grants only event listen/unlisten and the enumerated application commands to the local main window. External page navigation is denied. CSP restricts scripts to the bundled app and disables objects, base URLs, form submissions, and framing. The development server binds only to loopback.

Clip playback binds only to 127.0.0.1 and requires a random 256-bit per-launch token. Requests must use the exact loopback Host; supplied foreign Origin headers are rejected. Clip path/name validation and no-follow file opening apply to streaming. Responses disable caching and content-type sniffing. The media endpoint does not offer upload or arbitrary filesystem APIs.

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
