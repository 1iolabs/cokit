# Storage
#todo

## Abstract
One of the base building blocks of CO-kit is the content addressed storage.
The storage is represented as a very simple interface which writes and reads CID/BLOB pairs called Blocks.
The recommended serialization format (also used throughout CO-kit) is DAG-CBOR which is a subset of CBOR with links to CIDs.
A core is not restricted to DAG-CBOR and may use any given structure.

## Layers
Storages can be layered to add functionality.

### Encryption
The encryption layer encrypts blocks before writing them to disk through a configurable encryption algorithm.
The default encryption algorithm is [XChaCha20-Poly1305](https://datatracker.ietf.org/doc/html/rfc8439).

### Network
The network layer will fetch blocks on demand while being used.
It checks the layer before if the block is known by its CID, if not it will be fetched from any CO participant.

## Partial Data
As all data is represented as a graph, more precisely as a DAG (directed acyclic graph) the data is always accessed top-down, meaning we can fetch more data as we walk down the graph.
In addition, content addressing ensures validity of the data.
Distribution happens organically, but you can always opt to fetch all the data if needed.

## API
The rust API looks like this and can be easily implemented for different backends:
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
