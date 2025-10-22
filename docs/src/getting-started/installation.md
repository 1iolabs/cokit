# Installation
To get the fun started, you'll need to install rust and the `co` command.
In this chapter we will briefly guide you through the process step-by-step.

We're starting off with the installation, then we'll delve right into building your first app.

## Requirements
- `rust-1.88` or greater.

## Setup Rust

### Rustup
Rustup is the rust toolchain installer.
Head over to https://www.rust-lang.org/tools/install for instructions.

### Cargo B(inary)Install
We use this to speed-up the installation for the `co` and `dx` (used in the [App Quick Start](../getting-started/rust-app-quick-start.md)) command. You can skip this step if you want to build them from source.

Head over to https://github.com/cargo-bins/cargo-binstall?tab=readme-ov-file#installation for further instructions.

### Runtimes
Later in this documentation you will hear about [cores](../reference/core.md) which are data models. They will be compiled to [WebAssembly (WASM)](/glossary/glossary.md#wasm). We need the compiler toolchain for it which can be installed through:
```sh
rustup target add wasm32-unknown-unknown
```

## Setup CO-kit
CO-kit ships prebuilt binaries for its `co` CLI using [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall?tab=readme-ov-file#installation). This means you can install `co` without needing to compile from source:
```sh
cargo binstall co-cli
```

Of course, you can build it from source, too:
```sh
cargo install co-cli
```

## Building your first app
Lets build a collaborative todo list.

For any app you build you need these two major ingredients:
1. A core which is the data model of the app:
	- [Core Quick Start](../getting-started/rust-core-quick-start.md)
2. Setup an Application which uses the core:
	- [App Quick Start](../getting-started/rust-app-quick-start.md)

We'll take a closer look at these in the following two chapters.