#!/bin/bash
set -e
BASE_DIR="$(dirname "$(dirname "$(dirname "$(readlink -f "$0")")")")"
cd "$BASE_DIR"

set-headers -p "*.rs" ./HEADER.md
cargo +nightly fmt
