// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

// modules
mod co_v1;
mod library;
mod types;

/// exports
pub use cid::Cid;
pub use co_macros::co;
pub use co_primitives::{
	cid_to_raw, from_cbor, from_json, raw_to_cid, reducer_action_core, reducer_action_core_from_storage,
	serde_map_as_list, tags, to_cbor, to_json, to_json_string, AbsolutePath, AbsolutePathOwned, Block, BlockSerializer,
	BlockSerializerError, BlockStorage, BlockStorageExt, Clock, CoId, CoList, CoListIndex, CoListTransaction, CoMap,
	CoMapTransaction, CoMetadata, CoReference, CoSet, CoSetTransaction, CoTryStreamExt, Component, Components,
	CoreBlockStorage, Date, DefaultNodeSerializer, DefaultParams, Did, Entry, GuardInput, GuardOutput, IsDefault,
	LazyTransaction, Link, Linkable, Metadata, Network, Node, NodeBuilder, NodeBuilderError, NodeSerializer,
	OptionLink, Path, PathExt, PathOwned, RawCid, ReducerAction, ReducerInput, ReducerOutput, RelativePath,
	RelativePathOwned, Secret, SignedEntry, Storage, StorageError, StoreParams, Tag, TagMatcher, TagPattern, TagValue,
	Tags, TagsExpr, TotalFloat64, WeakCid, WithCoMetadata, CID_MAX_SIZE,
};
pub use co_v1::{storage_block_get, storage_block_set};
pub use library::{
	guard::{guard, GuardRef},
	reduce::{reduce, ReducerRef},
};
pub use types::{
	guard::Guard,
	reducer::{Context, Reducer},
};
