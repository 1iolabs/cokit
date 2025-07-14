#!/bin/bash
set -e
cargo install mdbook --version "0.4.51" --locked
cargo install mdbook-mermaid --version "0.15.0" --locked
cargo install mdbook-toc --version "0.14.2" --locked
cargo install mdbook-admonish --version "1.20.0" --locked
