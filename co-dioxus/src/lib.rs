// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

// fail with proper error message when try to us js for non wasm32
#[cfg(all(feature = "js", not(target_arch = "wasm32"), not(clippy)))]
compile_error!("feature \"js\" can only used for \"wasm32-unknown-unknown\" target");

// modules
mod hooks;
mod library;
mod types;

// exports
pub use hooks::{
	use_co::{use_co, Co},
	use_co_context::use_co_context,
	use_co_id::use_co_id,
	use_co_reducer_state::use_co_reducer_state,
	use_cos::{use_cos, Cos},
	use_did_key_identity::use_did_key_identity,
	use_selector::{use_selector, use_selector_state},
	use_selectors::{use_selector_states, use_selectors, CoSelector, CoSelectorState},
};
pub use library::{
	cli::{Cli, CoLogLevel},
	co_block_storage::CoBlockStorage,
	co_context::{CoContext, CoContextError},
};
pub use types::{
	co_settings::{CoLog, CoSettings},
	error::CoError,
};
