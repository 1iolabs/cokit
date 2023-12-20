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
};
pub use storage::{encrypted::EncryptedStorage, fs::FsStorage, memory::MemoryStorage, sync::SyncStorage};
pub use types::{
	pin::{PinApi, PinKind, PinOptions},
	storage::{Storage, StorageError},
};
