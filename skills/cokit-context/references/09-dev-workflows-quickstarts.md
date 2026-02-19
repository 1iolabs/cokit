# Developer Workflows and Quick Starts

## Requirements

- Rust 1.91+ (MSRV)
- LLVM 18 (for WASM native execution)
- wasm32-unknown-unknown target
- Nightly toolchain (for rustfmt)

## CRITICAL: Dependency Installation

**COkit crates are NOT published to crates.io.** All Rust dependencies must be added
via the git repository:
```sh
cargo add co-sdk --git https://gitlab.1io.com/1io/co-sdk.git
cargo add co-api --git https://gitlab.1io.com/1io/co-sdk.git
```

Do NOT use `cargo add co-sdk` without `--git` -- it will fail or pull the wrong package.

Similarly, COkit npm packages (`@1io/tauri-plugin-co-sdk-api`, `@1io/compare`, `co-js`)
are not on the public npm registry. They are installed from the project's own registry
or bundled locally.

## Installation

### Install co CLI
From source:
```sh
cargo install co-cli --git https://gitlab.1io.com/1io/co-sdk.git
```

### LLVM Setup (macOS)
```sh
brew install llvm@18
# Add to ~/.cargo/config.toml:
# [env]
# LLVM_SYS_180_PREFIX = "/opt/homebrew/opt/llvm@18"
```

### WASM Target
```sh
rustup target add wasm32-unknown-unknown
```

## Two-Part Development Model

Every COkit app has two parts:
1. **A Core** (data model + business logic) - built with `co-api`
2. **An Application** (frontend that uses the Core) - built with `co-sdk`

## Quick Start: Building a Core

1. Create Rust library crate
2. Add `co-api` dependency
3. Set `crate-type = ["lib", "cdylib"]` and add `features = ["core"]`
4. Define schema types with `#[co]` and `#[co(state)]`
5. Define actions enum with `#[co]`
6. Implement `Reducer<MyAction> for MyState`
7. Build to WASM: `co core build`

Key imports:
```rust
use co_api::{async_api::Reducer, co, BlockStorage, BlockStorageExt,
             CoMap, CoreBlockStorage, Link, OptionLink, ReducerAction};
```

## Quick Start: Rust App with Dioxus

Uses `co-dioxus` integration. Key setup:
```rust
let context = co_dioxus::CoContext::new(co_dioxus::CoSettings::cli("my-app"));
LaunchBuilder::desktop().with_context(context).launch(App);
```

Key hooks:
- `use_co(co_id)` - Opens a CO for read/write
- `use_selector(&co, |storage, co_state| async { ... })` - Selects state
- `use_did_key_identity(name)` - Gets/creates a did:key identity

Actions are dispatched via: `co.dispatch(identity, core_name, action)`

Cores can be added on-the-fly: `co.create_core_binary(identity, name, type, binary)`

## Quick Start: React App with Tauri

Uses `tauri-plugin-co-sdk` (Rust plugin) + `@1io/tauri-plugin-co-sdk-api` (TypeScript).

### Rust Side (src-tauri)
```rust
let co_settings = CoApplicationSettings::cli("my-app");
tauri::Builder::default()
    .plugin(tauri_plugin_co_sdk::init(co_settings).await)
    .run(tauri::generate_context!())
```

### TypeScript Side
Key packages: `@1io/tauri-plugin-co-sdk-api`, `co-js`, `@1io/compare`

Key hooks and functions:
- `useCoSession(coId)` - Opens a session for a CO
- `useCo(coId)` - Returns [stateCid, heads]
- `useCoCore(coCid, coreName, session)` - Gets Core state CID
- `useResolveCid<T>(cid, session)` - Resolves a CID to a typed value
- `useDidKeyIdentity(name)` - Gets/creates identity
- `useBlockStorage(session)` - Gets block storage for WASM operations
- `useCollectCoMap<T>(map, storage)` - Collects CoMap entries
- `pushAction(session, coreName, action, identity)` - Dispatches an action
- `createCo(identity, name, isPublic)` - Creates a new CO

TypeScript types mirror Rust Core types (TodoTask, TodoAction, etc.).
CoMap in TypeScript uses WASM wrapper from `co-js`.

### WASM Core Loading (React/Tauri)
The WASM binary goes in the `public/` folder. Load via:
```typescript
import { fetchBinary } from "@1io/tauri-plugin-co-sdk-api";
const stream = await fetchBinary("my_core.wasm");
```

## co CLI Commands

```
co co ls          - List all COs
co network listen - Listen for P2P connections
co core build     - Build current crate to WASM
co ipld           - IPLD utilities
co did            - Identity management
co storage        - Block storage operations
co file           - File operations
co room           - Room/messaging operations
co pin            - Pin operations
co schemars       - JSON schema generation
```

Global options: --base-path, --log-path, --no-log, --log-level, --no-keychain,
--instance-id, --open-telemetry, --feature, --no-default-features

## Application API (co-sdk)

Entry point: `ApplicationBuilder::new_with_path(name, path).build().await`

Key types:
- `Application` - Main entry point
- `CoContext` - Clonable handle for CO operations
- `CoReducer` - CO interaction handle (push actions, get state, join heads)
- `CoStorage` - CO's block storage instance
- `Cores` / `Guards` - Registries for built-in Cores and Guards
- `NetworkSettings` - Network startup configuration

Identity handling:
- `DidKeyIdentity::generate()` - Create new did:key
- `DidKeyProvider::new(local_co, keystore_name)` - Store provider
- `CoContext::identity_resolver()` - Resolve public identities
- `CoContext::private_identity_resolver()` - Resolve owned identities

UnixFS support: `unixfs_add`, `unixfs_add_file`, `unixfs_cat_buffer`,
`unixfs_stream`, `unixfs_encode_buffer`

## Environment Variables (Tauri)

- `CO_NO_KEYCHAIN=true` - Skip OS keychain (dev only, unsafe in production)
- `CO_BASE_PATH={path}` - Change data storage path

## Source Map

- docs/src/getting-started/installation.md (primary: requirements, setup)
- docs/src/getting-started/rust-core-quick-start.md (primary: Core development)
- docs/src/getting-started/rust-app-quick-start.md (primary: Dioxus app)
- docs/src/getting-started/react-app-quick-start.md (primary: Tauri/React app)
- docs/src/getting-started/first-steps.md (introductory concepts)
- docs/src/getting-started/next-steps.md (permissions example, more examples)
- docs/src/usage/cli.md (primary: CLI commands)
- docs/src/usage/api-overview-apps.md (primary: co-sdk API)
- docs/src/usage/configuration.md (app and CO configuration)
- README.md (development setup, MSRV, dependencies)
