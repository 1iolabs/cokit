#!/bin/bash
set -e
BASE_DIR="$(dirname "$(readlink -f "$0")")"
cd "$BASE_DIR"

echo "> building...";
cargo build

echo "> generate bindings...";
/Users/dominik/Workspaces/github/NiallBunting/uniffi-rs-dart/target/debug/uniffi-bindgen generate --library ../../target/debug/libco_uniffi.dylib --language dart --out-dir ./lib/src/generated --no-format
