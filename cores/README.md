# COre's

CO Reducers (COre) runtimes.

## List

| NAME | DESCRIPTION |
|------|-------------|
| `co` | CO Runtime. Manages the common CO state like participants, settings, .... |
| `keystore` | CO Key store |
| `membership` | CO Memberships. Stores membership information of an CO (counterpart to co.participants). |

## Build

Install wasm target:
```shell
rustup target add wasm32-unknown-unknown
```

Build cores:
```shell
cargo run --bin co-cli core build-builtin
```

Test:
```shell
cargo test --package co-sdk --lib -- types::cores::tests --nocapture
```

Note: Different compiler/OS may generate different assemblies. So the values in Cores.toml could change.

## Add new COre

- Add to `get_native` in: `../co-sdk/src/types/cores.rs`
- Build cores
