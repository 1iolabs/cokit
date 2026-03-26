// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::wasm_storage::WasmStorage;
use crate::{sync_api::Context, Cid, Storage};
use co_primitives::ReducerInput;

pub struct WasmContext {
	storage: WasmStorage,
	state: Option<Cid>,
	event: Cid,
}
impl WasmContext {
	pub fn from_reducer_input(input: ReducerInput) -> Self {
		let storage = WasmStorage::new();
		Self { storage, state: input.state, event: input.action }
	}
}
impl Context for WasmContext {
	fn storage(&self) -> &dyn Storage {
		&self.storage
	}

	fn storage_mut(&mut self) -> &mut dyn Storage {
		&mut self.storage
	}

	fn action(&self) -> Cid {
		self.event
	}

	fn state(&self) -> Option<Cid> {
		self.state
	}

	fn store_state(&mut self, cid: Cid) {
		self.state = Some(cid);
	}
}
