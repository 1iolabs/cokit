// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

#[cfg(feature = "frb")]
#[rustfmt::skip]
mod frb_generated;
mod library;
mod types;

// exports
#[cfg(feature = "uniffi")]
pub use library::co_context::co_context_open;
pub use library::{
	co::{Co, CoState},
	co_context::{CoContext, CreateCo, CreateCore},
	co_error::CoError,
	co_settings::CoSettings,
};
pub use types::{
	cid::Cid,
	co_map::CoMap,
	co_set::CoSet,
	identity::CoPrivateIdentity,
	level::CoLogLevel,
	network_settings::CoNetworkSettings,
	storage::{Block, BlockStorage},
	unixfs::unixfs_add_buffer,
};

// uniffi
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

// types
pub type CoCid = Cid;
