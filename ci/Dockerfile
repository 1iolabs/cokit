FROM docker-registry.intranet.1io.com/extern/rust:1.76
RUN apt-get update \
    && apt-get install -y build-essential libgtk-3-dev
RUN rustup component add rustfmt \
    && rustup target add wasm32-unknown-unknown
