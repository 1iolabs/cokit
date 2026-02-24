// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod library;
mod modules;
mod runtimes;
mod types;

#[cfg(feature = "llvm")]
pub use library::compile::compile_native;
pub use library::{
	api_context::ApiContext,
	async_context::AsyncContext,
	instance::RuntimeInstance,
	module_description::ModuleDescription,
	pool::{ExecuteError, IdleRuntimePool, RuntimePool},
};
pub use modules::co_v1;
pub use runtimes::create_runtime;
pub use types::{
	cid_resolver::{
		create_cid_resolver, CidResolver, CidResolverBox, IpldResolver, JoinCidResolver, MultiLayerCidResolver,
		MultiLayerCidResolverResult,
	},
	context::RuntimeContext,
	core::Core,
	guard::GuardReference,
};
