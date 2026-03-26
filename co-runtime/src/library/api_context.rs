// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{co_v1::CoV1Api, RuntimeContext};
use cid::Cid;
use co_api::{sync_api::Context, Block, Storage};
use co_primitives::{from_cbor, ReducerInput};

/// Native api context.
/// This should be only used for testing purposes.
pub struct ApiContext {
	api: CoV1Api,
	action: Cid,
}
impl ApiContext {
	pub fn new(mut api: CoV1Api) -> Self {
		let reducer_input: ReducerInput = from_cbor(&api.context().input).expect("valid ReducerInput in context.input");
		api.context_mut().state = reducer_input.state;
		Self { action: reducer_input.action, api }
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

	fn action(&self) -> Cid {
		self.action
	}

	fn state(&self) -> Option<Cid> {
		*self.api.state()
	}

	fn store_state(&mut self, cid: Cid) {
		self.api.set_state(cid);
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
