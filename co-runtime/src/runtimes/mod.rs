// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::cfg_wasmer;

// modules
mod runtime;
cfg_wasmer! {
	pub mod wasmer;
}

// export
pub use runtime::{Runtime, RuntimeBox, RuntimeError};
