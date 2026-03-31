// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
