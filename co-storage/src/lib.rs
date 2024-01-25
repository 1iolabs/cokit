mod crypto;
mod library;
mod storage;
mod types;

// exports
pub use crypto::{
	block::{Algorithm, AlgorithmError},
	secret::Secret,
};
pub use library::{
	block_serializer::{BlockSerializer, SerializeError},
	node_builder::{DefaultNodeSerializer, Node, NodeBuilder, NodeBuilderError, NodeSerializer},
	node_reader::node_reader,
	unixfs::{unixfs_add, unixfs_cat_buffer},
};
pub use storage::{
	encrypted::{EncryptedBlockStorage, EncryptedStorage},
	fs::FsStorage,
	memory::{MemoryBlockStorage, MemoryStorage},
	sync::{SyncBlockStorage, SyncStorage},
};
pub use types::{
	block::{BlockStat, BlockStorage},
	pin::{PinApi, PinKind, PinOptions},
	storage::{Storage, StorageError},
};
