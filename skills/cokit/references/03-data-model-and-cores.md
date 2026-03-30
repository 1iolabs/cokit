# Data Model and Cores

## What a Core Is

A Core (CO Reducer) combines a data model (schema) with business logic (reducer).
It is a pure function: given the same state and action, it always produces the same output.
This determinism is essential for distributed state verification.

Cores are compiled to WebAssembly (WASM) and executed in a sandbox on every peer.

## Core Structure

### Schema
The data model. In Rust, defined using structs and enums with the `#[co]` attribute macro
(from `co-api`). The macro auto-implements required serialization traits.

Advanced data types from co-primitives: CoMap, CoSet, CoList -- all content-addressed.

### Actions
Operations that modify state. Should be designed as logical tasks (not split across
multiple actions). More order-independent = better CRDT conflict handling.

Each action is serialized into a content-addressed block. Annotated with `#[co]`.

### State
The root state produced by sequentially applying the Log's ordered actions via the reducer.
State is also serialized into content-addressed blocks.

### Reducer
The `Reducer<A>` trait implementation. Signature (async):

```
async fn reduce(
    state_link: OptionLink<Self>,    // CID of current state (None for first event)
    event_link: Link<ReducerAction<A>>,  // CID of the action event
    storage: &CoreBlockStorage,       // block storage for reading/writing
) -> Result<Link<Self>, anyhow::Error>  // CID of new state
```

The reducer reads the action and current state from storage, computes the new state,
writes it to storage, and returns the new state's CID.

### Core Requirements

Cores need to be executed as WASM because:
- of different versions
- a CO can have unknown cores and we need them to calculate and verify its overall state

`Core::Native`/`Core::NativeAsync` are just helper which can be used during development.

## Core Characteristics

- **Passive**: Cores only compute state. They cannot trigger side effects or reactions.
- **Atomic**: Each reduce operation is all-or-nothing.
- **Isolated**: Cores execute independently, enabling parallel execution.
- **Serializable**: All state/actions use content-addressed blocks (default: DAG-CBOR).
  Block size limit: 1 MiB. Other formats acceptable if block-compatible.
- **Validatable**: Since the reducer is deterministic WASM, all peers compute the same
  result, enabling verification.
- **Composable**: Cores can be composed into higher-order Cores via composition.

## Core Migrations

Migrating from one schema version to another is just another action. When updating a
Core binary, a migration action can transform v1 state to v2 state using normal reducer
logic. No special migration framework needed.

## Built-in Cores

| Core | Purpose |
|------|---------|
| co-core-co | Root Core managing Cores, Guards, and Participants of a CO |
| co-core-keystore | Stores credentials (DID/PeerID private keys) in Local CO |
| co-core-membership | Tracks CO memberships in Local CO |
| co-core-board | Kanban board for coordinating pending network requests |
| co-core-storage | Reference tracking for storage blocks (garbage collection) |
| co-core-poa | Proof-of-Authority consensus implementation |
| co-core-room | Matrix-compatible messaging |
| co-core-file | Hierarchical file structures |
| co-core-rich-text | Conflict-free rich text |
| co-core-role | Role-based access control rules |

## Common Data Types (co-primitives)

- **CoMap<K, V>** - Content-addressed key/value map (sorted by keys, async operations).
  Supports transactions for optimized batch operations.
- **CoSet<V>** - Content-addressed set (sorted by values, async).
- **CoList<V>** - Content-addressed ordered list (async). Uses rational number indices
  for insert-between without rewriting.
- **BlockSerializer** - Creates Blocks from serde::Serialize data using DAG-CBOR.
- **NodeBuilder / NodeStream** - Build/read graphs for growing lists of items.
  Default node count: 172.

## WASM Interface (co_v1 API)

Core WASM binaries must export:
- `fn state()` for reducers
- `fn guard()` for guards

The WASM host provides imports for:
- Block storage (get/set by CID)
- State CID read/write
- Event CID read
- Diagnostics

The `co-api` `#[co]` macro handles these exports automatically.

## Development Workflow for Cores

1. `cargo init --lib ./my-core`
2. `cargo add co-api --git https://gitlab.1io.com/1io/co-sdk.git --branch wasm`
3. Add `crate-type = ["lib", "cdylib"]` and `features = ["core"]` to Cargo.toml
4. Define schema with `#[co]`, actions with `#[co]`, state with `#[co(state)]`
5. Implement `co_api::async_reducer::Reducer<MyAction> for MyState`
6. Build: `co core build` (produces .wasm in target-wasm/)

## Source Map

- docs/src/reference/core.md (primary: Core concept, structure, characteristics, built-in Cores)
- docs/src/getting-started/rust-core-quick-start.md (primary: development workflow)
- docs/src/usage/api-overview.md (primary: CoMap, CoSet, CoList, BlockSerializer, NodeBuilder)
- docs/src/usage/api-overview-core.md (primary: Reducer trait, Guard trait, WASM interface)
- docs/src/reference/co.md (context: Cores within COs)
- docs/src/usage/best-practices.md (pitfalls: stable IDs, task-based actions)
