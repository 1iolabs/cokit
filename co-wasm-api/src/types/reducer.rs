use crate::{
	co_v1::{event_cid_read, state_cid_read, state_cid_write},
	library::{read_cid, write_cid, WasmStorage},
	Cid, ReducerAction, Storage,
};

pub trait Reducer {
	type Action: Clone;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &dyn Context) -> Self;
}

pub trait Context {
	fn storage(&self) -> &dyn Storage;

	fn storage_mut(&mut self) -> &mut dyn Storage;

	fn event(&self) -> Cid;

	fn state(&self) -> Option<Cid>;

	fn store_state(&self, cid: &Cid);
}
