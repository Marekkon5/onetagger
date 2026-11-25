#!/usr/bin/env bash
set -euo pipefail

app="$1"
archive="$2"
scratch="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/onetagger-sign.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

codesign --force --sign - "$app"
ditto -c -k --sequesterRsrc --keepParent "$app" "$archive"
ditto -x -k "$archive" "$scratch"
codesign --verify --deep --strict --verbose=2 "$scratch/${app##*/}"
