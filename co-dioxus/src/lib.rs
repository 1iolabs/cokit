mod hooks;
mod library;
mod types;

pub use hooks::{
	use_co_api::{use_co_api, CoApi},
	use_co_context::use_co_context,
	use_co_context_provider::use_co_context_provider,
	use_co_error::use_co_error,
	use_co_error_provider::use_co_error_provider,
	use_co_error_signal::use_co_error_signal,
	use_co_selector::use_co_selector,
	use_co_state::use_co_state,
	use_co_storage::use_co_storage,
};
pub use library::co_context::CoContext;
pub use types::{
	co_settings::CoSettings,
	co_state_result::CoStateResult,
	error::{CoError, CoErrorSignal},
};
