use crate::{reduce, Reducer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Counter(i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CounterAction {
	Increment(i64),
	Decrement(i64),
}

impl Reducer for Counter {
	type Action = CounterAction;

	fn reduce(self, event: &crate::ReducerAction<Self::Action>, _: &crate::Context) -> Self {
		match event.payload {
			CounterAction::Increment(i) => Counter(self.0 + i),
			CounterAction::Decrement(i) => Counter(self.0 - i),
		}
	}
}

#[no_mangle]
pub extern "C" fn main() {
	reduce::<Counter>()
}
