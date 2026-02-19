// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{types::guards::Guards, CoreResolver, CoreResolverContext, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::GuardVerifyPayload;
use co_runtime::{GuardReference, RuntimeContext, RuntimePool};
use co_storage::{BlockStorageExt, ExtendedBlockStorage};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CoGuardResolver<C> {
	mapping: HashMap<Cid, GuardReference>,
	next: C,
}
impl<C> CoGuardResolver<C> {
	pub fn new(core_resolver: C) -> Self {
		Self { next: core_resolver, mapping: Guards::default().built_in_native_mapping() }
	}

	pub fn with_mapping(self, mapping: HashMap<Cid, GuardReference>) -> Self {
		Self { next: self.next, mapping }
	}

	fn guard(&self, wasm: Cid) -> GuardReference {
		self.mapping.get(&wasm).cloned().unwrap_or(GuardReference::Wasm(wasm))
	}
}
#[async_trait]
impl<S, C> CoreResolver<S> for CoGuardResolver<C>
where
	S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	C: CoreResolver<S> + Send + Sync + 'static,
{
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &CoreResolverContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		// verify
		if let Some(state) = *state {
			let co_state: co_core_co::Co = storage.get_deserialized(&state).await?;
			for (guard_name, guard) in co_state.guards {
				let heads = context.entry.entry().next.clone();
				let next_head = *context.entry.cid();
				let guard_reference = self.guard(guard.binary);
				let valid = runtime
					.execute_guard(
						storage,
						&guard.binary,
						&guard_reference,
						RuntimeContext::new_payload(&GuardVerifyPayload {
							guard: guard_name.clone(),
							state,
							heads,
							next_head,
						})?,
					)
					.await
					.map_err(|err| {
						CoreResolverError::Middleware(
							anyhow::Error::from(err).context(format!("execute guard: {}", guard_name)),
						)
					})?;
				if !valid {
					return Err(CoreResolverError::Middleware(anyhow::anyhow!(
						"Guard reports invalid head: {}: {}",
						guard_name,
						next_head
					)));
				}
			}
		}

		// next
		let result = self.next.execute(storage, runtime, context, state, action).await?;

		// result
		Ok(result)
	}
}
