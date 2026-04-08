#!/usr/bin/env bash
set -euo pipefail

echo "> checking release readiness"

# 1. verify changelogs exist
MISSING=()
for dir in co-macros co-primitives co-actor co-storage co-api co-messaging \
    co-runtime co-identity co-log co-network co-js co-sdk co-dioxus \
    co-bindings co-cli tauri-plugin-co-sdk \
    cores/board cores/co cores/file cores/keystore cores/membership \
    cores/names cores/rich-text cores/room cores/storage; do
    [ ! -f "${dir}/CHANGELOG.md" ] && MISSING+=("${dir}")
done

if [ ${#MISSING[@]} -gt 0 ]; then
    echo "ERROR: Missing CHANGELOG.md in: ${MISSING[*]}"
    exit 1
fi

# 2. check for accidental semver violations
echo "> running cargo-semver-checks"
if command -v cargo-semver-checks &> /dev/null; then
    cargo semver-checks check-release --workspace || {
        echo "WARNING: semver check found violations — review before releasing"
    }
else
    echo "WARNING: cargo-semver-checks not installed, skipping"
fi

echo "> release check passed"
