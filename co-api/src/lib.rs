// modules
mod co_v1;
mod library;
mod types;

// exports
pub use co_primitives::{
	BlockSerializer, BlockSerializerError, CoMetadata, Date, DefaultNodeSerializer, Did, Link, Linkable, Metadata,
	Node, NodeBuilder, NodeBuilderError, NodeSerializer, ReducerAction, Tag, Tags, TagsPattern, WithCoMetadata,
};
pub use co_v1::{event_cid_read, state_cid_read, state_cid_write, storage_block_get, storage_block_set};
pub use libipld::Cid;
pub use library::{
	reduce::{reduce, reduce_with_context},
	storage_ext::{ResolveError, StorageExt},
};
pub use types::{
	reducer::{Context, Reducer},
	storage::Storage,
};

// types
pub type Block = libipld::Block<libipld::DefaultParams>;
