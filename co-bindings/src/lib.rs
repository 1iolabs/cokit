// mods
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
#[cfg(feature = "network")]
pub use types::network_settings::CoNetworkSettings;
pub use types::{
	cid::Cid,
	co_map::CoMap,
	co_set::CoSet,
	identity::CoPrivateIdentity,
	level::CoLogLevel,
	storage::{Block, BlockStorage},
	unixfs::unixfs_add_buffer,
};

// uniffi
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

// types
pub type CoCid = Cid;
