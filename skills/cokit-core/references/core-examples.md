# Annotated Core Examples

Real examples from the cokit codebase, ordered from simple to complex. Read these to understand the patterns before building a new core.

---

## 1. Counter (Simplest Core)

**Location:** `examples/counter/src/lib.rs`

A minimal core with a tuple struct state and four actions. No collections, no async complexity.

```rust
use co_api::{co, BlockStorageExt, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction};

// State is a single i64 wrapped in a tuple struct.
// #[co(state)] adds Default (Counter(0)) and generates WASM state() export.
#[co(state)]
pub struct Counter(pub i64);

// Actions use serde rename for compact CBOR serialization.
#[co]
pub enum CounterAction {
    #[serde(rename = "i")]
    Increment(i64),
    #[serde(rename = "d")]
    Decrement(i64),
    #[serde(rename = "m")]
    Multiply(i64),
    #[serde(rename = "s")]
    Set(i64),
}

impl Reducer<CounterAction> for Counter {
    async fn reduce(
        state: OptionLink<Self>,
        event: Link<ReducerAction<CounterAction>>,
        storage: &CoreBlockStorage,
    ) -> Result<Link<Self>, anyhow::Error> {
        let event = storage.get_value(&event).await?;
        let current = storage.get_value_or_default(&state).await?;
        // Pure function: compute next state from current + action
        let next = match event.payload {
            CounterAction::Increment(value) => Counter(current.0 + value),
            CounterAction::Decrement(value) => Counter(current.0 - value),
            CounterAction::Multiply(value) => Counter(current.0 * value),
            CounterAction::Set(value) => Counter(value),
        };
        Ok(storage.set_value(&next).await?)
    }
}
```

**Key takeaways:**
- Simplest possible reducer: load state, compute new state, store it
- Tuple structs work fine as state
- Match on `event.payload` (not `&event.payload`) when the action type is Copy

---

## 2. KeyStore (CoMap Usage)

**Location:** `cores/keystore/src/lib.rs`

Demonstrates `CoMap<K, V>` for key-value storage. Simple insert/remove operations.

```rust
use co_api::{co, BlockStorageExt, CoMap, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction, Tags};
use schemars::JsonSchema;

#[co(state)]
pub struct KeyStore {
    pub keys: CoMap<String, Key>,
}

// Supporting types also use #[co]. JsonSchema is added for API docs.
#[co]
#[derive(JsonSchema)]
pub struct Key {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub secret: Secret,
    pub tags: Tags,
}

#[co]
#[derive(JsonSchema)]
pub enum Secret {
    Password(co_api::Secret),
    PrivateKey(co_api::Secret),
    SharedKey(co_api::Secret),
}

#[co]
pub enum KeyStoreAction {
    Set(Key),
    Remove(String),
}

impl Reducer<KeyStoreAction> for KeyStore {
    async fn reduce(
        state_link: OptionLink<Self>,
        event_link: Link<ReducerAction<KeyStoreAction>>,
        storage: &CoreBlockStorage,
    ) -> Result<Link<Self>, anyhow::Error> {
        let mut state = storage.get_value_or_default(&state_link).await?;
        let action = storage.get_value(&event_link).await?;
        match &action.payload {
            // CoMap.insert takes (storage, key, value)
            KeyStoreAction::Set(i) => {
                state.keys.insert(storage, i.uri.clone(), i.clone()).await?;
            },
            // CoMap.remove takes (storage, key)
            KeyStoreAction::Remove(uri) => {
                state.keys.remove(storage, uri.clone()).await?;
            },
        }
        Ok(storage.set_value(&state).await?)
    }
}
```

**Key takeaways:**
- `CoMap` direct methods pass `storage` each time (no transaction needed for simple ops)
- Supporting types get `#[co]` too (Key, Secret)
- `#[derive(JsonSchema)]` can be added alongside `#[co]` for API documentation

---

## 3. Membership (CoMap with Update, State Machine)

**Location:** `cores/membership/src/lib.rs`

Complex core showing:
- `CoMap.update()` for in-place modifications
- `CoMap.try_update_async()` for async closures
- `#[co(repr)]` for efficient enum serialization
- Helper functions with `.boxed().await?`
- State machine transitions

```rust
// State with a CoMap of memberships
#[co(state)]
pub struct Memberships {
    pub memberships: CoMap<CoId, Membership>,
}

// repr enum — serializes as integer (u8) for compactness
#[co(repr)]
#[non_exhaustive]
#[repr(u8)]
pub enum MembershipState {
    Active = 10,
    Pending = 15,
    Join = 20,
    Invite = 30,
    Inactive = 40,
}

// Complex action enum with shared options pattern
#[co]
pub enum MembershipsAction {
    Join { id: CoId, did: Did, options: MembershipOptions },
    Invited { id: CoId, did: Did, options: MembershipOptions },
    Remove { id: CoId, did: Option<Did> },
    // ... more variants
}

// Reducer delegates to helper functions
impl Reducer<MembershipsAction> for Memberships {
    async fn reduce(
        state_ref: OptionLink<Self>,
        action_ref: Link<ReducerAction<MembershipsAction>>,
        storage: &CoreBlockStorage,
    ) -> Result<Link<Self>, anyhow::Error> {
        let action = storage.get_value(&action_ref).await?;
        let mut result = storage.get_value_or_default(&state_ref).await?;
        match &action.payload {
            // .boxed().await? helps the compiler with async future sizing
            MembershipsAction::Join { id, did, options } => {
                reduce_join(&mut result.memberships, storage, id, did, options)
                    .boxed().await?;
            },
            // ... other variants
        }
        Ok(storage.set_value(&result).await?)
    }
}

// Helper functions take &mut CoMap and &CoreBlockStorage
async fn reduce_join(
    memberships: &mut CoMap<CoId, Membership>,
    storage: &CoreBlockStorage,
    id: &CoId,
    did: &Did,
    options: &MembershipOptions,
) -> Result<(), anyhow::Error> {
    if let Some(existing) = memberships.get(storage, id).await? {
        // CoMap.update() takes a closure to modify in-place
        let did = did.clone();
        let options = options.clone();
        memberships
            .update(storage, id.clone(), move |m| {
                m.did.insert(did, MembershipState::Active);
                apply_options(m, options);
            })
            .await?;
    } else {
        // CoMap.insert() for new entries
        memberships
            .insert(storage, id.clone(), Membership { /* fields */ })
            .await?;
    }
    Ok(())
}
```

**Key takeaways:**
- `CoMap.update()` takes a `FnOnce(&mut V)` closure — captures cloned values (closures must be `'static`)
- `CoMap.try_update_async()` for when the closure needs async operations
- `.boxed().await?` on helper function calls within match arms (required for complex async)
- `#[co(repr)]` + `#[repr(u8)]` for enums that should serialize as integers
- `#[non_exhaustive]` on enums that may grow
- `#[serde(default, skip_serializing_if = "IsDefault::is_default")]` for optional fields with defaults

---

## 4. Board (CoList + CoMap + LazyTransaction)

**Location:** `cores/board/src/lib.rs`

The most architecturally rich core. Demonstrates:
- `CoList<T>` for ordered items (lists, task ordering)
- `LazyTransaction` for efficient multi-collection access
- Custom transaction struct pattern
- Stream filtering for lookups

```rust
#[co(state)]
pub struct Board {
    #[serde(rename = "n", default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(rename = "l", default, skip_serializing_if = "CoList::is_empty")]
    pub lists: CoList<List>,
    #[serde(rename = "t", default, skip_serializing_if = "Tags::is_empty")]
    pub tags: Tags,
    #[serde(rename = "i", default, skip_serializing_if = "CoMap::is_empty")]
    pub tasks: CoMap<TaskId, Task>,
}

// Reducer delegates to a reduce() function with a custom transaction
impl Reducer<BoardAction> for Board {
    async fn reduce(
        state_link: OptionLink<Self>,
        event_link: Link<ReducerAction<BoardAction>>,
        storage: &CoreBlockStorage,
    ) -> Result<Link<Self>, anyhow::Error> {
        let event = storage.get_value(&event_link).await?;
        let mut state = storage.get_value_or_default(&state_link).await?;
        reduce(storage, &mut state, event.payload).await?;
        Ok(storage.set_value(&state).await?)
    }
}

// Custom transaction struct wraps LazyTransaction for each collection
struct BoardTransaction<S: BlockStorage + Clone + 'static> {
    storage: S,
    lists: LazyTransaction<S, CoList<List>>,
    tasks: LazyTransaction<S, CoMap<TaskId, Task>>,
}

async fn reduce<S>(storage: &S, state: &mut Board, action: BoardAction) -> Result<(), anyhow::Error>
where
    S: BlockStorage + Clone + 'static,
{
    // Open lazy transactions — collections only opened when accessed
    let mut transaction = BoardTransaction {
        storage: storage.clone(),
        lists: LazyTransaction::new(storage.clone(), state.lists.clone()),
        tasks: LazyTransaction::new(storage.clone(), state.tasks.clone()),
    };

    // Dispatch to handlers
    match action {
        BoardAction::TaskCreate { list, task, after } => {
            reduce_task_create(&mut transaction, list, task, after).boxed().await?
        },
        // ... other actions
    }

    // Commit only collections that were modified
    if transaction.lists.is_mut_access() {
        state.lists = transaction.lists.get_mut().await?.store().await?;
    }
    if transaction.tasks.is_mut_access() {
        state.tasks = transaction.tasks.get_mut().await?.store().await?;
    }

    Ok(())
}

// Helper methods on the transaction struct for common queries
impl<S: BlockStorage + Clone + 'static> BoardTransaction<S> {
    async fn find_list_by_name(&mut self, name: &str) -> Result<Option<(CoListIndex, List)>, anyhow::Error> {
        Ok(self.lists.get().await?
            .stream()
            .try_filter(|item| ready(item.1.name == name))
            .try_first()
            .await?)
    }
}

// Task creation: uses both lists and tasks collections
async fn reduce_task_create<S: BlockStorage + Clone + 'static>(
    transaction: &mut BoardTransaction<S>,
    list: ListName,
    task: Task,
    after: Option<TaskId>,
) -> Result<(), anyhow::Error> {
    let task_id = task.id.clone();

    // Find the target list
    let (list_index, mut list) = transaction
        .find_list_by_name(&list).await?
        .ok_or(anyhow!("List not found: {}", list))?;

    // Validate uniqueness
    if transaction.tasks.get().await?.contains_key(&task_id).await? {
        return Err(anyhow!("Task exists: {}", task_id));
    }

    // Insert task into CoMap
    transaction.tasks.get_mut().await?.insert(task_id.clone(), task).await?;

    // Insert task ID into list's CoList (with optional positioning)
    let mut list_tasks = list.tasks.open(&transaction.storage).await?;
    if let Some(after_index) = /* find after position */ None {
        list_tasks.insert(after_index, task_id).await?;
    } else {
        list_tasks.push(task_id).await?;
    }
    list.tasks = list_tasks.store().await?;

    // Update the list in the parent CoList
    transaction.lists.get_mut().await?.set(list_index, list).await?;

    Ok(())
}
```

**Key takeaways:**
- `LazyTransaction` pattern: wrap each collection, only open on access, commit only what changed
- Custom transaction struct with helper methods for common queries
- `CoList.open(storage)` returns a `CoListTransaction` for mutation
- `CoList.stream()` / `CoList.stream(storage)` for iteration
- Lists inside lists: `list.tasks` is a `CoList<TaskId>` inside a `CoList<List>`
- Always `store()` inner transactions before `set()`-ing them back into outer collections
- Use `std::future::ready()` with `try_filter` for synchronous predicates

---

## 5. Room (External Types as Actions)

**Location:** `cores/room/src/lib.rs`

Shows that the action type doesn't have to be defined in the core — it can come from an external crate (here `co_messaging::MatrixEvent`).

```rust
use co_messaging::MatrixEvent;

#[co(state)]
#[derive(JsonSchema)]
pub struct Room {
    pub name: String,
    pub events: CoList<Link<RoomEvent>>,
    pub event_index: CoMap<String, CoListIndex>,
    // ...
}

// Action type is MatrixEvent from co_messaging crate
impl Reducer<MatrixEvent> for Room {
    async fn reduce(
        state_link: OptionLink<Self>,
        event_link: Link<ReducerAction<MatrixEvent>>,
        storage: &CoreBlockStorage,
    ) -> Result<Link<Self>, anyhow::Error> {
        let event = storage.get_value(&event_link).await?;
        let mut state = storage.get_value_or_default(&state_link).await?;
        // event_link is stored in state for back-references
        // ...
    }
}
```

**Key takeaways:**
- Action types can be imported from other crates
- `Link<ReducerAction<T>>` can be stored in state to reference the original action
- `CoMap<String, CoListIndex>` pattern for building indexes alongside CoList
- `CoList<Link<T>>` for lists of references to other blocks

---

## Integration Test Pattern

**Location:** `cores/co/tests/integration_test.rs`

```rust
#[tokio::test]
async fn integration_test() {
    // Build the core to WASM
    assert!(Command::new("cargo")
        .args(["build", "--features", "core",
               "--target=wasm32-unknown-unknown",
               "--target-dir", "../../target-wasm", "--release"])
        .status().unwrap().success());

    // Set up in-memory storage
    let storage = MemoryBlockStorage::default();

    // Create action, serialize, store
    let action = ReducerAction {
        core: "".to_owned(),
        payload: CoAction::TagsInsert { tags: tags.clone() },
        from: "did:local:test".to_owned(),
        time: 0,
    };
    let action_block = BlockSerializer::default().serialize(&action).unwrap();
    let action_cid = *action_block.cid();
    storage.set(action_block).await.unwrap();

    // Load WASM, execute through runtime
    let wasm = unixfs_add_file(&storage,
        "../../target-wasm/wasm32-unknown-unknown/release/co_core_co.wasm"
    ).await.unwrap();

    let result = RuntimePool::default()
        .execute_state(&storage, &wasm, &wasm.into(),
            RuntimeContext::new(&ReducerInput { state: None, action: action_cid }).unwrap(),
        ).await.unwrap().state;

    // Deserialize and verify
    let block = storage.get(&result.unwrap()).await.unwrap();
    let state: Co = BlockSerializer::default().deserialize(&block).unwrap();
    assert_eq!(tags, state.tags);
}
```
