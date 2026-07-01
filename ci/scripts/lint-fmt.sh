#!/bin/bash
set -e
BASE_DIR="$(dirname "$(dirname "$(dirname "$(readlink -f "$0")")")")"
cd "$BASE_DIR"

# optional autofix: pass --fix (or fix) to format in place instead of only checking.
# -l lists the files that were reformatted.
fmt_args=(--check)
if [ "$1" = "--fix" ] || [ "$1" = "fix" ]; then
	fmt_args=(-- -l)
fi

# fmt
cargo +nightly-2025-12-09 fmt "${fmt_args[@]}"
