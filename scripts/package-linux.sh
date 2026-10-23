#!/usr/bin/env bash
set -euo pipefail
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="$repo_dir/dist/AEApp-for-Linux"
assets_dir=''
package_dir_set=false
while (( $# )); do
    case "$1" in
        --assets-dir)
            if (( $# < 2 )); then printf '%s\n' 'Missing --assets-dir value.' >&2; exit 2; fi
            assets_dir="$2"; shift 2 ;;
        -h|--help)
            printf '%s\n' 'Usage: package-linux.sh [PACKAGE_DIR] [--assets-dir LOCAL_ASSETS]'
            exit 0 ;;
        -*) printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
        *)
            if [[ "$package_dir_set" == true ]]; then printf '%s\n' 'Only one package directory is accepted.' >&2; exit 2; fi
            package_dir="$1"; package_dir_set=true; shift ;;
    esac
done
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
install -m 644 "$repo_dir/scripts/copy-local-assets.sh" "$package_dir/copy-local-assets.sh"
if [[ -n "$assets_dir" ]]; then
    source "$repo_dir/scripts/copy-local-assets.sh"
    copy_local_assets "$assets_dir" "$package_dir/ea-assets"
fi
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
