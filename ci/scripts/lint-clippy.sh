#!/bin/bash
set -e
BASE_DIR="$(dirname "$(dirname "$(dirname "$(readlink -f "$0")")")")"
cd "$BASE_DIR"

# optional autofix: pass --fix (or fix) to apply clippy's suggestions in place.
# --allow-dirty/--allow-staged let it run against an uncommitted working tree.
fix_args=()
if [ "$1" = "--fix" ] || [ "$1" = "fix" ]; then
	fix_args=(--fix --allow-dirty --allow-staged)
fi

# clippy
cargo clippy "${fix_args[@]}" --workspace --all-targets --all-features -- -D warnings -W clippy::large_stack_frames -W clippy::large_futures
