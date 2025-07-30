# Storage
#todo

## Abstract
One base building block is the content addressed storage.
This is a very simple interface which stores and receives CID, BLOB pairs.
The recommended serialization format (also used internally) is DAG-CBOR which is a subset of CBOR with links to CIDs. But a core is not restricted to DAG-CBOR any may use any structure.

## Encryption
Storages can be layered and the encryption is just another layer which encrypts blocks before writing them to disk.

## API
The rust API looks like this and can be very easily implemented for different backends:
```rust
#[async_trait]
pub trait BlockStorage: Send + Sync {
	type StoreParams: StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError>;

	/// Inserts a block into storage.
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError>;

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError>;

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError>;
}
```

The built in backends are filesystem and memory.

See: [BlockStorage](/crate/co-primitives/types/block_storage.html).
