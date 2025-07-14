#!/bin/bash
set -e
BASE_DIR="$(dirname "$(dirname "$(dirname "$(readlink -f "$0")")")")"
cd "$BASE_DIR/docs"
mdbook-admonish install .
mdbook-mermaid install .
mdbook build .
