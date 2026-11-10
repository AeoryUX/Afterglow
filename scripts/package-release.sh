#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
release_binary="$project_dir/src-tauri/target/release/afterglow"
[[ -x "$release_binary" ]] || { echo 'First run: npm ci && npm run tauri build -- --no-bundle' >&2; exit 1; }
out_dir="${1:-$project_dir/releases}"
mkdir -p "$out_dir"
out_dir="$(cd -- "$out_dir" && pwd)"
stage="$(mktemp -d)"
trap 'rm -rf -- "$stage"' EXIT
name=afterglow-0.2.2-linux-x86_64
mkdir -p "$stage/$name"
install -m755 "$release_binary" "$stage/$name/afterglow"
install -m755 "$project_dir/scripts/afterglow-save" "$stage/$name/afterglow-save"
install -m755 "$project_dir/scripts/install.sh" "$stage/$name/install.sh"
install -m644 "$project_dir/src-tauri/icons/128x128.png" "$stage/$name/icon.png"
install -m644 "$project_dir/packaging/afterglow.desktop" "$stage/$name/afterglow.desktop"
install -m644 "$project_dir/README.md" "$stage/$name/README.md"
install -m644 "$project_dir/THIRD_PARTY_NOTICES.txt" "$stage/$name/THIRD_PARTY_NOTICES.txt"
install -m644 "$project_dir/SECURITY.md" "$stage/$name/SECURITY.md"
install -m644 "$project_dir/VALIDATION.md" "$stage/$name/VALIDATION.md"
install -m644 "$project_dir/COPYING" "$stage/$name/COPYING"
install -m644 "$project_dir/LICENSE.md" "$stage/$name/LICENSE.md"
tar -czf "$out_dir/$name.tar.gz" -C "$stage" "$name"
(cd -- "$out_dir" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
printf 'Release: %s/%s.tar.gz\n' "$out_dir" "$name"
