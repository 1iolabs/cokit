use super::{storage::Storage, Date, Did};

pub trait Reducer
where
	Self: Sized + Clone,
{
	type Action: Clone;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &Context) -> Self;
}

#[derive(Debug, Clone)]
pub struct ReducerAction<T>
where
	T: Clone,
{
	pub from: Did,
	pub time: Date,
	pub payload: T,
}

pub struct Context {
	pub storage: Box<dyn Storage>,
}
