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
	from_serialized_block::from_serialized_block,
	to_serialized_block::{to_serialized_block, SerializeOptions},
};
pub use storage::{encrypted::EncryptedStorage, memory::MemoryStorage, sync::SyncStorage};
pub use types::storage::{Storage, StorageError};
