// modules
mod co_v1;
mod library;
mod types;

// exports
pub use co_primitives::{
	BlockSerializer, BlockSerializerError, CoMetadata, Date, Did, Link, Linkable, Metadata, ReducerAction, Tag, Tags,
	WithCoMetadata,
};
pub use co_v1::{event_cid_read, state_cid_read, state_cid_write, storage_block_get, storage_block_set};
pub use libipld::Cid;
pub use library::{reduce, ResolveError, StorageExt};
pub use types::{
	reducer::{Context, Reducer},
	storage::Storage,
};

// types
pub type Block = libipld::Block<libipld::DefaultParams>;
