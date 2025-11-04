# Installation
To get the fun started, you'll need to install rust and the `co` command.
This step-by-step guide covers both installing the tools, as well as building your first CO-kit app!

## Requirements
- `rust-1.88` or greater
- `llvm-18`

## Setup Rust

### Rustup
Rustup is the Rust toolchain installer.

Head over to <https://www.rust-lang.org/tools/install> for further instructions.

### Cargo B(inary)Install
We use this to speed up the the installation for the 'co' and 'dx' commands. ('dx' is used in the [App Quick Start](../getting-started/rust-app-quick-start.md)). You can skip this step if you want to build them from source.

Head over to <https://github.com/cargo-bins/cargo-binstall?tab=readme-ov-file#installation> for further instructions.

### Runtimes
Later in this documentation, you will hear about [cores](../reference/core.md) which are data models. They will be compiled to [WebAssembly (WASM)](/glossary/glossary.md#wasm). We need the compiler toolchain for it which can be installed through:
```sh
rustup target add wasm32-unknown-unknown
```

## LLVM

LLVM-18.0.0 is used to execute WebAssembly files as native code.

### MacOS

To install it using macOS use:

```sh
brew install llvm@18
```

### Setup cargo

To let cargo know where to search for llvm we need to add the `LLVM_SYS_180_PREFIX` variable to the cargo config.
Here is a script to help you with that:

```sh
mkdir -p ~/.cargo
touch ~/.cargo/config.toml
echo "[env]\nLLVM_SYS_180_PREFIX = \"$("brew" "--prefix" "llvm@18")\"" >> ~/.cargo/config.toml
cat ~/.cargo/config.toml
```

It should look likle this then:

`~/.cargo/config.toml`:
```toml
[env]
LLVM_SYS_180_PREFIX = "/opt/homebrew/opt/llvm@18"
```

```admonish note
The script may duplicates the `[env]` table which needs to be fixed manually.
```

## Setup CO-kit
CO-kit ships pre-built binaries[^issue-94] for its `co` CLI using [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall?tab=readme-ov-file#installation). This means you can install `co` without needing to compile from source:
```sh
cargo binstall co-cli --git https://gitlab.1io.com/1io/co-sdk.git
```

[^issue-94]: [Support cargo binstall (#94)](https://gitlab.1io.com/1io/co-sdk/-/issues/94)

Of course, you can build it from source, too:
```sh
cargo install co-cli --git https://gitlab.1io.com/1io/co-sdk.git
```

## Building your first app
Lets build a collaborative todo list.
Keep in mind that you can build any application with CO-kit, though whenever you need to think about some kind of collaboration, CO-kit is destined for the job.

For any app you build you need these two ingredients:
1. A core which is the data model of the app:
	- [Core Quick Start](../getting-started/rust-core-quick-start.md)
2. Setup an Application which uses the core:
	- [App Quick Start](../getting-started/rust-app-quick-start.md)

We'll take a closer look at these in the following two chapters.
