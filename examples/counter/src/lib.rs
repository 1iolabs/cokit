use co_wasm_api::{reduce, Context, Reducer, ReducerAction};
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

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &Context) -> Self {
		match event.payload {
			CounterAction::Increment(i) => Counter(self.0 + i),
			CounterAction::Decrement(i) => Counter(self.0 - i),
		}
	}
}

#[no_mangle]
pub extern "C" fn execute() {
	reduce::<Counter>()
}
