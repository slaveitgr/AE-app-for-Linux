#!/usr/bin/env bash
set -euo pipefail
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="${1:-$repo_dir/dist/AEApp-for-Linux}"
binary_dir="${CARGO_TARGET_DIR:-$repo_dir/target}/release"
mkdir -p "$package_dir"
for binary in aeapp aeapp-bootstrap aeapp-cli; do
    install -m 755 "$binary_dir/$binary" "$package_dir/$binary"
done
install -m 644 "$repo_dir/LICENSE" "$package_dir/LICENSE"
install -m 644 "$repo_dir/CREDITS.md" "$package_dir/CREDITS.md"
install -m 644 "$repo_dir/README-LINUX.md" "$package_dir/README.md"
install -m 644 "$repo_dir/aeapp-resources/assets/logo.png" "$package_dir/aeapp-icon.png"
install -m 755 "$repo_dir/scripts/install-linux.sh" "$package_dir/install-linux.sh"
cat > "$package_dir/aeapp.sh" <<'LAUNCH'
#!/usr/bin/env bash
set -euo pipefail
app_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec "$app_dir/aeapp" "$@"
LAUNCH
cat > "$package_dir/aeapp-preview.sh" <<'PREVIEW'
#!/usr/bin/env bash
set -euo pipefail
app_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec "$app_dir/aeapp.sh" --preview "$@"
PREVIEW
chmod 755 "$package_dir/aeapp.sh" "$package_dir/aeapp-preview.sh"
printf 'Created %s\n' "$package_dir"
