#!/usr/bin/env bash
# Shared, bounded static-asset copy for the local package and user installer.
# No account state, cache directories, executables, or arbitrary JSON is copied.
copy_local_assets() {
    local asset_source="$1" asset_destination="$2"
    local asset_file asset_relative asset_name asset_size asset_parent asset_component
    local -a asset_components
    local asset_count=0 asset_bytes=0
    if [[ ! -d "$asset_source" || -L "$asset_source" ]]; then
        printf 'Expected a regular local assets directory: %s\n' "$asset_source" >&2
        return 1
    fi
    asset_source="$(cd -- "$asset_source" && pwd -P)"
    if [[ -L "$asset_destination" ]]; then
        printf 'Refusing an assets destination symlink: %s\n' "$asset_destination" >&2
        return 1
    fi
    mkdir -p "$asset_destination"
    asset_destination="$(cd -- "$asset_destination" && pwd -P)"
    if [[ "$asset_source" == "$asset_destination" ]]; then return 0; fi
    while IFS= read -r -d '' asset_file; do
        asset_relative="${asset_file#"$asset_source/"}"
        asset_name="${asset_relative##*/}"
        if [[ ! "$asset_name" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,159}$ ]]; then continue; fi
        case "$asset_relative" in
            ea-text.otf|ea-text-bold.otf|ea-display.otf|ea-display-bold.otf|icon.png) ;;
            icons/*.svg)
                if [[ "${asset_relative#icons/}" == */* ]]; then continue; fi ;;
            covers/catalog.json|promotions/catalog.json|catalog/products.json) ;;
            covers/*|promotions/*|catalog/images/*)
                case "${asset_relative,,}" in
                    *.png|*.jpg|*.jpeg|*.webp) ;;
                    *) continue ;;
                esac
                case "$asset_relative" in
                    catalog/images/*) asset_parent="${asset_relative#catalog/images/}" ;;
                    *) asset_parent="${asset_relative#*/}" ;;
                esac
                if [[ "$asset_parent" == */* ]]; then continue; fi ;;
            *) continue ;;
        esac
        # find -P does not follow source symlinks. Bound accepted files and bytes.
        asset_size="$(stat -c %s -- "$asset_file")"
        asset_count=$((asset_count + 1))
        asset_bytes=$((asset_bytes + asset_size))
        if (( asset_count > 512 || asset_size > 16777216 || asset_bytes > 268435456 )); then
            printf '%s\n' 'Local assets exceed the 512-file, 16 MiB/file, or 256 MiB total limit.' >&2
            return 1
        fi
        asset_parent="$asset_destination"
        IFS='/' read -r -a asset_components <<< "$asset_relative"
        for asset_component in "${asset_components[@]}"; do
            asset_parent="$asset_parent/$asset_component"
            if [[ -L "$asset_parent" ]]; then
                printf 'Refusing an assets destination symlink: %s\n' "$asset_parent" >&2
                return 1
            fi
        done
        mkdir -p "$(dirname -- "$asset_destination/$asset_relative")"
        install -m 644 -- "$asset_file" "$asset_destination/$asset_relative"
    done < <(find -P "$asset_source" -mindepth 1 -maxdepth 3 -type f -print0)
    printf 'Copied %s local static assets (%s bytes).\n' "$asset_count" "$asset_bytes"
}
