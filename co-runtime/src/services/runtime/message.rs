// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{Core, ExecuteError, GuardReference, RuntimeContext};
use cid::Cid;
use co_actor::Response;
use co_primitives::CoreBlockStorage;

#[derive(Debug)]
pub enum RuntimeMessage {
	ExecuteState(ExecuteStateAction, Response<Result<RuntimeContext, ExecuteError>>),
	ExecuteGuard(ExecuteGuardAction, Response<Result<(RuntimeContext, bool), ExecuteError>>),
}

#[derive(Debug, Clone)]
pub struct ExecuteStateAction {
	pub storage: CoreBlockStorage,
	pub core_cid: Cid,
	pub core: Core,
	pub context: RuntimeContext,
}

#[derive(Debug, Clone)]
pub struct ExecuteGuardAction {
	pub storage: CoreBlockStorage,
	pub guard_cid: Cid,
	pub guard: GuardReference,
	pub context: RuntimeContext,
}
