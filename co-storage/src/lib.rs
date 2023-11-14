mod crypto;
#[cfg(test)]
mod example;
mod library;
mod storage;
mod types;

// exports
pub use library::{
	from_serialized_block::from_serialized_block,
	to_serialized_block::{to_serialized_block, SerializeOptions},
};
pub use storage::{encrypted::EncryptedStorage, memory::MemoryStorage, sync::SyncStorage};
pub use types::{
	cid::{Link, ResolveError},
	storage::{Storage, StorageError},
	Date, Did,
};
