// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	services::runtime::RuntimeHandle, types::guards::Guards, CoreResolver, CoreResolverContext, CoreResolverError,
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{DiagnosticMessage, GuardVerifyPayload};
use co_runtime::{GuardReference, RuntimeContext};
use co_storage::{BlockStorageExt, ExtendedBlockStorage};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CoGuardResolver<C> {
	mapping: HashMap<Cid, GuardReference>,
	next: C,
	mode: GuardRejectionMode,
}
impl<C> CoGuardResolver<C> {
	pub fn new(core_resolver: C, guards: &Guards) -> Self {
		Self { next: core_resolver, mapping: guards.mapping(), mode: GuardRejectionMode::Skip }
	}

	pub fn with_mapping(self, mapping: HashMap<Cid, GuardReference>) -> Self {
		Self { next: self.next, mapping, mode: GuardRejectionMode::Skip }
	}

	pub fn with_ignore_mode(mut self, ignore: bool) -> Self {
		if ignore {
			self.mode = GuardRejectionMode::Ignore;
		} else {
			self.mode = GuardRejectionMode::Skip;
		}
		self
	}

	pub fn with_failure_mode(mut self) -> Self {
		self.mode = GuardRejectionMode::Fail;
		self
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
		runtime: &RuntimeHandle,
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
				let (mut result, valid) = runtime
					.execute_guard(
						storage,
						&guard.binary,
						&guard_reference,
						RuntimeContext::new(Some(state), *action).with_payload(&GuardVerifyPayload {
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
					// diagnostics
					result.resolve_diagnostics(storage).await?;

					// trace
					tracing::trace!(
						co = ?co_state.id,
						guard_name,
						?guard,
						?state,
						heads = ?context.entry.entry().next,
						next_head = ?context.entry.cid(),
						mode = ?self.mode,
						diagnostics = ?result.diagnostics,
						"guard-reject"
					);

					// handle permission failure
					match self.mode {
						// fail
						GuardRejectionMode::Fail => {
							return Err(CoreResolverError::Middleware(anyhow::anyhow!(
								"Guard \"{}\" rejected head \"{}\"",
								guard_name,
								next_head
							)))
						},
						// skip to compute
						GuardRejectionMode::Skip => {
							result.push_diagnostic(DiagnosticMessage::Failure(format!(
								"Guard \"{}\" rejected head \"{}\"",
								guard_name, next_head
							)));
							return Ok(result);
						},
						// warn and ignore
						GuardRejectionMode::Ignore => {
							tracing::warn!(?guard_name, ?next_head, "guard-reject-ignore");
						},
					};
				}
			}
		}

		// next
		let result = self.next.execute(storage, runtime, context, state, action).await?;

		// result
		Ok(result)
	}
}

/// Guard rejection mode.
#[derive(Debug, Clone, Copy)]
enum GuardRejectionMode {
	/// Ignore rejection and just trace a warning.
	Ignore,
	/// Skip the computation and insert a diagnostic message.
	Skip,
	/// Fail the operation hard.
	Fail,
}
