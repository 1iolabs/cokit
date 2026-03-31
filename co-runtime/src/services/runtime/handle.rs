// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	services::runtime::{ExecuteGuardAction, ExecuteStateAction, RuntimeMessage},
	Core, ExecuteError, GuardReference, RuntimeContext,
};
use cid::Cid;
use co_actor::ActorHandle;
use co_primitives::{AnyBlockStorage, CoreBlockStorage};

#[derive(Debug, Clone)]
pub struct RuntimeHandle {
	pub(crate) handle: ActorHandle<RuntimeMessage>,
}
impl RuntimeHandle {
	pub async fn execute_state(
		&self,
		storage: &impl AnyBlockStorage,
		core_cid: &Cid,
		core: &Core,
		context: RuntimeContext,
	) -> Result<RuntimeContext, ExecuteError> {
		self.handle
			.request(|response| {
				RuntimeMessage::ExecuteState(
					ExecuteStateAction {
						storage: CoreBlockStorage::new(storage.clone(), false),
						core_cid: *core_cid,
						core: core.clone(),
						context,
					},
					response,
				)
			})
			.await
			.map_err(|err| ExecuteError::Other(err.into()))?
	}

	pub async fn execute_guard(
		&self,
		storage: &impl AnyBlockStorage,
		guard_cid: &Cid,
		guard: &GuardReference,
		context: RuntimeContext,
	) -> Result<(RuntimeContext, bool), ExecuteError> {
		self.handle
			.request(|response| {
				RuntimeMessage::ExecuteGuard(
					ExecuteGuardAction {
						storage: CoreBlockStorage::new(storage.clone(), false),
						guard_cid: *guard_cid,
						guard: guard.clone(),
						context,
					},
					response,
				)
			})
			.await
			.map_err(|err| ExecuteError::Other(err.into()))?
	}
}
