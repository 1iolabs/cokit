// mods
#[cfg(feature = "frb")]
mod frb_generated;
mod library;
mod types;

// exports
#[cfg(feature = "uniffi")]
pub use library::{
	co::{storage_get, storage_set, storage_set_data},
	co_context::co_context_open,
};
pub use library::{
	co::{Co, CoState},
	co_context::CoContext,
	co_error::CoError,
	co_settings::CoSettings,
};
pub use types::{
	cid::Cid,
	identity::CoPrivateIdentity,
	level::CoLogLevel,
	network_settings::CoNetworkSettings,
	storage::{Block, BlockStorage},
};

// uniffi
#[cfg(feature = "uniffi")]
#[allow(unpredictable_function_pointer_comparisons)]
uniffi::setup_scaffolding!();

// types
pub type CoCid = Cid;
