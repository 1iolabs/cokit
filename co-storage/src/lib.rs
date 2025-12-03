mod crypto;
mod library;
mod storage;
mod types;

// TODO: remove
pub use co_primitives::{BlockStat, BlockStorage, BlockStorageExt, StorageError};
// exports
pub use crypto::{
	block::{Algorithm, AlgorithmError},
	secret::Secret,
};
pub use library::{
	node_reader::node_reader,
	tmp_dir::TmpDir,
	unixfs::{unixfs_add, unixfs_cat_buffer, unixfs_encode_buffer},
	unixfs_add_file::unixfs_add_file,
	unixfs_stream::unixfs_stream,
};
pub use storage::{
	change::{BlockStorageChange, ChangeBlockStorage},
	encrypted::{EncryptedBlockStorage, EncryptedBlockStorageMapping, EncryptionReferenceMode},
	fs::FsStorage,
	join::JoinBlockStorage,
	links::LinksBlockStorage,
	mapped::MappedBlockStorage,
	memory::{MemoryBlockStorage, MemoryStorage},
	overlay::{OverlayBlockStorage, OverlayChange, OverlayChangeReference},
	request,
	static_storage::StaticBlockStorage,
	store_params::StoreParamsBlockStorage,
	sync::{SyncBlockStorage, SyncStorage},
};
pub use types::{
	extended_block_storage::{ExtendedBlock, ExtendedBlockOptions, ExtendedBlockStorage},
	mapping::{BlockStorageContentMapping, StorageContentMapping},
	pin::{PinApi, PinKind, PinOptions},
	storage::Storage,
};
