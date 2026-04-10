---
name: cokit-core
description: "Build cokit cores using the co_api package. Use this skill whenever the user wants to create a new core (standalone or in-workspace), implement a Reducer or Guard, work with CoMap/CoList/CoSet collections, scaffold a WASM-compiled core module, or ask about core architecture, action design, CRDT considerations, or composition patterns. Also use when modifying existing cores."
---

# Building Cokit Cores

A cokit **core** (CO Reducer) is a Rust library compiled to WASM that combines **data model, business logic, and storage** in one unit. Cores receive actions, validate and verify them, apply business rules, and produce the next state. They are responsible for permission checks, input validation, and enforcing invariants — not just storing data. All state is content-addressed (CID-based) and persisted via `CoreBlockStorage`.

Cores can live **anywhere** — inside the cokit workspace (under `cores/`) or as standalone crates in their own repository. The only requirement is a dependency on `co-api`.

### Design Principles

- **Pure functions**: Reducers are deterministic — same state + same action = same result. This is what makes distributed state and validation possible. Cores have no facilities to react to state changes or perform side effects.
- **Atomic**: Each reduce operation is a single unit — it either succeeds completely or fails completely.
- **Isolated**: Cores execute in a WASM sandbox, which enables verifiability and parallel execution.
- **Composable**: Existing cores can be composed into higher-order cores. Don't mutate an original core — compose it, since it has a well-specified interface.

### The 1 MiB Block Limit

Every serializable item (block) has a **hard limit of 1 MiB**. This is why cokit provides its own collection types instead of using standard Rust collections:

| Instead of | Use | Why |
|---|---|---|
| `HashMap` / `BTreeMap` | `CoMap<K, V>` | Splits entries across multiple blocks so no single block exceeds 1 MiB |
| `HashSet` / `BTreeSet` | `CoSet<T>` | Same — distributes set members across blocks |
| `Vec` | `CoList<T>` | Same — distributes list items across blocks with fractional indexing |

**Never use `HashMap`, `BTreeMap`, `HashSet`, `BTreeSet`, or `Vec` for unbounded or growable data in state structs.** These serialize into a single block and will fail once the data exceeds 1 MiB. Use `BTreeMap`/`BTreeSet`/`Vec` only for small, bounded fields (e.g., a fixed set of enum variants, a handful of config entries) where you are certain the data will never approach the limit.

### Stable Identifiers

Cokit is a CRDT-based system — when concurrent actions arrive from multiple participants, transactions may be reordered during conflict resolution. This means **identifiers must be stable across reordering**. A monotonic counter (auto-increment ID) that works fine in a single-writer database will produce different values when actions are replayed in a different order, breaking any external references to those IDs.

Use identifiers that are **determined by the action itself**, not derived from current state:
- UUIDs generated client-side (e.g., `uuid::Uuid::new_v4().to_string()`)
- Content-derived IDs (hashes, CIDs)
- User-provided natural keys (URLs, email addresses, DIDs)

Never use counters, `state.len()`, or any state-dependent value as an identifier that may be referenced from outside the core.

## Quick Start

A core is a standard Rust crate with `crate-type = ["lib", "cdylib"]` and a dependency on `co-api`.

### Option A: Standalone Core (own repository)

```sh
cargo init --lib ./my-core
cd ./my-core
cargo add co-api serde anyhow
```

Then add to `Cargo.toml`:
```toml
[lib]
crate-type = ["lib", "cdylib"]

[features]
"core" = []
```

Build with: `co core build`

### Option B: Core inside the cokit workspace

For cores that live under `cores/<name>/` in the cokit workspace:

```toml
[package]
name = "co-core-<name>"
version = "0.1.0"
edition = "2021"
rust-version = "1.91"
license = "AGPL-3.0-only"
homepage = "https://www.cokit.org"
repository = "https://github.com/1iolabs/cokit.git"
documentation = "https://www.cokit.org/docs/"
description = "<One-line description of the core>"

[lib]
crate-type = ["lib", "cdylib"]

[dependencies]
anyhow = { workspace = true }
co-api = { workspace = true }
serde = { workspace = true, features = ["derive"] }

# Add as needed:
# cid = { workspace = true }
# ipld-core = { workspace = true }
# serde_repr = { workspace = true }
# schemars = { workspace = true }
# thiserror = { workspace = true }
# futures = { workspace = true }

[dev-dependencies]
co-storage = { workspace = true }
co-runtime = { workspace = true }

[features]
"core" = []
```

After creating the Cargo.toml, register the core in the **workspace** `Cargo.toml` at the repo root:
- Add `"cores/<name>"` to the `[workspace] members` list
- Add `co-core-<name> = { path = "cores/<name>", version = "0.1.0" }` to `[workspace.dependencies]`

### License Header

Source files in the cokit workspace start with:
```rust
// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH
```

### Minimal Core (src/lib.rs)

```rust
// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_api::{co, BlockStorageExt, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction};

#[co(state)]
pub struct MyState {
    pub value: String,
}

#[co]
pub enum MyAction {
    Set(String),
    Clear,
}

impl Reducer<MyAction> for MyState {
    async fn reduce(
        state: OptionLink<Self>,
        event: Link<ReducerAction<MyAction>>,
        storage: &CoreBlockStorage,
    ) -> Result<Link<Self>, anyhow::Error> {
        let action = storage.get_value(&event).await?;
        let mut result = storage.get_value_or_default(&state).await?;
        match &action.payload {
            MyAction::Set(v) => result.value = v.clone(),
            MyAction::Clear => result.value = String::new(),
        }
        Ok(storage.set_value(&result).await?)
    }
}
```

---

## The `#[co]` Macro

The `#[co]` attribute macro (from `co_macros`) auto-derives common traits and optionally generates WASM exports.

### Flags

| Flag | Effect |
|------|--------|
| `#[co]` | Derives: `Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize` |
| `#[co(state)]` | Same as `#[co]` + `Default` + generates WASM `state()` export |
| `#[co(guard)]` | Generates WASM `guard()` export |
| `#[co(state, guard)]` | Both state and guard exports |
| `#[co(repr)]` | Uses `serde_repr` for efficient integer-based enum serialization. Also derives `Copy`. Requires `#[repr(u8)]` on the enum. |
| `#[co(no_default)]` | Skip `Default` derive (use with `state` when state has no sensible default) |
| `#[co(no_derive)]` | Skip all auto-derives (you handle them manually) |

### When to use which

- **State struct**: `#[co(state)]` - this is your root reducer state
- **Action enum**: `#[co]` - the enum of actions the reducer handles
- **Supporting types** (structs/enums used in state or actions): `#[co]`
- **Compact enums** with numeric representation: `#[co(repr)]` + `#[repr(u8)]`
- **Types needing JsonSchema**: add `#[derive(JsonSchema)]` after `#[co]` (the macro doesn't interfere with additional derives)

### Serde Attributes

Use `#[serde(rename = "x")]` on fields/variants to minimize serialized CBOR size. This is important because all state is content-addressed. Existing cores use single-letter renames:

```rust
#[co(state)]
pub struct Board {
    #[serde(rename = "n", default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(rename = "l", default, skip_serializing_if = "CoList::is_empty")]
    pub lists: CoList<List>,
}
```

For action enums, rename variants:
```rust
#[co]
pub enum CounterAction {
    #[serde(rename = "i")]
    Increment(i64),
    #[serde(rename = "d")]
    Decrement(i64),
}
```

---

## The Reducer Trait

Every core must implement `Reducer<A>` for its state type:

```rust
#[allow(async_fn_in_trait)]
pub trait Reducer<A>
where
    Self: Sized,
    A: Clone,
{
    async fn reduce(
        state: OptionLink<Self>,     // current state (None on first action)
        event: Link<ReducerAction<A>>, // the action to apply
        storage: &CoreBlockStorage,    // storage for reading/writing blocks
    ) -> Result<Link<Self>, anyhow::Error>;
}
```

### The Reducer Pattern

Every reducer follows this exact sequence:

```rust
impl Reducer<MyAction> for MyState {
    async fn reduce(
        state: OptionLink<Self>,
        event: Link<ReducerAction<MyAction>>,
        storage: &CoreBlockStorage,
    ) -> Result<Link<Self>, anyhow::Error> {
        // 1. Load the action
        let action = storage.get_value(&event).await?;

        // 2. Load current state (or default if first action)
        let mut result = storage.get_value_or_default(&state).await?;

        // 3. Apply action to state
        match &action.payload {
            // handle each action variant...
        }

        // 4. Store and return new state
        Ok(storage.set_value(&result).await?)
    }
}
```

### ReducerAction<T> Fields

The `ReducerAction<T>` wrapper gives you metadata about who/when/which-core:

```rust
pub struct ReducerAction<T> {
    pub from: Did,      // sender's decentralized identifier
    pub time: Date,     // timestamp (u64)
    pub core: String,   // core name (e.g., "keystore", "board")
    pub payload: T,     // your action enum variant
}
```

Access these via `action.from`, `action.time`, `action.core`, `action.payload`.

---

## The Guard Trait (Optional)

Guards validate whether an action is allowed to be integrated. Only implement this if your core needs authorization logic.

```rust
pub trait Guard {
    async fn verify(
        storage: &CoreBlockStorage,
        guard: String,
        state: Cid,
        heads: BTreeSet<Cid>,
        next_head: Cid,
    ) -> Result<bool, anyhow::Error>;
}
```

To use both Reducer and Guard on the same type: `#[co(state, guard)]`

See the `co` core (`cores/co/src/lib.rs`) for a complete Guard implementation that checks participant access and core existence.

---

## Working with Collections

Cokit provides three async-transactional collection types. They are stored as content-addressed blocks and support concurrent modification.

### CoMap<K, V>

A key-value map backed by an LSM tree.

```rust
// Direct operations (pass storage each time)
state.keys.insert(storage, "key".to_owned(), value).await?;
state.keys.remove(storage, "key".to_owned()).await?;
let val = state.keys.get(storage, &"key".to_owned()).await?; // Option<V>

// Update existing value with closure
state.keys.update(storage, "key".to_owned(), |v| {
    v.name = "new name".to_owned();
}).await?;

// Async update (when closure needs async work)
state.keys.try_update_async(storage, "key".to_owned(), |v| async move {
    // async operations...
    Ok(v)
}).await?;

// Check existence
let exists = state.keys.contains_key(storage, &"key".to_owned()).await?;
```

### CoList<T>

An ordered list with fractional indexing for conflict-free ordering.

```rust
// Open a transaction to work with the list
let mut list_tx = state.lists.open(storage).await?;

// Add items
list_tx.push(item).await?;                    // append
list_tx.insert(after_index, item).await?;      // insert after index

// Access items
let stream = list_tx.stream();                 // async stream of (CoListIndex, T)

// Remove
list_tx.remove(index).await?;

// Update in place
list_tx.set(index, new_value).await?;

// Commit the transaction
state.lists = list_tx.store().await?;
```

### CoSet<T>

A deduplicating set.

```rust
// Direct operations
state.network.insert(storage, item).await?;
state.network.remove(storage, &item).await?;
let exists = state.network.contains(storage, &item).await?;

// Streaming
let stream = state.network.stream(storage);  // async stream
```

### Transaction Pattern for Collections

When modifying nested collections inside a CoMap, open a transaction:

```rust
// Open CoMap transaction
let mut map_tx = state.memberships.open(storage).await?;

// Read from transaction
let item = map_tx.get(&key).await?;

// Modify via transaction
map_tx.insert(key, value).await?;

// Commit and update state
state.memberships = map_tx.store().await?;
```

### LazyTransaction

For cores with multiple collections that may or may not be modified in a single action, use `LazyTransaction` to defer opening until actually needed:

```rust
use co_api::LazyTransaction;

struct MyTransaction<S: BlockStorage + Clone + 'static> {
    storage: S,
    items: LazyTransaction<S, CoMap<String, Item>>,
    list: LazyTransaction<S, CoList<Entry>>,
}

// Create
let mut tx = MyTransaction {
    storage: storage.clone(),
    items: LazyTransaction::new(storage.clone(), state.items.clone()),
    list: LazyTransaction::new(storage.clone(), state.list.clone()),
};

// Use (opens lazily on first access)
tx.items.get().await?;       // read-only access
tx.items.get_mut().await?;   // mutable access

// Commit only modified collections
if tx.items.is_mut_access() {
    state.items = tx.items.get_mut().await?.store().await?;
}
if tx.list.is_mut_access() {
    state.list = tx.list.get_mut().await?.store().await?;
}
```

See the `board` core for a complete LazyTransaction example.

### Indexes

There is no built-in query or indexing system — indexes are maintained by the core developer as part of the state and updated in the reducer alongside the primary data.

A common pattern is a `CoMap` that maps a lookup key to a position or reference in another collection. For example, the `room` core stores events in an ordered `CoList` but also maintains a `CoMap<String, CoListIndex>` index for O(1) lookup by event ID:

```rust
#[co(state)]
pub struct Room {
    /// Ordered events for display
    pub events: CoList<Link<RoomEvent>>,

    /// Index from event_id to CoListIndex — maintained by the reducer
    pub event_index: CoMap<String, CoListIndex>,
}
```

When the reducer inserts a new event into the `CoList`, it also inserts the corresponding entry into the index `CoMap`. When removing, it removes from both. The index is just another piece of state — treat it the same way.

---

## Link Types

- `Link<T>` - Non-null typed reference to a content-addressed block. Wraps a `Cid`.
- `OptionLink<T>` - Nullable typed reference. Wraps `Option<Cid>`. Used for state that may not exist yet (first reducer call).

Read values through storage:
```rust
let value: T = storage.get_value(&link).await?;
let value: T = storage.get_value_or_default(&option_link).await?;
```

Write values:
```rust
let link: Link<T> = storage.set_value(&value).await?;
```

---

## Common Key Types

| Type | Description |
|------|-------------|
| `CoId` | CO unique identifier (String) |
| `Did` | Decentralized identifier (String) |
| `Tags` | Key-value metadata (`BTreeMap<String, TagValue>`) |
| `Cid` | Content identifier (from `cid` crate) |
| `Date` | Timestamp (u64) |
| `Secret` | Sensitive data container |
| `Network` | Network service descriptor |
| `WeakCid` | Cid wrapper for external/non-owned references |
| `IsDefault` | Trait for `skip_serializing_if` checks |

### Tags Operations

```rust
let mut tags = Tags::new();
tags.insert(("key".to_owned(), "value".to_owned().into()));
tags.append(&mut other_tags);
tags.clear(Some(&tags_to_remove));  // remove specific keys
tags.clear(None);                    // clear all
```

---

## Imports Cheat Sheet

Most cores need a subset of these:

```rust
use co_api::{
    // Always needed
    co, BlockStorageExt, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction,
    // Collections (as needed)
    CoMap, CoList, CoListIndex, CoSet, LazyTransaction,
    // Types (as needed)
    CoId, Did, Tags, IsDefault, Cid, Date, Secret, WeakCid, CoReference,
    // Guard (if implementing)
    Guard, SignedEntry,
    // Storage traits (for generic helper functions)
    BlockStorage, StorageError,
    // Serialization (for tests)
    BlockSerializer,
};
```

---

## Helper Function Patterns

For complex cores, extract action handling into helper functions. Two common signatures:

**Simple (mutable state reference):**
```rust
async fn reduce_rename(state: &mut Board, name: String) -> Result<(), anyhow::Error> {
    state.name = name;
    Ok(())
}
```

**Generic over storage (when you need storage access):**
```rust
async fn reduce_set_key<S>(
    storage: &S,
    keys: &mut CoMap<String, Key>,
    key: Key,
) -> Result<(), anyhow::Error>
where
    S: BlockStorage + Clone + 'static,
{
    keys.insert(storage, key.uri.clone(), key).await?;
    Ok(())
}
```

**Important: Use `.boxed().await?` on helper calls in the root reduce function.** Rust async futures can grow very large on the stack — each match arm's future gets inlined into the parent future. For complex cores with many actions, this causes stack overflows. Calling `.boxed()` (from `futures::FutureExt`) heap-allocates the helper's future, keeping the root reduce function's stack small:

```rust
use futures::FutureExt;

async fn reduce(state: &mut MyTransaction, from: Did, action: MyAction) -> Result<(), anyhow::Error> {
    match action {
        MyAction::Insert(a) => reduce_insert(state, from, a).boxed().await,
        MyAction::Update(a) => reduce_update(state, from, a).boxed().await,
        MyAction::Remove(a) => reduce_remove(state, from, a).boxed().await,
    }
}
```

This is **recommended for all cores with extracted helper functions** — not just as a compiler hint, but to prevent runtime stack overflow. See the `names` and `board` cores for examples.

---

## Testing

Cores are normal Rust code and can be tested natively — there is no need to compile to WASM for tests. The reducer is just an async function that takes storage, state, and an action. Use `MemoryBlockStorage` and call the reducer directly.

### Recommended: Native tests

Write a `dispatch` helper that wraps the boilerplate of creating a `ReducerAction`, storing it, and calling the reducer:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use co_api::{BlockStorage, BlockStorageExt, CoreBlockStorage, Date, Reducer, ReducerAction};
    use co_storage::MemoryBlockStorage;

    /// Helper: dispatch an action and return the next state.
    async fn dispatch<S>(
        storage: &S,
        time: &mut Date,
        state: MyState,
        action: impl Into<MyAction>,
    ) -> MyState
    where
        S: BlockStorage + Clone + 'static,
    {
        let action = ReducerAction {
            core: "".to_owned(),
            from: "did:local:test".to_owned(),
            payload: action.into(),
            time: *time,
        };
        *time += 1;
        let action_link = storage.set_value(&action).await.unwrap();
        let state_link = storage.set_value(&state).await.unwrap();
        let next = MyState::reduce(
            state_link.into(),
            action_link,
            &CoreBlockStorage::new(storage.clone(), true),
        )
        .await
        .unwrap();
        storage.get_value(&next).await.unwrap()
    }

    #[tokio::test]
    async fn test_basic() {
        let storage = MemoryBlockStorage::default();
        let mut time = 1;

        let state = MyState::default();
        let state = dispatch(&storage, &mut time, state, MyAction::Set("hello".into())).await;
        assert_eq!(state.value, "hello");

        let state = dispatch(&storage, &mut time, state, MyAction::Clear).await;
        assert_eq!(state.value, "");
    }
}
```

This pattern (from the `rich-text` core) lets you chain multiple actions, inspect intermediate state, and test complex sequences without WASM overhead. The `from` field can be varied to test multi-participant scenarios.

For workspace cores, add `co-storage` to `[dev-dependencies]`. For standalone cores: `cargo add --dev co-storage`.

---

## Action Design

Actions are sorted by the Log and should be **as order-independent as possible**. The more order-independent they are, the better the CRDT handles conflicts between concurrent participants.

Key principles:
- **Keep actions as logical operations** — a "move" should be a single `Move { from, to }` action, not a `Remove` followed by an `Add`. Splitting logical operations into multiple actions creates ordering dependencies that break under CRDT reordering.
- **Each action sees a consistent state** and is applied atomically (all or nothing).
- **Actions must be serializable** into content-addressed blocks (the `#[co]` macro handles this).

---

## Higher-Order Cores and Composition

Existing cores can be composed into new, more complex cores. For example, a "document manager" core could internally use multiple rich-text states — one per document.

The pattern is to pass relevant data to the inner core's types or handle it in your reducer, rather than modifying the original core. This works because cores have a well-specified interface (state + actions + reducer).

### Migrations

A migration (e.g. v1.0 → v2.0) is just another action variant in the reducer. When the core binary is upgraded, a migration action can transform old state into the new schema. See `examples/counter-upgraded` for a concrete example with `MigrateFromV1`.

---

## Building

Build a core to WASM using the `co` CLI:

```sh
co core build
```

Or manually:
```sh
cargo build --features core --target=wasm32-unknown-unknown --release
```

The resulting `.wasm` file will be at `target-wasm/wasm32-unknown-unknown/release/<crate_name>.wasm`.

---

## Checklist for New Cores

1. Create crate with `crate-type = ["lib", "cdylib"]` and `"core"` feature
2. If inside cokit workspace: register in root `Cargo.toml` (members + workspace.dependencies)
3. If inside cokit workspace: add license header to all `.rs` files
4. Define state struct with `#[co(state)]`
5. Define action enum with `#[co]`
6. Design actions as order-independent logical operations (don't split a "move" into "remove" + "add")
7. Use stable identifiers (UUIDs, natural keys) — never counters or state-derived IDs
8. Implement `Reducer<Action> for State`
9. Use `storage.get_value_or_default()` for state, `storage.get_value()` for action
10. Return `storage.set_value(&result).await?` at the end
11. Use `CoMap`/`CoList`/`CoSet` for any growable data (never `HashMap`/`Vec` for unbounded data)
12. Use serde renames on fields/variants for compact serialization
13. Add `skip_serializing_if` for default/empty fields
14. Build with `co core build`

## Reference Files

For detailed API type documentation, see `references/api-types.md`.
For annotated examples from existing cores, see `references/core-examples.md`.
