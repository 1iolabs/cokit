// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
pub use library::node_reader::node_reader;
#[cfg(feature = "fs")]
pub use library::unixfs_add_file::unixfs_add_file;
#[cfg(feature = "fs")]
pub use storage::fs::FsStorage;
#[cfg(all(feature = "indexeddb", target_arch = "wasm32"))]
pub use storage::indexeddb::IndexedDbBlockStorage;
#[cfg(feature = "overlay")]
pub use storage::overlay::{OverlayBlockStorage, OverlayChange, OverlayChangeReference};
#[cfg(feature = "native")]
pub use storage::sync::{SyncBlockStorage, SyncStorage};
pub use storage::{
	change::{BlockStorageChange, ChangeBlockStorage},
	encrypted::{EncryptedBlockStorage, EncryptedBlockStorageMapping, EncryptionReferenceMode},
	join::JoinBlockStorage,
	links::LinksBlockStorage,
	mapped::MappedBlockStorage,
	memory::{MemoryBlockStorage, MemoryStorage},
	request,
	static_storage::StaticBlockStorage,
	store_params::StoreParamsBlockStorage,
};
pub use types::{
	extended_block_storage::{ExtendedBlock, ExtendedBlockOptions, ExtendedBlockStorage},
	mapping::{BlockStorageContentMapping, StorageContentMapping},
	pin::{PinApi, PinKind, PinOptions},
	storage::Storage,
};
