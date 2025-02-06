use crate::{co_v1::CoV1Api, RuntimeContext};
use cid::Cid;
use co_api::{Context, Storage};

/// Native api context.
/// This should be only used for testing purposes.
pub struct ApiContext {
	api: CoV1Api,
}
impl ApiContext {
	pub fn new(api: CoV1Api) -> Self {
		// let (storage, context) = api.into_inner();
		Self { api }
	}

	pub fn context(&self) -> &RuntimeContext {
		self.api.context()
	}
}
impl Context for ApiContext {
	fn storage(&self) -> &dyn Storage {
		&self.api
	}

	fn storage_mut(&mut self) -> &mut dyn Storage {
		&mut self.api
	}

	fn event(&self) -> Cid {
		*self.api.event()
	}

	fn state(&self) -> Option<Cid> {
		*self.api.state()
	}

	fn store_state(&mut self, cid: Cid) {
		self.api.set_state(cid);
	}

	fn write_diagnostic(&mut self, cid: Cid) {
		self.api.write_diagnostic(cid);
	}
}
