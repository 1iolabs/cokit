// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod hooks;
mod library;
mod types;

pub use hooks::{
	use_co::{use_co, Co},
	use_co_context::use_co_context,
	use_co_id::use_co_id,
	use_co_reducer_state::use_co_reducer_state,
	use_did_key_identity::use_did_key_identity,
	use_selector::{use_selector, use_selector_state},
};
pub use library::{
	cli::{Cli, CoLogLevel},
	co_block_storage::CoBlockStorage,
	co_context::CoContext,
};
pub use types::{
	co_settings::{CoSettings, CoStorageSetting},
	error::CoError,
};
