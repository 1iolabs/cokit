// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod library;
mod macros;
mod types;

#[cfg(any(test, feature = "benchmarking"))]
pub use library::test::TestStorage;
pub use library::{
	block_diff::{block_diff, block_diff_added_with_parent, BlockDiff, BlockDiffFollow},
	block_links::{BlockLinks, BlockLinksFilter, IgnoreFilter, JoinFilter, WeakCoReferenceFilter},
	block_serializer::{BlockSerializer, BlockSerializerError},
	cbor::{from_cbor, to_cbor, CborError},
	co_try_stream_ext::CoTryStreamExt,
	is_default::IsDefault,
	json::{from_json, from_json_string, to_json, to_json_string, JsonError},
	lsm_tree_map::{LsmTreeMap, LsmTreeMapSettings},
	node_builder::{DefaultNodeSerializer, Node, NodeBuilder, NodeBuilderError, NodeSerializer},
	node_reader::{node_reader, NodeReaderError},
	node_stream::NodeStream,
	reducer_action_core::{reducer_action_core, reducer_action_core_from_storage},
	serde_map_as_list,
	storage::CoreBlockStorage,
	unixfs::{unixfs_add, unixfs_cat_buffer, unixfs_encode_buffer},
	unixfs_stream::unixfs_stream,
};
pub use types::{
	action::ReducerAction,
	any_block_storage::AnyBlockStorage,
	block::{Block, BlockError},
	block_storage::{
		BlockStat, BlockStorage, BlockStorageCloneSettings, BlockStorageStoreParams, CloneWithBlockStorageSettings,
		StorageError,
	},
	block_storage_ext::BlockStorageExt,
	cid::CoCid,
	clock::Clock,
	co::CoId,
	co_date::{CoDate, CoDateRef, DynamicCoDate, MonotonicCoDate, StaticCoDate},
	co_list::{CoList, CoListIndex, CoListTransaction},
	co_map::{CoMap, CoMapTransaction},
	co_reference::CoReference,
	co_set::{CoSet, CoSetTransaction},
	codec::{KnownMultiCodec, MultiCodec, MultiCodecError},
	core_name::CoreName,
	date::Date,
	diagnostic_message::DiagnosticMessage,
	did::Did,
	entry::{Entry, SignedEntry},
	guard::{GuardInput, GuardOutput},
	invite::{CoConnectivity, CoInviteMetadata},
	known_tags::{CoInvite, CoJoin, CoNetwork, CoTimeout, KnownTag, KnownTags},
	lazy_transaction::{LazyTransaction, Transactionable},
	link::{Link, Linkable, OptionLink},
	mapped_cid::{MappedCid, OptionMappedCid},
	metadata::{CoMetadata, Metadata, WithCoMetadata},
	network::{Network, NetworkCoHeads, NetworkDidDiscovery, NetworkPeer, NetworkRendezvous},
	path::{
		AbsolutePath, AbsolutePathOwned, Component, Components, Path, PathError, PathExt, PathOwned, RelativePath,
		RelativePathOwned,
	},
	raw_cid::{cid_to_raw, raw_to_cid, RawCid, CID_MAX_SIZE},
	reducer::{ReducerInput, ReducerOutput},
	secret::Secret,
	storage::Storage,
	store_params::{DefaultParams, StoreParams},
	streamable::Streamable,
	tags::{Tag, TagMatcher, TagPattern, TagValue, Tags, TagsExpr},
	total_float::TotalFloat64,
	weak_cid::WeakCid,
};
