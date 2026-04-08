#!/usr/bin/env bash
set -euo pipefail

# cargo-release pre-release hook
# verifies the changelog contains an entry for the version being released
# environment variables provided by cargo-release:
#   CRATE_NAME, CRATE_ROOT, NEW_VERSION

CHANGELOG="${CRATE_ROOT}/CHANGELOG.md"

if [ ! -f "${CHANGELOG}" ]; then
    echo "ERROR: ${CHANGELOG} does not exist"
    echo "  run ./ci/scripts/update-changelog.sh first"
    exit 1
fi

if ! grep -q "\[${NEW_VERSION}\]" "${CHANGELOG}"; then
    echo "ERROR: ${CHANGELOG} has no entry for version ${NEW_VERSION}"
    echo "  run ./ci/scripts/update-changelog.sh and commit before releasing"
    exit 1
fi

echo "> changelog verified for ${CRATE_NAME} v${NEW_VERSION}"
