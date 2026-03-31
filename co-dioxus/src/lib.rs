// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
