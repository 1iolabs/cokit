// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
	co_settings::CoSettings,
	co_state_result::CoStateResult,
	error::{CoError, CoErrorSignal},
};
