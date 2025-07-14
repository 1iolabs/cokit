FROM rust:1.82

# OS
RUN apt-get update \
 && apt-get install --no-install-recommends -y musl-tools

# RUST
RUN rustup component add rustfmt \
 && rustup target add wasm32-unknown-unknown \
 && rustup toolchain install nightly \
 && rustup component add --toolchain nightly rustfmt

# MDBOOK
COPY scripts/docs-setup.sh /usr/local/bin/docs-setup.sh
RUN /usr/local/bin/docs-setup.sh \
 && rm -f /usr/local/bin/docs-setup.sh
