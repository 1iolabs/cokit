#!/bin/bash
set -e
BASE_DIR="$(dirname "$(dirname "$(dirname "$(readlink -f "$0")")")")"
cd "$BASE_DIR"

# check
./ci/scripts/check-web.sh
./ci/scripts/check-no-default-features.sh
