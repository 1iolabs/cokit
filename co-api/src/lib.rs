// modules
mod co_v1;
mod library;
mod types;

/// exports
pub use cid::Cid;
pub use co_macros::{co_data, co_state};
pub use co_primitives::{
	from_cbor, from_json, tags, to_cbor, to_json, to_json_string, AbsolutePath, AbsolutePathOwned, Block,
	BlockSerializer, BlockSerializerError, BlockStorage, BlockStorageExt, CoId, CoList, CoListIndex, CoListTransaction,
	CoMap, CoMapTransaction, CoMetadata, CoReference, CoSet, CoSetTransaction, CoTryStreamExt, Component, Components,
	DagCollection, DagCollectionExt, DagMap, DagMapExt, DagSet, DagSetExt, DagVec, DagVecExt, Date,
	DefaultNodeSerializer, DefaultParams, Did, IsDefault, LazyTransaction, Link, Linkable, Metadata, Network, Node,
	NodeBuilder, NodeBuilderError, NodeSerializer, OptionLink, Path, PathExt, PathOwned, ReducerAction, RelativePath,
	RelativePathOwned, Secret, Storage, StorageError, StoreParams, Tag, Tags, TagsExpr, TotalFloat64, WeakCid,
	WithCoMetadata,
};
pub use co_v1::{
	diagnostic_cid_write, event_cid_read, state_cid_read, state_cid_write, storage_block_get, storage_block_set,
};
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
