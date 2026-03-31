// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#[cfg(feature = "llvm")]
pub mod compile;
#[cfg(feature = "js")]
pub mod deferred_storage;
crate::cfg_wasmer! {
	pub mod instance;
	pub mod module_description;
	pub mod pool;
}
