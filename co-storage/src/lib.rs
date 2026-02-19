// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
pub use library::{node_reader::node_reader, unixfs_add_file::unixfs_add_file};
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
