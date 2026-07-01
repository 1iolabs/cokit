#!/bin/bash
set -e
BASE_DIR="$(dirname "$(dirname "$(dirname "$(readlink -f "$0")")")")"
cd "$BASE_DIR"

# lint
./ci/scripts/lint-fmt.sh
./ci/scripts/lint-clippy.sh
