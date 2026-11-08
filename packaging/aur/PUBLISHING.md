# Publishing the one-command installer

The prepared package is `afterglow-bin` 0.1.3-1, licensed GPL-3.0-only. After publication, users can run:

```bash
paru -S afterglow-bin
```

Paru reads .SRCINFO/PKGBUILD, downloads the pinned prebuilt native release, checks its SHA-256, and installs the declared app dependencies through pacman. GPU Screen Recorder is currently in Arch Extra. The recipe also requires FFmpeg, GTK3, WebKitGTK 4.1, Ayatana tray support, GStreamer playback plugins, xdg-utils, and nvidia-utils. A working NVIDIA kernel driver matching the system's kernel remains necessary.

## Current blockers

- There is no `afterglow-bin` AUR entry and no connected AUR SSH identity.
- The user does not yet have an AUR maintainer account, and new AUR registration is temporarily closed. Do not automate retries. Use the signed repository alternative in ../repository/HOSTING.md if installation is needed before registration reopens.
- GitHub source and signed package hosting are now live. The signed pacman repository is the active installation route while AUR registration remains closed.

## Required account setup

1. Create an AUR account at https://aur.archlinux.org/register/ and verify the email. Add the public part of an SSH key to the account. Keep the private key private; do not paste it into chat.
2. The public GitHub repository is already available at https://github.com/AeoryUX/Afterglow. The version-pinned native archive URL below matches the AUR recipe.

## Host the release first

Publish the prepared source under tag `v0.1.3` in AeoryUX/Afterglow. Host the exact `afterglow-0.1.3-linux-x86_64.tar.gz` used by this recipe, its checksum, and `afterglow-source.zip` as corresponding source in the packages branch under the immutable version tag. The release archive includes COPYING, LICENSE.md, and third-party notices. Do not replace the asset later without updating the AUR checksum and pkgrel.

The public source URL expected by this recipe is:

```
https://raw.githubusercontent.com/AeoryUX/Afterglow/v0.1.3-packages/releases/v0.1.3/afterglow-0.1.3-linux-x86_64.tar.gz
```

## Submit the package

Review PKGBUILD. From an Arch/CachyOS machine with git, base-devel/makepkg and your AUR SSH key configured, run:

```bash
./publish.sh YOUR_AUR_USERNAME
```

The script checks the public asset hash, clones the AUR repository, refuses to overwrite an existing package history, generates .SRCINFO with makepkg, commits, and pushes. Your normal git author identity must be configured. SSH host-key verification stays enabled.

After pushing, confirm the AUR entry and test `paru -S afterglow-bin` on CachyOS. Do not advertise that command as working before the AUR entry exists.
