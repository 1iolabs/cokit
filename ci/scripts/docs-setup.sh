#!/bin/bash
set -e

# check for binstall
if command -v cargo-binstall >/dev/null 2>&1; then
    export HAVE_CARGO_BINSTALL=1
fi

# lib
cargo_install_package () {
    PACKAGE_NAME="$1"
    PACKAGE_VERSION="$2"
    if [ -n "$HAVE_CARGO_BINSTALL" ]; then
        cargo binstall "$PACKAGE_NAME" --version "$PACKAGE_VERSION" -y
    else
        cargo install "$PACKAGE_NAME" --version "$PACKAGE_VERSION" --locked
    fi
}

# install
cargo_install_package mdbook "0.4.52"
cargo_install_package mdbook-mermaid "0.16.0"
cargo_install_package mdbook-toc "0.14.2"
cargo_install_package mdbook-admonish "1.20.0"
