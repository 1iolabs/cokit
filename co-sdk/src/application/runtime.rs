// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_runtime::RuntimeHandle;

#[derive(Debug, Clone)]
pub struct Runtime {
	handle: RuntimeHandle,
}
impl Runtime {
	pub fn new(handle: RuntimeHandle) -> Self {
		Self { handle }
	}

	pub fn runtime(&self) -> &RuntimeHandle {
		&self.handle
	}
}
