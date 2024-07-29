FROM rust:1.79
RUN apt-get update
RUN apt-get install -y \
    build-essential \
    libgtk-3-dev \
    libsoup-3.0-dev \
    libjavascriptcoregtk-4.1-dev \
    libwebkit2gtk-4.1-dev
RUN rustup component add rustfmt \
    && rustup target add wasm32-unknown-unknown \
    && rustup toolchain install nightly
