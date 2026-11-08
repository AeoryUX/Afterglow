# Signed pacman repository

AUR registration is temporarily closed. This signed repository is an alternative route to the same installer command. It does not create or bypass an AUR account.

After one-time repository setup, users can run:

```bash
paru -S afterglow-bin
```

## Hosting needed

Create the empty public GitHub repository AeoryUX/afterglow and grant the connected GitHub app access if required. Publish the GPL application source and a versioned source/binary release first. Create a separate release tagged `arch-repository`, then upload every file from `afterglow-arch-repo/` to it. Pacman uses the stable HTTPS download path for that release.

The repository contains afterglow-bin 0.1.1-2, its detached signature, signed pacman database and file database, public signing key/fingerprint, and setup script. The private signing key is retained in the protected work directory and is not in these deliverables. Preserve that private key in maintainer-controlled secure storage before relying on this repository for ongoing releases.

## One-time client setup

Download and review `setup-afterglow-repo.sh`, verify its pinned fingerprint against `afterglow-signing-key.fingerprint`, then run:

```bash
sudo bash setup-afterglow-repo.sh
paru -S afterglow-bin
```

The setup checks the downloaded public key fingerprint and database signature before adding the key to pacman's trust store. It creates /etc/pacman.d/afterglow.conf with `SigLevel = Required`, adds one Include line to pacman.conf, and syncs only the new repository. It does not install a kernel driver or update the other repository databases. Keep the rest of an Arch/CachyOS system up to date through the normal system upgrade workflow.

Review/removal: remove the Include line and /etc/pacman.d/afterglow.conf to stop using this repository. If removing trust as well, use `pacman-key --delete FINGERPRINT` with the fingerprint from the public key file. Removing repository configuration does not remove installed clips or preferences.

## Publication status

The repository files are prepared and checked locally. They are not hosted yet, and the one-time setup URL will not work until the GitHub release assets exist. The GitHub connection rejected repository creation, so the repository owner must create AeoryUX/afterglow or grant suitable creation access first.
