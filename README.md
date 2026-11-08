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
