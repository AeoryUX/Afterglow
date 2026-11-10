# Afterglow 0.2.3 alpha validation

Latest validation on 2026-11-10 in a Linux x86_64 cloud environment. The target desktop is CachyOS + Hyprland + Caelestia + NVIDIA GTX 1660 Super.

## Passed

- Vite production build and optimized, stripped Rust/Tauri release build.
- 44 optimized Rust backend/media tests, including real 1080p60 FFmpeg fast and precise trimming, original preservation, filename/path boundaries, HTTP byte ranges, capture settings, process-tree memory, and retaining/recovering a valid palette while a Caelestia scheme is partially written.
- Seven JavaScript preference/key-recorder tests: corrupt/legacy settings recover safe defaults and excessive encoded replay budgets are bounded.
- Browser preview: capture guard, navigation, saved preferences, search, 900px/600px layouts, and no JavaScript errors.
- Browser with a simulated native bridge: start/save/stop, rename, trim, open-folder, delayed monitor detection, forced diagnostic refresh, and Home capture controls without discarding unsaved Settings drafts.
- Native WebKitGTK test under Xvfb: actual compiled interface/Rust IPC, recorder start/save/stop, real MP4 duration/rename/trim/playback (readyState 4), live dark/light Caelestia schemes, partial scheme recovery, and separate Home/library/settings navigation.
- Game mode tears down the web process, preserves recording and saving, and relaunch restores the library. The recorder fixture held a resident 90 MB buffer. Earlier 0.1.8 fixture tree PSS: 436,784,128 bytes with the library and 247,147,520 bytes in game mode. These are fixture results, not a hardware benchmark or a universal ceiling.
- Headless --version, --help, --doctor, and --save-replay; the latter signals an owned recorder and rejects negative PIDs.
- Package regression checks under umask 077, desktop-file validation, pacman metadata, SHA-256 integrity, and package/repository signatures. See PACKAGING-VALIDATION.md.

## Startup and export regression checks

Fresh native startup begins replay without clicking Start. A legacy 0.1.4 preferences file with automatic startup disabled is migrated once; an explicit later opt-out persists. An injected early recorder failure remains visible in the compiled interface and --doctor logs; retry succeeds after removing the fixture error. Capture now requests AAC audio for MP4 compatibility and disables repetitive FPS logging; flags were checked against GPU Screen Recorder 6.1.7's packaged manual.

Native DOM tests click Export trimmed copy while replay runs, confirm a new fast MP4, and keep replay active. A deliberately corrupted scratch source fails without closing the editor or discarding its range. The browser bridge tests the same failure-and-retry flow. The backend suite checks real 1080p60 fast/precise exports, source preservation, quota cleanup, and killing a memory-heavy processing child without terminating the app. Preview is unloaded before exports. Memory pressure no longer destroys the window automatically; explicit Game mode refuses while the editor is open.

## Settings and tray regression checks

The browser flow checks the direct Keybinds and bottom-left Storage navigation, accessible tab keyboard navigation, shortcut recording, storage presets/custom/unlimited controls, saved capture/session/resolution/cursor/behavior choices, and responsive layouts. The mocked native bridge preserves the existing start/save/stop/rename/trim/folder flows and unsaved capture drafts during status polling.

Native WebKit tests use the compiled UI and real Rust IPC. A test-only hyprctl models binding queries and mutations; tests verify applying, repeated application without duplicates, rebinding, and preserving an existing foreign shortcut. A wrong session selection is rejected before capture starts. Private preferences round-trip, valid capacity bounds, and the replay buffer budget have backend tests.

The recorder is spawned by a persistent capture worker so its parent-death signal remains tied to a live thread throughout background recording. A WM_DELETE_WINDOW client message exercises the actual close event under Xvfb. The app and simulated recorder remain alive, the web process is released, the recorder survives 20 seconds with no UI (past async-pool idle retirement), the exact configured shortcut command saves a clip while closed, and --show reopens the library. --toggle-replay stops and restarts capture through the local control channel. A fresh native X11-mode launch verifies saved capture flags, automatic replay, start-in-tray, explicit reopen, and close-to-quit behavior. A storage budget blocks saves before creating files; limited exports fail cleanly without altering the original.

These tests simulate the recorder and Hyprland IPC; they do not establish real compositor hotkey delivery, a visible Caelestia tray host, or NVIDIA capture performance. The white tray asset is verified as transparent RGBA with white RGB pixels. Capture cursor and scale options were checked against the GPU Screen Recorder 6.1.7 package usage.

## Security regression checks

The optimized backend suite includes private file/directory permissions, symlink/hard-link rejection without altering the target, FIFO rejection without blocking, stable inode reads after a filename swap, blocked sockets, blocked process creation, bounded diagnostic output, timeout cleanup, root refusal, and the backported GLib iterator fix. The HTTP tests reject foreign Origin/Host headers, wrong tokens, traversal, and clip symlinks. Real host FFmpeg still exports the correct trim while preserving the source.

Native WebKit tests confirm mode 0700 for the library, mode 0600 for recorded clips, PID state and exports, denied broad native monitor access, blocked external navigation, and normal MP4 playback under the production CSP. The save command still signals an owned recorder and rejects a negative PID after switching to pidfds.

npm audit reports no advisories in the current lockfile. OSV reports the backported GLib issue under two identifiers and four unmaintained transitive-crate warnings; the audit displays the documented exceptions and fails on new/unreviewed issues, expired review, or a changed backport checksum. See SECURITY.md and dependency-review.json. These scans cover the pinned app dependencies, not every system package.

## Limits

The native test substitutes nvidia-smi and GPU Screen Recorder with scratch fixtures. Real NVIDIA encoding, Wayland/KMS capture permissions, microphone/system-audio synchronization, game performance, and the actual Hyprland shortcut require target-machine testing. No physical audio device was tested. A live Caelestia shell was not available; its launcher and Quickshell's desktop-entry loader were reviewed from source.

Native tests use the existing bundled GTK/WebKit payload because the rootless cloud sysroot has relocated helper paths. WebKit's normal /var/tmp media cache was permitted for testing. The native release links the target machine's system libraries. Test fixtures and the private signing key are excluded from source and release archives.

## Build

Rust stable 1.99.1; Tauri 2.12.5; Vite 6.4.5. Release optimization level 1 with stripped symbols. Linux x86_64, glibc >= 2.39, GTK3/WebKitGTK 4.1. Cargo.lock and package-lock.json pin dependency resolution.

## 0.1.9 controls regression

Browser checks simulate an initial library error and confirm Start replay stays available, edit shortcuts using dropdowns, apply them through the native bridge, and retain prior export failure/retry checks. Unit tests cover missing and Unidentified KeyboardEvent.code. Native WebKit checks exercise the dropdowns, startup/failure/retry, export, and tray paths. User diagnostics identified an incompatible saved X11 session blocking startup on Wayland. The startup repair resets it to Auto, and an injected legacy X11 preference in a simulated Wayland session exercises that path. The updated UI includes recorder log details on failure.

Saved X11/Wayland capture preferences that conflict with the current desktop are reset to Auto on launch. Explicit incompatible choices are disabled in the session selector; backend validation still rejects incompatible manual requests. A regression test verifies the saved-setting repair.

## 0.1.9 layout, single keys, and sound

Browser checks cover Home as the initial page, the library containing only clips/tools, search and sort, active sidebar states, single F2 and slash capture/application, export failure/retry, custom glowing switches with keyboard operation, mute persistence, storage controls, and 900px/600px layouts. Screenshots were inspected for the new layout.

Native WebKit checks verify unmodified F2 and slash through a Hyprland IPC fixture, foreign slash-binding conflict preservation, rebinding cleanup, separate Home/library controls, saved-clip WAV delivery, mute, and sound delivery after closing to tray. PipeWire playback is replaced by a fixture that reads and checks the embedded PCM WAV. This proves routing and mute behavior, not physical speaker output. The normal package now requires pipewire-audio; the Arch package file list confirms it supplies pw-play. The self-authored tone is included under the app's GPL license. NVIDIA recording and physical audio still require target-machine checks.

## 0.1.9 login, themes and Hyprland compatibility

37 backend tests and six JavaScript tests passed. New tests cover owned login registration/removal, marked Lua/Hyprlang hooks, idempotency, foreign-file preservation, Lua binding generation and ownership, and complete theme palettes. The new login options default off; existing preferences migrate without enabling login startup.

Browser tests cover automatic key application, captured-key dropdown selection, Home shortcut sync, inline conflict errors, retry, rapid edits and cancellation after removing bindings. Automatic completion is also checked not to discard an open clip editor or unsaved Settings form. They also cover dependency of the login window switch on login startup, saved theme choices and the existing clip/export/storage/keyboard/responsive flows.

Native WebKit tests use the compiled UI and real Rust IPC. The Lua-mode hyprctl fixture reports unknown request for legacy keyword commands and executes generated Lua with the host Lua 5.4 interpreter against a modeled hl binding API. Tests verify idempotency, rebinding, ownership markers, slash/F-key conflicts, confirmed sound routing, clip export and background recording. A second fixture mode exercises the legacy keyword API. API names and behavior were checked against the Hyprland v0.57.1 source, matching the user's version. No real compositor was available.

Login tests validate the actual generated desktop entry, marked standalone Hyprland registration, default hidden login, visible login, duplicate request handling, disable/removal and stale-entry exit. A managed-session fixture verifies XDG registration plus systemd user daemon-reload without adding a compositor hook. These tests simulate desktop login by launching the generated command; a physical machine reboot/sign-in was not performed.

## 0.1.9 playback and clip/window actions

38 optimized backend tests and seven JavaScript tests passed. The real-media test now also creates a 720p VP8/Opus WebM preview from a 1080p60 MP4, verifies its codec/resolution, checks original preservation and cancellation cleanup. A separate deletion test rejects traversal and symlinks, removes only the selected clip, and preserves the other file. Loopback HTTP tests cover preview MIME/ranges and app-origin CORS while retaining foreign Host/Origin rejection.

Browser tests also verify that a failed metadata probe does not block media playback. They use real MP4/WebM fixtures with byte-range responses and verify progressing playback, pause, seeking, skip, mute/volume, fallback after a failed source, visible window buttons, per-clip external opening and delete confirmation/cancel. The existing settings, automatic shortcut updates, editor/export retry and responsive checks also pass.

Native WebKitGTK tests use the final compiled interface. MP4 playback advances and a canvas read verifies decoded frame pixels. A deliberately failed HTTP source triggers automatic compatible preview creation; the real generated WebM then plays with advancing time while the recorder fixture remains active and the original hash is unchanged. Closing the editor cancels an in-flight conversion and removes cache output. Native clip actions route the selected path to an external-opener fixture and confirm deletion while preserving another clip. Minimize-to-tray releases the window, keeps recording alive for 20 seconds, saves again and reopens. Close follows close-to-quit settings. The maximize command dispatches successfully, but Xvfb has no normal desktop window manager, so compositor-specific geometry is not established by this test.

WebKit's software video presentation flag was checked against upstream GStreamer media code. Arch's current gst-plugins-bad-libs file list confirms it supplies libgstvideoparsersbad.so. Hardware-specific NVIDIA/Wayland behavior and the user's particular failed clip were not available for direct reproduction. External player launch and speaker output use fixtures. No test media is embedded in the distributed app.

## 0.2.0 playback, closing, and folders

41 optimized Rust tests and nine JavaScript tests pass. New backend tests cover folder creation/validation, persistence, clip-rename mapping, export inheritance, removal without file deletion, rollback after a failed metadata write, and preservation of corrupt metadata. HTTP tests cover authenticated opaque/alternate app origins, byte ranges/CORS, wrong-token rejection with an opaque Origin, and rejection of duplicate/foreign origins. The metadata-only player tests verify Play can request frames immediately, autoplay is not gated on future-data readiness, and closing unloads the source.

Browser tests exercise folder creation, duplicate-name errors, move/filter, rename and removal that keeps clips, metadata-only Play readiness, and the absence of app-window actions inside the clip dialog. Existing real MP4/WebM playback, seek, mute/volume, fallback, settings, shortcuts, export failure/retry and responsive flows remain covered. A native compiled WebKit test verifies decoded MP4 frames, progressing MP4/WebM playback, conversion cancellation, folder creation/move, clip rename, real FFmpeg export inheritance and folder persistence across backend restart. Closing the clip leaves the main window and recorder alive, including when close-to-tray is disabled. Removing a folder preserves its recorded files.

The user’s 0.1.9 diagnostics showed H.264/VP8/parser/demux plugins available, media code 4 before data loaded, and cancelled previews. The exact user clip and current target WebKit build are unavailable here. The release fixes identified controller/origin compatibility issues and adds token-free media request diagnostics; it does not claim the specific target failure was reproduced. The simulated recorder and host software decoders do not establish physical NVIDIA/Wayland performance.

## 0.2.1 fullscreen, selection, and local preview transport

44 optimized Rust tests and 11 JavaScript tests pass. New backend tests reject invalid/duplicate/oversized selections before deletion, report partial deletion without touching unselected files, preserve atomic folder assignments, and reject symlink/oversized binary preview reads. Controller tests cover Blob creation/revocation and enlargement/Escape restoration.

Browser tests exercise selection toggles, select-visible, folder moves/filtering, delete Cancel/confirmation, injected partial failure and retry, removal of Unfiled, an injected refresh during delayed editor activation, playing media before a delayed duration probe completes, actual MP4/Blob WebM playback, player enlargement/Escape, fullscreen control/F11 dispatch and responsive layouts. Existing settings, storage, automatic shortcuts, export failure/retry and unsaved-editor flows still pass.

Native WebKitGTK tests use the compiled app, actual Rust IPC and production CSP. An Xfwm-managed Xvfb window enters fullscreen with larger actual dimensions and returns to its original size. Player enlargement enters fullscreen, and cancellation shrinks without closing the clip. A deliberately failed original URL triggers real FFmpeg conversion, native binary IPC, local Blob WebM loading and advancing playback; MP4 frame pixels and source hash preservation are also checked. Native selection tests move two clips to a folder, cancel then confirm bulk deletion, and preserve the unselected clip. Real trimmed export and 20-second background recording/saving/reopening are checked. Intentional web-process teardown invalidates the automation page; reopened window visibility is verified through the X window tree and live backend/recorder checks.

The user’s 0.2.0 log shows HTTP 200 responses with the expected app Origin, so rejected media requests are not established as the cause. The new compatible-preview transport bypasses HTTP and the GUI uses software WebKit composition, but the exact user clip and WebKit 2.54/NVIDIA/Hyprland combination remain unavailable for direct reproduction. Arch’s current package file lists were reviewed: playback factories already come from gst-plugins-base-libs, so no speculative extra codec dependency was added. --doctor now loads more actual factories instead of querying only four cached registry entries. Software preview and fixture results do not establish physical NVIDIA recording performance or a universal RAM ceiling.

## 0.2.2 Wayland blank-window regression and Opus packaging

The 0.2.1 binary was tested under a private Weston 14 Wayland compositor with WebKitGTK 2.54. The DOM loaded, but a compositor screenshot contained an unpainted window. This reproduces the blank-window regression at the display level; it was missed by the previous X11/DOM/native snapshot checks. Removing HardwareAccelerationPolicy::Never restores visible Home navigation and controls in the actual Wayland compositor capture. The GUI now uses WebKit’s platform rendering policy; software media presentation/decoder preferences remain unchanged.

The new optional tests/check-wayland-render.py puts a known paint marker in the loaded app and verifies it in a compositor screenshot. It requires an isolated Wayland QA session, WebKitWebDriver, weston-screenshooter with output capture enabled, and Pillow. It checks display pixels rather than trusting DOM readiness or WebKit’s own offscreen snapshot. This native regression check passes with 0.2.2.

The native Wayland smoke test also verifies auto-started simulated replay, MP4 playback progression, real VP8/Opus conversion, binary IPC/Blob playback progression, preservation of the source hash, and closing the clip while capture stays active. 44 optimized Rust tests and 11 JavaScript tests pass; the CLI owned-PID/negative-PID checks remain valid. The package contract test now requires gst-plugins-base as well as the existing decoder/parser packages. Arch’s current file lists establish that gst-plugins-base supplies libgstopus.so and gst-plugins-base-libs does not; the user’s doctor output explicitly reports opusdec missing.

These tests use software Mesa/Weston and a simulated recorder, not a GTX 1660 Super or live Hyprland compositor. The Wayland painting regression is reproduced and fixed here; exact hardware capture performance and the user’s original MP4 remain outside direct reproduction.

## 0.2.3 video overlays, corruption mitigation and fullscreen reconciliation

44 Rust backend tests pass, including the real 1080p60 media case. Compatible output is checked for H.264 Constrained Baseline, 1280×720, YUV420p, zero B-frames and AAC audio. A strict full decode must succeed. Four decoded, downscaled frames at 0.5/1.5/2.5/3.5 seconds are compared against independently decoded original frames; each must stay below 12 mean absolute RGB levels. Invalid input must fail without modifying the source, exposing a completed preview or leaving temporary files. Cancellation and storage/path/private-file tests remain included. HTTP tests also prove that an original named preview.mp4 is not shadowed by the fixed preview route.

15 JavaScript tests pass. Player checks cover metadata-only readiness, local MP4 Blob MIME/lifetime, external fullscreen exit, external/preexisting fullscreen ownership on close, idempotent enter/exit, Escape, rapid repeated enlargement and closing an unfinished request. The review also guards asynchronous Retry results and reflects actual pause/play state.

Browser checks verify every playback/tool button and seek slider is inside the video, external fullscreen changes clear stale enlargement, Escape/F11 behavior, real Blob MP4 playback, and the existing folder/bulk-delete/partial-retry, settings, shortcut, export and responsive flows. Overlay controls remain keyboard-accessible and do not consume modifier chords such as Super+F.

The compiled app is tested on Wayland/Weston with production IPC/CSP. The tests verify actual compositor UI/video pixels and compare four decoded native-preview frames against FFmpeg reference frames. Fullscreen is changed outside the player controller, then the layout must reconcile while the clip/source remain open. Repeated setters are idempotent; closing externally entered fullscreen preserves it; Escape exits player enlargement without closing the clip. Source bytes and background replay remain intact.

The user confirms the original is clean in VLC/mpv, while the screenshot shows the VP8 compatible preview active with corrupted video. The release replaces that path with normalized H.264/AAC. The exact user clip and physical NVIDIA/Hyprland environment are not available here, so this is a targeted format change with decoded-frame validation rather than a claim that the user’s exact corruption was reproduced. Desktop mode changes are exercised through native window requests outside the player, not a real Hyprland Super+F keystroke.
