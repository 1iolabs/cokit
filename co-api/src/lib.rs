// modules
mod co_v1;
mod library;
mod types;

// exports
pub use cid::Cid;
pub use co_primitives::{
	tags, AbsolutePath, AbsolutePathOwned, Block, BlockSerializer, BlockSerializerError, BlockStorage, BlockStorageExt,
	CoId, CoMap, CoMapTransaction, CoMetadata, Component, Components, DagCollection, DagCollectionExt, DagMap,
	DagMapExt, DagSet, DagSetExt, DagVec, DagVecExt, Date, DefaultNodeSerializer, DefaultParams, Did, Link, Linkable,
	Metadata, Network, Node, NodeBuilder, NodeBuilderError, NodeSerializer, OptionLink, Path, PathExt, PathOwned,
	ReducerAction, RelativePath, RelativePathOwned, Secret, Storage, StorageError, StoreParams, Tag, Tags, TagsExpr,
	TotalFloat64, WithCoMetadata,
};
pub use co_v1::{event_cid_read, state_cid_read, state_cid_write, storage_block_get, storage_block_set};
pub use library::{
	reduce::{reduce, reduce_with_context},
	storage_ext::StorageExt,
};
pub use types::reducer::{Context, Reducer};

// async export
pub mod async_api {
	pub use crate::{
		library::reduce::async_reduce::{reduce, reduce_execute_with_context, reduce_with_context},
		types::reducer::async_reducer::{Context, Reducer},
	};
}
