use co_runtime::RuntimePool;

pub struct Runtime {
	pool: RuntimePool,
}
impl Runtime {
	pub fn new() -> Self {
		Self { pool: Default::default() }
	}

	pub fn runtime(&self) -> &RuntimePool {
		&self.pool
	}
}
