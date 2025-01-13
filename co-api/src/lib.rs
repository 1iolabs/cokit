// modules
mod co_v1;
mod library;
mod types;

// exports
pub use cid::Cid;
pub use co_primitives::{
	tags, AbsolutePath, AbsolutePathOwned, Block, BlockSerializer, BlockSerializerError, CoId, CoMetadata, Component,
	Components, Date, DefaultNodeSerializer, DefaultParams, Did, Link, Linkable, Metadata, Network, Node, NodeBuilder,
	NodeBuilderError, NodeSerializer, Path, PathExt, PathOwned, ReducerAction, RelativePath, RelativePathOwned, Secret,
	StoreParams, Tag, Tags, TagsExpr, TotalFloat64, WithCoMetadata,
};
pub use co_v1::{event_cid_read, state_cid_read, state_cid_write, storage_block_get, storage_block_set};
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
