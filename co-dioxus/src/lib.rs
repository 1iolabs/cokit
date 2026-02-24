#[cfg(all(feature = "web", feature = "desktop"))]
compile_error!("Features 'web' and 'desktop' cannot be enabled together.");
#[cfg(all(feature = "web", feature = "mobile"))]
compile_error!("Features 'web' and 'mobile' cannot be enabled together.");
#[cfg(all(feature = "desktop", feature = "mobile"))]
compile_error!("Features 'desktop' and 'mobile' cannot be enabled together.");

mod hooks;
mod library;
mod types;

pub use hooks::{
	use_co::{use_co, use_selector, Co},
	use_co_actions::use_co_actions,
	use_co_api::{use_co_api, CoApi},
	use_co_block::{use_co_block, use_co_block_deserialized},
	use_co_context::use_co_context,
	use_co_context_provider::use_co_context_provider,
	use_co_error::use_co_error,
	use_co_error_provider::use_co_error_provider,
	use_co_error_signal::use_co_error_signal,
	use_co_selector::use_co_selector,
	use_co_state::use_co_state,
	use_co_storage::{use_co_storage, CoBlockStorage},
	use_did_key_identity::use_did_key_identity,
};
pub use library::{
	cli::{Cli, CoLogLevel},
	co_context::CoContext,
};
pub use types::{
	co_settings::{CoSettings, CoStorageSetting},
	co_state_result::CoStateResult,
	error::{CoError, CoErrorSignal},
};
