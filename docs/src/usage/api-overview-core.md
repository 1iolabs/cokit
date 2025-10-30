# API Overview Core (co-api)

#todo #review

The [`co-api`](/crate/co_api/index.html) is the foundation package to create CO-kit [cores](../reference/core.md).
It re-exports [`co-primitives`](/crate/co_primitives/index.html) used to implement cores.

## `Reducer`

The reducer trait.
Normally implemented on the root state of the core.
Its purpose is to apply actions to the current state.

Minimal usage example:

```rust
use co_api::{async_api::Reducer, co, BlockStorage, BlockStorageExt, Link, OptionLink, ReducerAction};

#[co]
pub enum MyAction {}

#[co(state)]
pub struct MyState {}
impl<S> Reducer<MyAction, S> for MyState
where
	S: BlockStorage + Clone + 'static,
{
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<TodoAction>>,
		storage: &S,
	) -> Result<Link<Self>, anyhow::Error> {
			let event = storage.get_value(&event_link).await?;
			let mut state = storage.get_value_or_default(&state_link).await?;
			// ...
			Ok(storage.set_value(&state).await?)
	}
}
```

For further information see:
- [Core Quick Start](../getting-started/rust-core-quick-start.md)
- [Core](../reference/core.md)

## `Guard`

The guard trait.
Verifies if `next_head` are allowed to be integrated into state.

Minimal usage example:

```rust
use cid::Cid;
use co_api::{co, Guard, BlockStorage};
use std::collections::{BTreeSet};

#[co(guard)]
struct MyGuard {}
impl<S: BlockStorage + Clone + 'static> Guard<S> for MyGuard {
	async fn verify(
		storage: &S,
		guard: String,
		state: Cid,
		heads: BTreeSet<Cid>,
		next_head: Cid,
	) -> Result<bool, anyhow::Error> {
	    Ok(true)
	}
}
```

For further information see:
- [Guard](../reference/guard.md)
- [co-core-co: Co](/crate/co_core_co/struct.Co.html#impl-Guard<S>-for-Co)

## Core

### Imports

The `co_v1` API imports.

BlockStorage:
```rust
extern "C" fn storage_block_get(cid: *const u8, cid_size: u32, buffer: *mut u8, buffer_size: u32) -> u32;
extern "C" fn storage_block_set(cid: *const u8, cid_size: u32, buffer: *const u8, buffer_size: u32) -> u32;
```

Reducer:
```rust
extern "C" fn state_cid_read(buffer: *mut u8, buffer_size: u32) -> u32;
extern "C" fn state_cid_write(buffer: *const u8, buffer_size: u32) -> u32;
extern "C" fn event_cid_read(buffer: *mut u8, buffer_size: u32) -> u32;
```

Guard:
```rust
extern "C" fn payload_read(buffer: *mut u8, buffer_size: u32, offset: u32) -> u32;
```

Diagnostics:
```rust
extern "C" fn diagnostic_cid_write(buffer: *const u8, buffer_size: u32) -> u32;
```

### Exports

The WASM Binary for reducers must export following functions:

```rust
extern "C" fn state();
```

The WASM Binary for guards must export following functions:

```rust
extern "C" fn guard();
```

When `co-api` is used this export is handled by the `co` macro.
A single binary may be export both.

## References
- [Core](../reference/core.md)
- [Guard](../reference/guard.md)
- [co-api](/crate/co_api/index.html)
- [Glossary: WASM](../glossary/glossary.md#wasm)
