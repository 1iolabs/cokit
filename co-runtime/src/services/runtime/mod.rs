// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::cfg_wasmer;

mod handle;
mod message;
cfg_wasmer! {
	mod actor;
	pub use actor::RuntimeActor;
}

pub use handle::RuntimeHandle;
pub use message::{ExecuteGuardAction, ExecuteStateAction, RuntimeMessage};
