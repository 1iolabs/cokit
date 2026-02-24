// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{co_v1::CoV1Api, RuntimeContext};
use cid::Cid;
use co_api::{sync_api::Context, Block, Storage};

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
		self
	}

	fn storage_mut(&mut self) -> &mut dyn Storage {
		self
	}

	fn payload(&self) -> Vec<u8> {
		self.api.payload().to_vec()
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
impl Storage for ApiContext {
	fn get(&self, cid: &Cid) -> Block {
		co_storage::Storage::get(&self.api, cid).expect("get")
	}

	fn set(&mut self, block: Block) -> Cid {
		co_storage::Storage::set(&mut self.api, block).expect("set")
	}
}
