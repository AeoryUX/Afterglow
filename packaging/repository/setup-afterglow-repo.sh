#!/usr/bin/env bash
# One-time setup for the Afterglow signed pacman repository.
# Review this script and the expected fingerprint before running with sudo.
set -euo pipefail
[[ $EUID == 0 ]] || { echo 'Run: sudo bash setup-afterglow-repo.sh' >&2; exit 1; }
[[ $(uname -m) == x86_64 && -f /etc/pacman.conf ]] || { echo 'This repository targets x86_64 Arch/CachyOS.' >&2; exit 1; }
for command in curl gpg gpgv pacman pacman-key install grep; do command -v "$command" >/dev/null || { echo "Missing command: $command" >&2; exit 1; }; done
server='https://github.com/AeoryUX/afterglow/releases/download/arch-repository'
expected_fingerprint='0BD7BEF64831B7CD5EA167C5952EE3DFB307F111'
stage="$(mktemp -d)"
trap 'rm -rf -- "$stage"' EXIT
mkdir -m700 "$stage/gnupg"
for file in afterglow-signing-key.asc afterglow.db afterglow.db.sig; do
    curl --fail --location --proto '=https' --tlsv1.2 --output "$stage/$file" "$server/$file"
done
actual_fingerprint="$(gpg --batch --homedir "$stage/gnupg" --with-colons --show-keys "$stage/afterglow-signing-key.asc" | awk -F: '$1=="fpr" {print $10;exit}')"
key_count="$(gpg --batch --homedir "$stage/gnupg" --with-colons --show-keys "$stage/afterglow-signing-key.asc" | awk -F: '$1=="pub" {n++} END {print n+0}')"
[[ "$key_count" == 1 ]] || { echo 'Expected exactly one publisher public key.' >&2; exit 1; }
[[ "$actual_fingerprint" == "$expected_fingerprint" ]] || { echo 'Signing key fingerprint mismatch. No repository changes made.' >&2; exit 1; }
gpg --batch --homedir "$stage/gnupg" --dearmor --output "$stage/public-key.gpg" "$stage/afterglow-signing-key.asc"
gpgv --homedir "$stage/gnupg" --keyring "$stage/public-key.gpg" "$stage/afterglow.db.sig" "$stage/afterglow.db"
cat > "$stage/afterglow.conf" <<CONFIG
[afterglow]
SigLevel = Required
Server = $server
CONFIG
if grep -Eq '^[[:space:]]*\[afterglow\][[:space:]]*$' /etc/pacman.conf; then
    echo 'An afterglow block already exists in pacman.conf. Review it before using this setup.' >&2; exit 1
fi
if [[ -f /etc/pacman.d/afterglow.conf ]] && ! cmp -s "$stage/afterglow.conf" /etc/pacman.d/afterglow.conf; then
    echo 'Existing /etc/pacman.d/afterglow.conf differs. Review it before replacing it.' >&2; exit 1
fi
pacman-key --add "$stage/afterglow-signing-key.asc"
pacman-key --lsign-key "$expected_fingerprint"
install -Dm644 "$stage/afterglow.conf" /etc/pacman.d/afterglow.conf
if ! grep -Eq '^Include[[:space:]]*=[[:space:]]*/etc/pacman.d/afterglow.conf[[:space:]]*$' /etc/pacman.conf; then
    printf '\n# Afterglow signed repository\nInclude = /etc/pacman.d/afterglow.conf\n' >> /etc/pacman.conf
fi
# Sync only this new repository; leave the other repository databases unchanged.
cat > "$stage/pacman.conf" <<CONFIG
[options]
Architecture = auto
Include = /etc/pacman.d/afterglow.conf
CONFIG
pacman --config "$stage/pacman.conf" -Sy
printf '\nRepository configured. Install with: paru -S afterglow-bin\n'
