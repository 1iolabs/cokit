// modules
mod co_v1;
mod library;
mod types;

// exports
pub use co_primitives::{
	tags, AbsolutePath, AbsolutePathOwned, BlockSerializer, BlockSerializerError, CoId, CoMetadata, Component,
	Components, Date, DefaultNodeSerializer, Did, Link, Linkable, Metadata, Node, NodeBuilder, NodeBuilderError,
	NodeSerializer, Path, PathExt, PathOwned, ReducerAction, RelativePath, RelativePathOwned, Secret, Tag, Tags,
	TagsPattern, TotalFloat64, WithCoMetadata,
};
pub use co_v1::{event_cid_read, state_cid_read, state_cid_write, storage_block_get, storage_block_set};
pub use libipld::Cid;
pub use library::{
	node_reader::NodeReaderError,
	reduce::{reduce, reduce_with_context},
	storage_ext::{StorageError, StorageExt},
};
pub use types::{
	dag_link::{DagCollection, DagMap, DagSet, DagVec},
	reducer::{Context, Reducer},
	storage::Storage,
};

// types
pub type Block = libipld::Block<libipld::DefaultParams>;
