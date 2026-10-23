#!/usr/bin/env bash
set -euo pipefail
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="${1:-$repo_dir/dist/EA-for-Linux}"
binary_dir="${CARGO_TARGET_DIR:-$repo_dir/target}/release"
mkdir -p "$package_dir"
for binary in maxima maxima-bootstrap maxima-cli; do
    install -m 755 "$binary_dir/$binary" "$package_dir/$binary"
done
install -m 644 "$repo_dir/LICENSE" "$package_dir/LICENSE"
install -m 644 "$repo_dir/README-LINUX.md" "$package_dir/README.md"
cat > "$package_dir/EA-for-Linux.sh" <<'LAUNCH'
#!/usr/bin/env bash
set -euo pipefail
app_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec "$app_dir/maxima" "$@"
LAUNCH
cat > "$package_dir/Preview.sh" <<'PREVIEW'
#!/usr/bin/env bash
set -euo pipefail
app_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec "$app_dir/EA-for-Linux.sh" --preview "$@"
PREVIEW
chmod 755 "$package_dir/EA-for-Linux.sh" "$package_dir/Preview.sh"
printf 'Created %s\n' "$package_dir"
