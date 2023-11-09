use crate::{
	co_v1,
	library::{read_cid, write_cid, StorageApi},
	Cid, Date, Did, Storage,
};
use serde::{Deserialize, Serialize};

pub trait Reducer {
	type Action: Clone;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &Context) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReducerAction<T> {
	pub from: Did,
	pub time: Date,
	pub payload: T,
}

pub struct Context {
	storage: StorageApi,
}

impl Context {
	pub fn new() -> Self {
		Self { storage: StorageApi::new() }
	}

	pub fn storage(&self) -> &dyn Storage {
		&self.storage
	}

	pub fn storage_mut(&mut self) -> &mut dyn Storage {
		&mut self.storage
	}

	pub fn event(&self) -> Cid {
		read_cid(co_v1::event_cid_read)
	}

	pub fn state(&self) -> Cid {
		read_cid(co_v1::state_cid_read)
	}

	pub fn store_state(&self, cid: &Cid) {
		write_cid(co_v1::state_cid_write, cid);
	}
}
