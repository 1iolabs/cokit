use crate::services::runtime::RuntimeHandle;

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
