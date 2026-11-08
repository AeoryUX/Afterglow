#!/usr/bin/env bash
# Run once from a machine with the maintainer's AUR SSH identity configured.
set -euo pipefail
[[ $# == 1 && "$1" =~ ^[A-Za-z0-9_-]+$ ]] || { echo 'Usage: ./publish.sh YOUR_AUR_USERNAME' >&2; exit 1; }
maintainer="$1"
recipe_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$recipe_dir/PKGBUILD"
[[ "${license[0]}" == GPL-3.0-only ]] || { echo 'Unexpected release license.' >&2; exit 1; }
release_url="${source[0]#*::}"
stage="$(mktemp -d)"
trap 'rm -rf -- "$stage"' EXIT
# Do not publish a recipe whose public release is absent or mismatched.
curl --fail --location --proto '=https' --tlsv1.2 --output "$stage/release.tar.gz" "$release_url"
printf '%s  %s\n' "${sha256sums[0]}" "$stage/release.tar.gz" | sha256sum --check -
git clone ssh://aur@aur.archlinux.org/afterglow-bin.git "$stage/aur"
if git -C "$stage/aur" rev-parse --verify HEAD >/dev/null 2>&1; then
    echo 'AUR repository already contains commits. Review its ownership and history before updating.' >&2
    exit 1
fi
sed "s/^# Maintainer:.*/# Maintainer: $maintainer/" "$recipe_dir/PKGBUILD" > "$stage/aur/PKGBUILD"
(cd "$stage/aur" && makepkg --printsrcinfo > .SRCINFO)
git -C "$stage/aur" add PKGBUILD .SRCINFO
git -C "$stage/aur" commit -m 'Add Afterglow binary package and runtime dependencies'
git -C "$stage/aur" push origin HEAD:master
printf 'AUR package pushed. Confirm https://aur.archlinux.org/packages/afterglow-bin before announcing availability.\n'
