FROM rust:1.88

# OS
RUN apt-get update \
 && apt-get install --no-install-recommends -y \
    musl-tools \
    libgtk-3-dev \
    libsoup-3.0-dev \
    libjavascriptcoregtk-4.1-dev \
    libwebkit2gtk-4.1-dev

# LLVM
RUN apt-get install --no-install-recommends -y wget gnupg lsb-release software-properties-common \
 && wget https://apt.llvm.org/llvm.sh \
 && chmod +x llvm.sh \
 && ./llvm.sh 18 \
 && rm llvm.sh

# RUST
RUN rustup component add rustfmt \
 && rustup target add wasm32-unknown-unknown \
 && rustup toolchain install nightly \
 && rustup component add --toolchain nightly rustfmt

# cargo-binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# MDBOOK
COPY scripts/docs-setup.sh /usr/local/bin/docs-setup.sh
RUN /usr/local/bin/docs-setup.sh \
 && rm -f /usr/local/bin/docs-setup.sh
