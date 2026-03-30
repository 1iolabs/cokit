// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

// fail with proper error message when try to us js for non wasm32
#[cfg(all(feature = "js", not(target_arch = "wasm32"), not(clippy), not(test)))]
compile_error!("feature \"js\" can only used for \"wasm32-unknown-unknown\" target");

// modules
mod library;
mod macros;
mod modules;
mod runtimes;
mod services;
mod types;

// exports
#[cfg(feature = "llvm")]
pub use library::compile::compile_native;
pub(crate) use macros::cfg_wasmer;
pub use modules::co_v1;
pub use services::runtime::RuntimeHandle;
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
cfg_wasmer! {
	pub use services::runtime::RuntimeActor;
	pub use library::{
		instance::RuntimeInstance,
		pool::{IdleRuntimePool, RuntimePool},
	};
	pub use runtimes::wasmer::{create_runtime, create_runtime_with_engines, WasmerRuntimeKind};
	pub use library::module_description::ModuleDescription;
}
