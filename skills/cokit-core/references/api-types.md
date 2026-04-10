# co_api Type Reference

Complete reference for types exported by the `co_api` package. All types below are re-exported from `co_primitives` unless noted.

---

## Core Traits

### `Reducer<A>` (from `co_api::types::reducer`)

```rust
pub trait Reducer<A>
where
    Self: Sized,
    A: Clone,
{
    async fn reduce(
        state: OptionLink<Self>,
        event: Link<ReducerAction<A>>,
        storage: &CoreBlockStorage,
    ) -> Result<Link<Self>, anyhow::Error>;
}
```

### `Guard` (from `co_api::types::guard`)

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

### `Context` (from `co_api::types::reducer`)

Runtime execution context for WASM:
```rust
pub trait Context {
    fn storage(&self) -> &CoreBlockStorage;
    fn payload(&self) -> Vec<u8>;
    fn event(&self) -> Cid;
    fn state(&self) -> Option<Cid>;
    fn set_state(&mut self, cid: Cid);
    fn write_diagnostic(&mut self, cid: Cid);
}
```

---

## Identifier Types

### `CoId`
CO unique identifier. Newtype over `String`.

### `Did`
Decentralized identifier. Newtype over `String`. Format: `"did:local:test"`, `"did:key:z6M..."`.

### `Cid`
Content identifier from the `cid` crate. Used for content-addressed block references.

### `WeakCid`
A Cid wrapper for external/non-owned references. Has `.cid()` method to get inner Cid.

---

## Link Types

### `Link<T>`
Non-null typed reference wrapping a `Cid`. Phantom type `T` tracks what the CID points to.

```rust
// Read value
let value: T = storage.get_value(&link).await?;

// Create from value
let link: Link<T> = storage.set_value(&value).await?;

// Access raw CID
let cid: Cid = link.into();
```

### `OptionLink<T>`
Nullable typed reference wrapping `Option<Cid>`. Used for state parameter (None on first reduce call).

```rust
// Read with default if None
let value: T = storage.get_value_or_default(&option_link).await?;
```

---

## Action Types

### `ReducerAction<T>`

```rust
pub struct ReducerAction<T> {
    pub from: Did,      // sender identity
    pub time: Date,     // timestamp
    pub core: String,   // core name
    pub payload: T,     // action data
}
```

### `ReducerInput`
WASM function input:
```rust
pub struct ReducerInput {
    pub state: Option<Cid>,  // current state CID (None if first)
    pub action: Cid,         // action CID
}
```

### `ReducerOutput`
WASM function output with state CID.

### `GuardInput` / `GuardOutput`
Input/output types for Guard WASM exports.

### `SignedEntry`
Cryptographically signed entry, used in Guard verification:
```rust
pub struct SignedEntry {
    pub identity: Did,      // who signed it
    pub entry: Entry,       // the signed entry data
    // ... signature fields
}
```

---

## Collection Types

### `CoMap<K, V>`
Transactional key-value map backed by LSM tree.

**Direct methods** (require `&storage` each call):
```rust
insert(storage, key, value).await?          -> ()
remove(storage, key).await?                 -> Option<V>
get(storage, &key).await?                   -> Option<V>
contains_key(storage, &key).await?          -> bool
update(storage, key, |v| { ... }).await?    -> ()
try_update_async(storage, key, |v| async { Ok(v) }).await?
```

**Transaction methods**:
```rust
let mut tx: CoMapTransaction = map.open(storage).await?;
tx.get(&key).await?                         -> Option<V>
tx.insert(key, value).await?
tx.remove(key).await?                       -> Option<V>
tx.contains_key(&key).await?                -> bool
tx.stream()                                 -> impl Stream<Item = Result<(K, V)>>
let new_map: CoMap<K,V> = tx.store().await?;
```

**Serialization helpers**:
```rust
CoMap::is_empty(&self) -> bool   // for skip_serializing_if
```

### `CoList<T>`
Order-preserving list with fractional indexing.

**Must use transaction** (open first):
```rust
let mut tx: CoListTransaction = list.open(storage).await?;
tx.push(value).await?                       -> ()
tx.insert(after_index, value).await?        -> ()
tx.remove(index).await?                     -> ()
tx.set(index, value).await?                 -> ()
tx.stream()                                 -> impl Stream<Item = Result<(CoListIndex, T)>>
let new_list: CoList<T> = tx.store().await?;
```

**Without transaction** (read-only stream):
```rust
list.stream(storage)                        -> impl Stream<Item = Result<(CoListIndex, T)>>
```

### `CoListIndex`
Position identifier using rational numbers (numerator/denominator). Used for conflict-free ordering.

### `CoSet<T>`
Deduplicating set.

**Direct methods**:
```rust
insert(storage, value).await?               -> ()
remove(storage, &value).await?              -> ()
contains(storage, &value).await?            -> bool
stream(storage)                             -> impl Stream<Item = Result<T>>
```

**Transaction methods**:
```rust
let mut tx: CoSetTransaction = set.open(storage).await?;
tx.insert(value).await?
tx.remove(&value).await?
let new_set: CoSet<T> = tx.store().await?;
```

### `LazyTransaction<S, C>`
Defers opening a collection transaction until first access. Useful when a reducer has multiple collections but each action only touches some of them.

```rust
let lazy = LazyTransaction::new(storage.clone(), collection.clone());
lazy.get().await?       // read-only access, opens on first call
lazy.get_mut().await?   // mutable access, opens on first call
lazy.is_mut_access()    // true if get_mut was called
```

---

## Tags

### `Tags`
Key-value metadata. Internally `BTreeMap<String, TagValue>`.

```rust
let mut tags = Tags::new();
tags.insert(("key".to_owned(), "value".to_owned().into()));
tags.insert(("num".to_owned(), 42.into()));
tags.append(&mut other_tags);
tags.clear(Some(&keys_to_remove));
tags.clear(None);  // clear all
tags.is_empty() -> bool
```

### `TagValue`
Enum: `String(String)`, `Number(f64)`, `Bool(bool)`.

### `Tag`
A `(String, TagValue)` tuple.

---

## Metadata Types

### `Date`
Timestamp as `u64`.

### `Secret`
Container for sensitive data (passwords, keys). Content is zeroized on drop.

### `Network`
Network service descriptor.

### `CoReference<T>`
Typed reference to another core's data. Can be strong (`Link`) or weak (`WeakCid`).

### `IsDefault` trait
Used for `skip_serializing_if` on fields:
```rust
#[serde(default, skip_serializing_if = "IsDefault::is_default")]
pub lock: Option<String>,
```

---

## Storage Types

### `CoreBlockStorage`
The primary storage interface passed to reducers. Implements `BlockStorage + BlockStorageExt`.

### `BlockStorage` trait
Low-level block access:
```rust
async fn get(&self, cid: &Cid) -> Result<Block>;
async fn set(&self, block: Block) -> Result<()>;
async fn has(&self, cid: &Cid) -> Result<bool>;
```

### `BlockStorageExt` trait
Convenience methods (imported separately):
```rust
async fn get_value<T>(&self, link: &Link<T>) -> Result<T>;
async fn get_value_or_default<T>(&self, link: &OptionLink<T>) -> Result<T>;
async fn set_value<T>(&self, value: &T) -> Result<Link<T>>;
async fn get_deserialized<T>(&self, cid: &Cid) -> Result<T>;
```

### `BlockSerializer`
Serializes values to content-addressed blocks:
```rust
let block = BlockSerializer::default().serialize(&value)?;
let cid = *block.cid();
let value: T = BlockSerializer::default().deserialize(&block)?;
```

### `Block`
A content-addressed block with CID and data.

### `StorageError`
Error type for storage operations.

---

## Serialization Functions

```rust
to_cbor(&value) -> Result<Vec<u8>>          // CBOR serialization
from_cbor::<T>(&bytes) -> Result<T>         // CBOR deserialization
to_json(&value) -> Result<Vec<u8>>          // JSON serialization
from_json::<T>(&bytes) -> Result<T>         // JSON deserialization
to_json_string(&value) -> Result<String>    // JSON as String
```

---

## Path Types

For file-system-like cores:

- `AbsolutePath` / `AbsolutePathOwned` - Absolute paths
- `RelativePath` / `RelativePathOwned` - Relative paths
- `Component` / `Components` - Path component iteration
- `PathExt` trait - Path utilities

---

## Stream Utilities

### `CoTryStreamExt`
Extension trait for working with async streams from collections:
```rust
stream.try_first().await?  // get first item or None
```

Used with `futures::TryStreamExt`:
```rust
use futures::TryStreamExt;
stream.try_filter(|item| ready(item.name == target)).try_first().await?
```

---

## WASM Library Functions

These are used internally by the `#[co(state)]` and `#[co(guard)]` macros to generate WASM exports. You typically don't call them directly:

```rust
co_api::reduce::<R, A>(input: &RawCid, output: &mut RawCid)  // state export
co_api::guard::<R>(input: &RawCid, output: &mut RawCid)       // guard export
```

`ReducerRef` and `GuardRef` provide runtime wrappers:
```rust
ReducerRef::execute_blocking()  // sync WASM context
ReducerRef::execute_async()     // native async context
```
