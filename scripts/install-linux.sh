#!/usr/bin/env bash
set -euo pipefail

usage() {
    printf '%s\n' 'Usage: install-linux.sh [--from BINARY_DIR] [--assets-dir LOCAL_ASSETS] [--icon LOCAL_PNG]'
    printf '%s\n' 'Installs for the current user. Existing aeapp account/game data and local fonts are preserved.'
}

if [[ "$(uname -s)" != Linux ]]; then
    printf '%s\n' 'This installer requires Linux.' >&2
    exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
binary_dir=''
assets_dir=''
icon_source=''
while (( $# )); do
    case "$1" in
        --from|--assets-dir|--icon)
            if (( $# < 2 )); then usage >&2; exit 2; fi
            case "$1" in
                --from) binary_dir="$2" ;;
                --assets-dir) assets_dir="$2" ;;
                --icon) icon_source="$2" ;;
            esac
            shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done

if [[ -z "$binary_dir" ]]; then
    if [[ -x "$script_dir/aeapp" ]]; then
        binary_dir="$script_dir"
    elif [[ -x "$repo_dir/dist/AEApp-for-Linux/aeapp" ]]; then
        binary_dir="$repo_dir/dist/AEApp-for-Linux"
    else
        binary_dir="${CARGO_TARGET_DIR:-$repo_dir/target}/release"
    fi
fi
if [[ ! -d "$binary_dir" ]]; then
    printf 'Binary directory does not exist: %s\n' "$binary_dir" >&2
    exit 1
fi
binary_dir="$(cd -- "$binary_dir" && pwd)"
for binary in aeapp aeapp-bootstrap aeapp-cli; do
    if [[ ! -f "$binary_dir/$binary" || ! -x "$binary_dir/$binary" ]]; then
        printf 'Missing native executable: %s/%s\n' "$binary_dir" "$binary" >&2
        exit 1
    fi
    magic="$(od -An -tx1 -N4 "$binary_dir/$binary")"
    if [[ "${magic// /}" != 7f454c46 ]]; then
        printf 'Expected a native ELF executable: %s/%s\n' "$binary_dir" "$binary" >&2
        exit 1
    fi
done

data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
install_dir="$data_home/ae-app-for-linux"
bin_dir="$HOME/.local/bin"
applications_dir="$data_home/applications"
app_id='io.github.slaveitgr.aeapp'
wrapper="$bin_dir/aeapp"
desktop_file="$applications_dir/$app_id.desktop"

if [[ -z "$assets_dir" && -d "$binary_dir/ea-assets" ]]; then
    assets_dir="$binary_dir/ea-assets"
fi
if [[ -n "$assets_dir" && ! -d "$assets_dir" ]]; then
    printf 'Local assets directory does not exist: %s\n' "$assets_dir" >&2
    exit 1
fi
if [[ -z "$icon_source" && -n "$assets_dir" && -f "$assets_dir/icon.png" ]]; then
    icon_source="$assets_dir/icon.png"
elif [[ -z "$icon_source" && -f "$install_dir/ea-assets/icon.png" ]]; then
    icon_source="$install_dir/ea-assets/icon.png"
elif [[ -z "$icon_source" && -f "$binary_dir/aeapp-icon.png" ]]; then
    icon_source="$binary_dir/aeapp-icon.png"
elif [[ -z "$icon_source" ]]; then
    icon_source="$repo_dir/aeapp-resources/assets/logo.png"
fi
if [[ ! -f "$icon_source" ]]; then
    printf 'Icon PNG was not found: %s\n' "$icon_source" >&2
    exit 1
fi
header="$(od -An -tu1 -v -N24 "$icon_source")"
read -r -a png_header <<< "${header//$'\n'/ }"
if (( ${#png_header[@]} != 24 )) ||
    [[ "${png_header[*]:0:8}" != '137 80 78 71 13 10 26 10' ||
       "${png_header[*]:12:4}" != '73 72 68 82' ]]; then
    printf 'Expected a PNG icon: %s\n' "$icon_source" >&2
    exit 1
fi
icon_width=$((png_header[16] * 16777216 + png_header[17] * 65536 + png_header[18] * 256 + png_header[19]))
icon_height=$((png_header[20] * 16777216 + png_header[21] * 65536 + png_header[22] * 256 + png_header[23]))
if (( icon_width < 1 || icon_width > 4096 || icon_width != icon_height )); then
    printf '%s\n' 'The icon must be a square PNG no larger than 4096 pixels.' >&2
    exit 1
fi

mkdir -p "$install_dir/ea-assets" "$bin_dir" "$applications_dir"
temporary_file=''
trap 'if [[ -n "$temporary_file" ]]; then rm -f -- "$temporary_file"; fi' EXIT
for binary in aeapp aeapp-bootstrap aeapp-cli; do
    temporary_file="$(mktemp "$install_dir/.update-$binary.XXXXXX")"
    install -m 755 "$binary_dir/$binary" "$temporary_file"
    mv -f -- "$temporary_file" "$install_dir/$binary"
    temporary_file=''
done
if [[ -n "$assets_dir" ]]; then
    for font in ea-text.otf ea-display.otf; do
        if [[ -f "$assets_dir/$font" && "$assets_dir/$font" != "$install_dir/ea-assets/$font" ]]; then
            install -m 644 "$assets_dir/$font" "$install_dir/ea-assets/$font"
        fi
    done
fi
if [[ "$icon_source" != "$install_dir/ea-assets/icon.png" ]]; then
    install -m 644 "$icon_source" "$install_dir/ea-assets/icon.png"
fi
icon_dir="$data_home/icons/hicolor/${icon_width}x${icon_height}/apps"
mkdir -p "$icon_dir"
install -m 644 "$install_dir/ea-assets/icon.png" "$icon_dir/$app_id.png"
for document in LICENSE CREDITS.md README.md; do
    if [[ -f "$binary_dir/$document" ]]; then
        install -m 644 "$binary_dir/$document" "$install_dir/$document"
    elif [[ -f "$repo_dir/$document" ]]; then
        install -m 644 "$repo_dir/$document" "$install_dir/$document"
    fi
done

{
    printf '%s\n' '#!/usr/bin/env bash' 'set -euo pipefail'
    printf 'app_dir=%q\n' "$install_dir"
    printf '%s\n' 'export AEAPP_INSTALLED=1'
    printf '%s\n' 'exec "$app_dir/aeapp" "$@"'
} > "$wrapper"
chmod 755 "$wrapper"

# Desktop Entry Exec uses its own escaping rules, independently of shell syntax.
desktop_exec="$wrapper"
desktop_exec="${desktop_exec//\\/\\\\}"
desktop_exec="${desktop_exec//\"/\\\"}"
desktop_exec="${desktop_exec//\$/\\\$}"
desktop_exec="${desktop_exec//\`/\\\`}"
desktop_exec="${desktop_exec//%/%%}"
cat > "$desktop_file" <<DESKTOP
[Desktop Entry]
Type=Application
Name=aeapp
Comment=Native Linux game launcher with Proton support
Exec="$desktop_exec"
TryExec=$wrapper
Icon=$install_dir/ea-assets/icon.png
Terminal=false
Categories=Game;
StartupNotify=true
StartupWMClass=$app_id
DESKTOP
chmod 644 "$desktop_file"

bootstrap_exec="$install_dir/aeapp-bootstrap"
bootstrap_exec="${bootstrap_exec//\\/\\\\}"
bootstrap_exec="${bootstrap_exec//\"/\\\"}"
bootstrap_exec="${bootstrap_exec//\$/\\\$}"
bootstrap_exec="${bootstrap_exec//\`/\\\`}"
bootstrap_exec="${bootstrap_exec//%/%%}"
cat > "$applications_dir/aeapp-qrc.desktop" <<CALLBACK
[Desktop Entry]
Type=Application
Name=aeapp browser callback
NoDisplay=true
Terminal=false
Exec=/usr/bin/env "$bootstrap_exec" %u
MimeType=x-scheme-handler/qrc;
CALLBACK
chmod 644 "$applications_dir/aeapp-qrc.desktop"
if command -v xdg-mime >/dev/null 2>&1; then
    if ! xdg-mime default aeapp-qrc.desktop x-scheme-handler/qrc; then
        printf '%s\n' 'Could not set the qrc callback association; check xdg-utils.' >&2
        exit 1
    fi
else
    printf '%s\n' 'Install xdg-utils before using browser sign-in (xdg-mime was not found).' >&2
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$applications_dir" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$data_home/icons/hicolor" >/dev/null 2>&1 || true
fi
printf 'Installed application: %s\n' "$install_dir"
printf 'Application menu entry: %s\n' "$desktop_file"
printf 'Launch command: %s\n' "$wrapper"
