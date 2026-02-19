// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_runtime::RuntimePool;

#[derive(Debug, Clone)]
pub struct Runtime {
	pool: RuntimePool,
}
impl Default for Runtime {
	fn default() -> Self {
		Self::new()
	}
}

impl Runtime {
	pub fn new() -> Self {
		Self { pool: Default::default() }
	}

	pub fn runtime(&self) -> &RuntimePool {
		&self.pool
	}
}
