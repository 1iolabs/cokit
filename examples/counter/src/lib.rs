// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_api::{
	sync_api::{Context, Reducer},
	ReducerAction,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Counter(pub i64);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CounterAction {
	#[serde(rename = "i")]
	Increment(i64),
	#[serde(rename = "d")]
	Decrement(i64),
	#[serde(rename = "m")]
	Multiply(i64),
	#[serde(rename = "s")]
	Set(i64),
}

impl Reducer for Counter {
	type Action = CounterAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &mut dyn Context) -> Self {
		match event.payload {
			CounterAction::Increment(i) => Counter(self.0 + i),
			CounterAction::Decrement(i) => Counter(self.0 - i),
			CounterAction::Multiply(i) => Counter(self.0 * i),
			CounterAction::Set(i) => Counter(i),
		}
	}
}

#[cfg(all(feature = "core", target_arch = "wasm32", target_os = "unknown"))]
#[no_mangle]
pub extern "C" fn state(input: *const co_api::RawCid, output: *mut co_api::RawCid) {
	co_api::sync_api::reduce::<Counter>(unsafe { &*input }, unsafe { &mut *output })
}
