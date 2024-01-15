use crate::{Cid, ReducerAction, Storage};

pub trait Reducer {
	type Action: Clone;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self;
}

pub trait Context {
	fn storage(&self) -> &dyn Storage;

	fn storage_mut(&mut self) -> &mut dyn Storage;

	fn event(&self) -> Cid;

	fn state(&self) -> Option<Cid>;

	fn store_state(&self, cid: &Cid);
}
