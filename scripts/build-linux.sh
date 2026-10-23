#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
    printf '%s\n' 'This script builds the native Linux client. Run it on Linux.' >&2
    exit 1
fi

if (( $# != 0 )); then
    printf '%s\n' 'Usage: ./scripts/build-linux.sh' >&2
    printf '%s\n' 'Use Cargo environment variables for jobs and the target directory.' >&2
    exit 2
fi

for dependency in cargo protoc; do
    if ! command -v "$dependency" >/dev/null 2>&1; then
        printf 'Missing build dependency: %s\n' "$dependency" >&2
        printf '%s\n' 'See README-LINUX.md. protoc is supplied by protobuf-compiler on Debian/Ubuntu.' >&2
        exit 1
    fi
done

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_dir"

cargo build --locked --release \
    -p maxima-ui \
    -p maxima-bootstrap \
    -p maxima-cli

printf '%s\n' 'Build completed. Keep maxima, maxima-bootstrap and maxima-cli together.'
printf '%s\n' 'Default output: target/release/ (or your configured Cargo output directory).'
