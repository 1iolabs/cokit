#!/usr/bin/env bash
set -euo pipefail

# triggered by tag push — publishes crates to crates.io and npm packages
# version bumps, commits, and tags are done locally by the developer

echo "> publishing Rust crates to crates.io"
cargo publish --workspace

echo "> building and publishing npm packages"
(cd co-js && wasm-pack build --target web && cd pkg && npm publish --access public)
(cd tauri-plugin-co-sdk && npm run build && npm publish --access public)

echo "> publish complete"
