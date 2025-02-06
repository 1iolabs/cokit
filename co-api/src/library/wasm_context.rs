use super::{read_cid::read_cid, wasm_storage::WasmStorage, write_cid::write_cid};
use crate::{async_api, diagnostic_cid_write, event_cid_read, state_cid_read, state_cid_write, Cid, Context, Storage};

pub struct WasmContext {
	storage: WasmStorage,
}
impl WasmContext {
	pub fn new() -> Self {
		Self { storage: WasmStorage::new() }
	}
}
impl Context for WasmContext {
	fn storage(&self) -> &dyn Storage {
		&self.storage
	}

	fn storage_mut(&mut self) -> &mut dyn Storage {
		&mut self.storage
	}

	fn event(&self) -> Cid {
		read_cid(event_cid_read).expect("event")
	}

	fn state(&self) -> Option<Cid> {
		read_cid(state_cid_read)
	}

	fn store_state(&mut self, cid: Cid) {
		write_cid(state_cid_write, &cid);
	}

	fn write_diagnostic(&mut self, cid: Cid) {
		write_cid(diagnostic_cid_write, &cid);
	}
}
impl async_api::Context<WasmStorage> for WasmContext {
	fn storage(&self) -> &WasmStorage {
		&self.storage
	}

	fn event(&self) -> Cid {
		<Self as Context>::event(self)
	}

	fn state(&self) -> Option<Cid> {
		<Self as Context>::state(self)
	}

	fn set_state(&mut self, cid: Cid) {
		<Self as Context>::store_state(self, cid)
	}

	fn write_diagnostic(&mut self, cid: Cid) {
		<Self as Context>::write_diagnostic(self, cid)
	}
}
