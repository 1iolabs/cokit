use crate::RuntimeContext;
use cid::Cid;
use co_api::{async_api, BlockStorage, CoreBlockStorage};

pub struct AsyncContext {
	storage: CoreBlockStorage,
	context: RuntimeContext,
}
impl AsyncContext {
	pub fn new<S>(storage: S, context: RuntimeContext, checked: bool) -> Self
	where
		S: BlockStorage + Clone + 'static,
	{
		Self { storage: CoreBlockStorage::new(storage, checked), context }
	}

	pub fn context(self) -> RuntimeContext {
		self.context
	}
}
impl async_api::Context for AsyncContext {
	fn storage(&self) -> &CoreBlockStorage {
		&self.storage
	}

	fn payload(&self) -> Vec<u8> {
		self.context.payload.clone()
	}

	fn event(&self) -> Cid {
		self.context.event
	}

	fn state(&self) -> Option<Cid> {
		self.context.state
	}

	fn set_state(&mut self, cid: Cid) {
		self.context.state = Some(cid);
	}

	fn write_diagnostic(&mut self, cid: Cid) {
		self.context.diagnostics.push(cid.into());
	}
}
