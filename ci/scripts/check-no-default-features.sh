#!/bin/bash
# Check that each workspace member builds without default features.
#
# Rationale: most crates should compose into downstream projects even when the
# consumer turns off default features. A few top-level crates intrinsically
# require a minimum feature set (transport backend, runtime backend, binding
# flavor) — those are listed with a baseline set of features below.
set -e
BASE_DIR="$(dirname "$(dirname "$(dirname "$(readlink -f "$0")")")")"
cd "$BASE_DIR"

check () {
    PKG="$1"; shift
    echo "> checking $PKG $* ..."
    cargo check -p "$PKG" --no-default-features "$@"
}

# Libraries: must build with zero features.
check co-actor
check co-api
check co-core-board
check co-core-co
check co-core-file
check co-core-keystore
check co-core-membership
check co-core-names
check co-core-rich-text
check co-core-room
check co-core-storage
check co-identity
check co-js
check co-log
check co-macros
check co-messaging
check co-primitives
check co-runtime
check co-sdk
check co-storage
check co-test
check example-counter
check example-counter-upgraded
check example-message

# Packages that intrinsically require a minimum feature set.
# - co-network: p2p transport requires either `native` (libp2p/tcp/quic/...) or
#   `js` on wasm32 (webrtc/websocket).
check co-network -F native

# - co-bindings: FFI surface is gated on `frb` (Flutter) or `uniffi`. The
#   generated frb bindings also reference the `network` feature surface, so
#   include that here.
check co-bindings -F frb,network

# - co-cli: binary that requires at least one runtime backend and the network
#   feature to expose its full command surface.
check co-cli -F cranelift,wasmi,pinning,network

# - co-dioxus: targets a platform bundle (desktop/mobile/web). Check desktop.
check co-dioxus -F desktop

# - tauri-plugin-co-sdk: inherits the recommended co-sdk bundle.
check tauri-plugin-co-sdk -F co-sdk/recommended
