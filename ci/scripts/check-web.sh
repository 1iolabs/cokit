#!/bin/bash
set -e
BASE_DIR="$(dirname "$(dirname "$(dirname "$(readlink -f "$0")")")")"
cd "$BASE_DIR"

check_wasm () {
    PROJECT="$1"
    PROJECT_ARGS=${@:2}
    echo "> checking $PROJECT ... "
    cargo check --target wasm32-unknown-unknown --target-dir target-wasm -p "$PROJECT" $PROJECT_ARGS
}

check_wasm co-primitives
check_wasm co-actor
check_wasm co-api
check_wasm co-identity --no-default-features -F web
check_wasm co-storage --no-default-features -F web
check_wasm co-log --no-default-features -F web
check_wasm co-messaging
check_wasm co-macros
check_wasm co-runtime --no-default-features -F web
check_wasm co-sdk --no-default-features -F web
check_wasm co-dioxus --no-default-features -F web
