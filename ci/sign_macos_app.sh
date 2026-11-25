#!/usr/bin/env bash
set -euo pipefail

app="$1"
archive="$2"
scratch="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/onetagger-sign.XXXXXX")"
keychain="$scratch/signing.keychain-db"
cleanup() {
    if [[ -f "$keychain" ]]; then
        security delete-keychain "$keychain" || true
    fi
    rm -rf "$scratch"
}
trap cleanup EXIT

distribution="${MACOS_DISTRIBUTION:-false}"
if [[ "$distribution" == true ]]; then
    for secret in MACOS_SIGNING_P12 MACOS_SIGNING_PASSWORD MACOS_APPLE_ID MACOS_TEAM_ID MACOS_NOTARY_PASSWORD; do
        if [[ -z "${!secret:-}" ]]; then
            echo "::error::Set the $secret repository secret before building a macOS release."
            exit 1
        fi
    done
    umask 077
    printf '%s' "$MACOS_SIGNING_P12" | base64 -D > "$scratch/certificate.p12"
    password="$(openssl rand -hex 32)"
    security create-keychain -p "$password" "$keychain"
    security unlock-keychain -p "$password" "$keychain"
    security import "$scratch/certificate.p12" -k "$keychain" -P "$MACOS_SIGNING_PASSWORD" -T /usr/bin/codesign
    security set-key-partition-list -S apple-tool:,apple: -s -k "$password" "$keychain"
    identity="$(security find-identity -v -p codesigning "$keychain" |
        awk -v team="$MACOS_TEAM_ID" '/"Developer ID Application:/ && index($0, "(" team ")") { print $2 }')"
    if [[ ! "$identity" =~ ^[0-9A-Fa-f]{40}$ ]]; then
        echo "::error::Expected one valid Developer ID Application identity for the configured Apple team."
        exit 1
    fi
    codesign --force --options runtime --timestamp --keychain "$keychain" --sign "$identity" "$app"
    ditto -c -k --sequesterRsrc --keepParent "$app" "$scratch/notary.zip"
    xcrun notarytool submit "$scratch/notary.zip" --apple-id "$MACOS_APPLE_ID" \
        --team-id "$MACOS_TEAM_ID" --password "$MACOS_NOTARY_PASSWORD" --wait --timeout 10m
    xcrun stapler staple "$app"
else
    codesign --force --sign - "$app"
fi

ditto -c -k --sequesterRsrc --keepParent "$app" "$archive"
ditto -x -k "$archive" "$scratch/extracted"
packaged_app="$scratch/extracted/${app##*/}"
codesign --verify --deep --strict --verbose=2 "$packaged_app"
if [[ "$distribution" == true ]]; then
    xcrun stapler validate "$packaged_app"
    syspolicy_check distribution "$packaged_app"
fi
