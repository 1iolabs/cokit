// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

mod co_v1;
mod library;
mod types;

/// exports
pub use cid::Cid;
pub use co_macros::{co, co_data, co_state};
pub use co_primitives::{
	from_cbor, from_json, reducer_action_core, reducer_action_core_from_storage, serde_map_as_list, tags, to_cbor,
	to_json, to_json_string, AbsolutePath, AbsolutePathOwned, Block, BlockSerializer, BlockSerializerError,
	BlockStorage, BlockStorageExt, Clock, CoId, CoList, CoListIndex, CoListTransaction, CoMap, CoMapTransaction,
	CoMetadata, CoReference, CoSet, CoSetTransaction, CoTryStreamExt, Component, Components, DagCollection,
	DagCollectionExt, DagMap, DagMapExt, DagSet, DagSetExt, DagVec, DagVecExt, Date, DefaultNodeSerializer,
	DefaultParams, Did, Entry, IsDefault, LazyTransaction, Link, Linkable, Metadata, Network, Node, NodeBuilder,
	NodeBuilderError, NodeSerializer, OptionLink, Path, PathExt, PathOwned, ReducerAction, RelativePath,
	RelativePathOwned, Secret, SignedEntry, Storage, StorageError, StoreParams, Tag, TagMatcher, TagPattern, TagValue,
	Tags, TagsExpr, TotalFloat64, WeakCid, WithCoMetadata,
};
pub use co_v1::{
	diagnostic_cid_write, event_cid_read, state_cid_read, state_cid_write, storage_block_get, storage_block_set,
};
pub use library::guard::{guard, guard_with_context};
pub use types::{guard::Guard, storage::CoreBlockStorage};

// sync export
pub mod sync_api {
	pub use crate::{
		library::{
			reduce::{reduce, reduce_with_context},
			storage_ext::StorageExt,
		},
		types::reducer::{Context, Reducer},
	};
}

// async export
pub mod async_api {
	pub use crate::{
		library::reduce::async_reduce::{reduce, reduce_execute_with_context, reduce_with_context},
		types::reducer::async_reducer::{Context, Reducer},
	};
}
