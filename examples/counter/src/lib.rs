use co_wasm_api::{reduce, Context, Reducer, ReducerAction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Counter(pub i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CounterAction {
	#[serde(rename = "i")]
	Increment(i64),
	#[serde(rename = "d")]
	Decrement(i64),
}

impl Reducer for Counter {
	type Action = CounterAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &mut dyn Context) -> Self {
		match event.payload {
			CounterAction::Increment(i) => Counter(self.0 + i),
			CounterAction::Decrement(i) => Counter(self.0 - i),
		}
	}
}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<Counter>()
}
