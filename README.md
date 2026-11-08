# Afterglow signed pacman repository

Stable server: https://raw.githubusercontent.com/AeoryUX/Afterglow/packages

One-time setup (review the downloaded script first):

```bash
curl -fL https://raw.githubusercontent.com/AeoryUX/Afterglow/packages/setup-afterglow-repo.sh -o setup-afterglow-repo.sh
sudo bash setup-afterglow-repo.sh
paru -S afterglow-bin
```

This is a signed pacman repository, separate from the AUR. No AUR account is needed. The setup pins the public signing key, verifies the database, locally trusts the publisher key, creates /etc/pacman.d/afterglow.conf with SigLevel=Required, adds one Include line to pacman.conf, and syncs only this repository. It does not install a kernel module or upgrade other repository databases.

Publisher fingerprint: 0BD7BEF64831B7CD5EA167C5952EE3DFB307F111

GPL source: https://github.com/AeoryUX/Afterglow

Versioned native archives and corresponding source are under releases/. Database and package signatures are provided. SHA256SUMS supplies additional file-integrity checks. The private signing key is excluded from this repository and all downloads.

Afterglow is an alpha. Real NVIDIA/Wayland capture and game performance still need hardware validation; cloud UI/RAM tests used a simulated recorder. A working NVIDIA kernel driver remains part of the existing system setup.
