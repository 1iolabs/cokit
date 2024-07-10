# CO

## Abstract
CO implementation using the rust progamming language.

## Development

### Setup

```
rustup target add wasm32-unknown-unknown

```


## Log

```shell
tail -f data/log/co.log | bunyan -c '!/^(libp2p|hickory_proto|dioxus_core|log|quinn|tower|tonic|h2|hyper|quinn_proto|tokio_util::codec::framed_impl)/.test(this.target)'
```
