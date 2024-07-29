# CO

## Abstract
CO implementation using the rust progamming language.

## Development

### Setup

Dependencies:
- `rust-1.76` (MSRV)
- `rustfmt`
- `wasm32-unknown-unknown` to build cores.
- `toolchain nightly` to use `rustfmt +nightly`

Commands:
```shell
rustup component add rustfmt
rustup target add wasm32-unknown-unknown
rustup toolchain install nightly
rustup component add --toolchain nightly rustfmt
```

### Utility

fmt:
```shell
cargo +nightly fmt --check
```

## Log

```shell
tail -f data/log/co.log | bunyan -c '!/^(libp2p|hickory_proto|dioxus_core|log|quinn|tower|tonic|h2|hyper|quinn_proto|tokio_util::codec::framed_impl)/.test(this.target)'
```
