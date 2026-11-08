#!/usr/bin/env bash
# Build an alpm-compatible local binary package, including .PKGINFO and .MTREE.
# On Arch you may instead use makepkg with packaging/arch/PKGBUILD.
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="${1:-$project_dir/releases}"
mkdir -p "$out_dir"
out_dir="$(cd -- "$out_dir" && pwd)"
source_archive="$out_dir/afterglow-0.1.2-linux-x86_64.tar.gz"
[[ -f "$source_archive" ]] || { echo 'First run scripts/package-release.sh with the same output directory.' >&2; exit 1; }
for tool in bsdtar zstd tar sha256sum desktop-file-validate; do command -v "$tool" >/dev/null || { echo "Missing packaging tool: $tool" >&2; exit 1; }; done
stage="$(mktemp -d)"
trap 'rm -rf -- "$stage"' EXIT
export srcdir="$stage/source" pkgdir="$stage/package"
mkdir -p "$srcdir" "$pkgdir"
tar -xzf "$source_archive" -C "$srcdir"
recipe="$project_dir/packaging/arch/PKGBUILD"
# Load our tracked local recipe, never an untrusted external PKGBUILD.
source "$recipe"
actual_sha="$(sha256sum "$source_archive" | cut -d' ' -f1)"
[[ "$actual_sha" == "${sha256sums[0]}" ]] || { echo 'Release checksum differs from PKGBUILD. Regenerate the recipe checksum first.' >&2; exit 1; }
package
# Fail before creating a repository artifact if a launcher cannot be read.
[[ $(stat -c %a "$pkgdir/usr/share/applications/dev.afterglow.desktop.desktop") == 644 ]] || { echo 'Desktop entry must be mode 0644.' >&2; exit 1; }
[[ -x "$pkgdir/usr/bin/afterglow" ]] || { echo 'App executable is not executable.' >&2; exit 1; }
installed_size="$(du -sb "$pkgdir" | cut -f1)"
{
    printf 'pkgname = %s\npkgbase = %s\npkgver = %s-%s\npkgdesc = %s\n' "$pkgname" "$pkgname" "$pkgver" "$pkgrel" "$pkgdesc"
    printf 'builddate = %s\npackager = Afterglow local build\nsize = %s\narch = x86_64\n' "$(date +%s)" "$installed_size"
    for value in "${depends[@]}"; do printf 'depend = %s\n' "$value"; done
    for value in "${license[@]}"; do printf 'license = %s\n' "$value"; done
    for value in "${optdepends[@]:-}"; do [[ -z "$value" ]] || printf 'optdepend = %s\n' "$value"; done
    for value in "${provides[@]}"; do printf 'provides = %s\n' "$value"; done
    for value in "${conflicts[@]}"; do printf 'conflict = %s\n' "$value"; done
} > "$pkgdir/.PKGINFO"
(cd "$pkgdir" && bsdtar --uid 0 --gid 0 --format=mtree --options='!all,use-set,type,uid,gid,mode,time,size,sha256,link' -czf .MTREE .PKGINFO usr)
package_file="$pkgname-$pkgver-$pkgrel-x86_64.pkg.tar.zst"
# Package ownership is root without running this build as root.
tar --zstd --owner=0 --group=0 --numeric-owner -cf "$out_dir/$package_file" -C "$pkgdir" .PKGINFO .MTREE usr
(cd "$out_dir" && sha256sum "$package_file" > "$package_file.sha256")
printf 'Arch package: %s/%s\n' "$out_dir" "$package_file"
