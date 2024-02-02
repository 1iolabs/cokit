use super::{read_cid::read_cid, wasm_storage::WasmStorage, write_cid::write_cid};
use crate::{event_cid_read, state_cid_read, state_cid_write, Cid, Context, Storage};

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
}
