use co_runtime::RuntimePool;

#[derive(Clone)]
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
