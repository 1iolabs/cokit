// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod application;
mod library;
mod pin;
pub mod reducer;
mod services;
pub mod state;
mod types;

#[cfg(feature = "pinning")]
pub use crate::library::{
	storage_cleanup::storage_cleanup,
	storage_snapshots::storage_snapshots,
	storage_structure::{storage_structure_recursive, CoStructureResolver, StructureResolveResult, StructureResolver},
};
#[cfg(feature = "tracing")]
pub use application::tracing::TracingBuilder;
pub use application::{
	application::{Application, ApplicationBuilder},
	co_context::CoContext,
	local::{LocalCoBuilder, CO_ID_LOCAL},
	reducer::{Reducer, ReducerBuilder, ReducerChangeContext, ReducerChangedHandler},
	runtime::Runtime,
	shared::CreateCo,
	storage::Storage,
};
pub use co_actor::TaskSpawner;
pub use co_core_keystore::{Key, KeyStore, KeyStoreAction};
pub use co_identity::{
	DidKeyIdentity, DidKeyIdentityResolver, Identity, IdentityBox, IdentityResolver, IdentityResolverError,
	PrivateIdentity, PrivateIdentityBox, PrivateIdentityResolver, PrivateIdentityResolverBox,
};
#[cfg(feature = "network")]
pub use co_network::NetworkSettings;
pub use co_primitives::{
	from_cbor, from_json, from_json_string, tag, tags, to_cbor, to_json, to_json_string, unixfs_add, unixfs_cat_buffer,
	unixfs_encode_buffer, unixfs_stream, AbsolutePath, AbsolutePathOwned, AnyBlockStorage, Block, BlockSerializer,
	BlockStat, BlockStorage, BlockStorageExt, CloneWithBlockStorageSettings, CoDate, CoDateRef, CoId, CoInvite, CoList,
	CoListIndex, CoListTransaction, CoMap, CoMapTransaction, CoNetwork, CoSet, CoSetTransaction, CoTryStreamExt,
	Component, Components, CoreName, DagCollection, DagCollectionAsyncExt, DagCollectionExt, Date, DefaultParams, Did,
	DynamicCoDate, IsDefault, KnownMultiCodec, KnownTag, KnownTags, Link, MultiCodec, MultiCodecError, NodeStream,
	OptionLink, Path, PathError, PathExt, PathOwned, ReducerAction, RelativePath, RelativePathOwned, StorageError, Tag,
	Tags,
};
pub use co_runtime::{co_v1, Core, ExecuteError, GuardReference, RuntimeContext, RuntimeInstance, RuntimePool};
pub use co_storage::{BlockStorageContentMapping, MemoryBlockStorage};
#[cfg(feature = "fs")]
pub use library::build_core::{build_core, crate_repository_path, BuildCoreArtifact};
#[cfg(feature = "network")]
pub use library::keystore_fetch::keystore_fetch;
#[cfg(feature = "network")]
pub use library::local_keypair_fetch::local_keypair_fetch;
#[cfg(feature = "network")]
pub use library::token::{CoToken, CoTokenParameters};
#[cfg(feature = "network")]
pub use library::update_co::update_co;
pub use library::{
	core_source::CoreSource,
	did_key_provider::DidKeyProvider,
	find_co_by_pin::find_co_by_pin,
	find_co_identities::{find_co_identities, find_co_private_identity},
	find_co_secret::find_co_secret,
	find_membership::{find_membership, find_memberships},
	generate_random_name::generate_random_name,
	ipld_resolve_recursive::ipld_resolve_recursive,
	is_cid_encrypted::is_cid_encrypted,
	local_secret::{DynamicLocalSecret, LocalSecret, MemoryLocalSecret},
	local_secret_password::PasswordLocalSecret,
	memory_dispatch::MemoryDispatch,
};
pub use pin::PinAPI;
pub use reducer::core_resolver::{
	co::CoCoreResolver, single::SingleCoreResolver, CoreResolver, CoreResolverContext, CoreResolverError,
};
pub use services::{
	application::{Action, ActionError, ApplicationMessage},
	reducer::CoReducer,
};
#[cfg(feature = "js")]
pub use types::js_co_date::JsCoDate;
#[cfg(feature = "native")]
pub use types::system_co_date::SystemCoDate;
pub use types::{
	co_dispatch::{CoDispatch, DynamicCoDispatch},
	co_pinning_key::CoPinningKey,
	co_reducer_context::CoReducerContext,
	co_reducer_factory::{CoReducerFactory, CoReducerFactoryError, CoReducerFactoryResultExt},
	co_reducer_state::{CoReducerState, MappedCoReducerState},
	co_root::CoRoot,
	co_storage::CoStorage,
	co_storage_setting::CoStorageSetting,
	co_uuid::{CoUuid, DynamicCoUuid, MonotonicCoUuid, RandomCoUuid},
	cores::{
		Cores, CO_CORE_CO, CO_CORE_DATA_SERIES, CO_CORE_FILE, CO_CORE_KEYSTORE, CO_CORE_MEMBERSHIP, CO_CORE_NAME_CO,
		CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_CORE_NAME_PIN, CO_CORE_NAME_STORAGE, CO_CORE_PIN,
		CO_CORE_ROOM,
	},
	error::{ErrorContext, ErrorKind, IntoAction},
	guards::Guards,
};
