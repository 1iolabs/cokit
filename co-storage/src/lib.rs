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
	node_reader::node_reader,
	store_file::store_file,
	unixfs::{unixfs_add, unixfs_cat_buffer, unixfs_encode_buffer},
};
pub use storage::{
	encrypted::{EncryptedBlockStorage, EncryptedStorage},
	fs::FsStorage,
	memory::{MemoryBlockStorage, MemoryStorage},
	store_params::StoreParamsBlockStorage,
	sync::{SyncBlockStorage, SyncStorage},
};
pub use types::{
	block::{BlockStat, BlockStorage},
	block_ext::BlockStorageExt,
	mapping::{BlockStorageContentMapping, StorageContentMapping},
	pin::{PinApi, PinKind, PinOptions},
	storage::{Storage, StorageError},
};
