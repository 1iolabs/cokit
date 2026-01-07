#![allow(unpredictable_function_pointer_comparisons)]

// mods
mod library;
mod types;

// exports
pub use library::{
	co::{storage_get, storage_set, storage_set_data, Co, CoState},
	co_context::{co_context_open, CoContext},
	co_error::CoError,
	co_settings::CoSettings,
};
pub use types::{cid::CoCid, identity::CoPrivateIdentity, level::CoLogLevel, network_settings::CoNetworkSettings};

// uniffi
uniffi::setup_scaffolding!();
