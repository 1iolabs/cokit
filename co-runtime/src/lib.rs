// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

// modules
mod library;
mod modules;
mod runtimes;
mod services;
mod types;

// exports
#[cfg(feature = "llvm")]
pub use library::compile::compile_native;
#[cfg(wasmer_backend)]
pub use library::module_description::ModuleDescription;
pub use library::{
	instance::RuntimeInstance,
	pool::{IdleRuntimePool, RuntimePool},
};
pub use modules::co_v1;
#[cfg(wasmer_backend)]
pub use runtimes::create_runtime;
#[cfg(wasmer_backend)]
pub use runtimes::wasmer::{create_runtime_with_engines, WasmerRuntimeKind};
pub use services::runtime::{RuntimeActor, RuntimeHandle};
pub use types::{
	cid_resolver::{
		create_cid_resolver, CidResolver, CidResolverBox, IpldResolver, JoinCidResolver, MultiLayerCidResolver,
		MultiLayerCidResolverResult,
	},
	context::RuntimeContext,
	core::Core,
	execute_error::ExecuteError,
	guard::GuardReference,
};
