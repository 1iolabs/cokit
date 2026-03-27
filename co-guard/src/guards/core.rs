// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{GuardDefinition, GuardError, GuardRejectionMode};
use cid::Cid;
use co_primitives::GuardInput;
use co_runtime::{GuardReference, RuntimeContext, RuntimeHandle};
use co_storage::BlockStorageExt;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone)]
pub struct CoreGuard {
	mapping: HashMap<Cid, GuardReference>,
	mode: GuardRejectionMode,
}
impl CoreGuard {
	pub fn new(mapping: HashMap<Cid, GuardReference>) -> Self {
		Self { mapping, mode: GuardRejectionMode::Skip }
	}

	pub fn with_mode(mut self, mode: GuardRejectionMode) -> Self {
		self.mode = mode;
		self
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

	/// Verify all guards against the given state, heads, and next_head.
	///
	/// Returns `Ok(())` if all guards pass (or rejected but Ignore mode).
	/// Returns `Err(GuardError::Skipped(..))` for Skip mode rejection.
	/// Returns `Err(GuardError::Rejected(..))` for Fail mode rejection.
	pub async fn verify_guards<S>(
		&self,
		runtime: &RuntimeHandle,
		storage: &S,
		guards: &BTreeMap<String, GuardDefinition>,
		state: &Cid,
		heads: &BTreeSet<Cid>,
		next_head: &Cid,
	) -> Result<(), GuardError>
	where
		S: BlockStorageExt + Send + Sync + Clone + 'static,
	{
		for (guard_name, guard) in guards {
			let guard_reference = self.guard(guard.binary);
			let (mut result, valid) = runtime
				.execute_guard(
					storage,
					&guard.binary,
					&guard_reference,
					RuntimeContext::new(&GuardInput {
						guard: guard_name.clone(),
						state: *state,
						heads: heads.clone(),
						next_head: *next_head,
					})
					.map_err(|err| GuardError::Execute(err))?,
				)
				.await
				.map_err(|err| {
					GuardError::Execute(anyhow::Error::from(err).context(format!("execute guard: {}", guard_name)))
				})?;

			if !valid {
				tracing::trace!(
					guard_name,
					?guard,
					?state,
					?heads,
					?next_head,
					mode = ?self.mode,
					result = ?result.result,
					"guard-reject"
				);

				match self.mode {
					GuardRejectionMode::Fail => {
						return Err(GuardError::Rejected(format!(
							"Guard \"{}\" rejected head \"{}\"",
							guard_name, next_head
						)));
					},
					GuardRejectionMode::Skip => {
						let message = format!("Guard \"{}\" rejected head \"{}\"", guard_name, next_head);
						result.result = Some(Err(message.clone()));
						return Err(GuardError::Skipped(message, result));
					},
					GuardRejectionMode::Ignore => {
						tracing::warn!(?guard_name, ?next_head, "guard-reject-ignore");
					},
				};
			}
		}
		Ok(())
	}
}
