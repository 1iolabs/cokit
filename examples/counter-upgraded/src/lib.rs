// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_api::{BlockStorage, BlockStorageExt, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::future::Future;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Counter {
	pub count: i64,
}

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

	/// Migrate/Upgrade state from `examples/counter` to `examples/counter-upgrade`.
	MigrateFromV1,
}

impl Reducer<CounterAction> for Counter {
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<CounterAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let event = storage.get_value(&event).await?;
		match event.payload {
			CounterAction::Increment(i) => {
				update(storage, state, |_, mut state| async move {
					state.count = state.count.saturating_add(i);
					Ok(state)
				})
				.await
			},
			CounterAction::Decrement(i) => {
				update(storage, state, |_, mut state| async move {
					state.count = state.count.saturating_sub(i);
					Ok(state)
				})
				.await
			},
			CounterAction::Multiply(i) => {
				update(storage, state, |_, mut state| async move {
					state.count = state.count.saturating_mul(i);
					Ok(state)
				})
				.await
			},
			CounterAction::Set(i) => {
				update(storage, state, |_, mut state| async move {
					state.count = i;
					Ok(state)
				})
				.await
			},
			CounterAction::MigrateFromV1 => {
				#[derive(Debug, Default, Deserialize)]
				struct CounterV1(i64);
				let state_v1: OptionLink<CounterV1> = state.cid().into();
				update(storage, state_v1, |_, state| async move { Ok(Counter { count: state.0 }) }).await
			},
		}
	}
}

async fn update<S, I, O, F, Fut>(storage: &S, state_link: OptionLink<I>, update: F) -> Result<Link<O>, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
	F: FnOnce(&S, I) -> Fut,
	Fut: Future<Output = Result<O, anyhow::Error>>,
	I: Send + Sync + DeserializeOwned + Default,
	O: Send + Sync + Serialize + Default,
{
	let state = storage.get_value_or_default(&state_link).await?;
	let next_state = update(storage, state).await?;
	let next_state_link = storage.set_value(&next_state).await?;
	Ok(next_state_link)
}

#[cfg(all(feature = "core", target_arch = "wasm32", target_os = "unknown"))]
#[no_mangle]
pub extern "C" fn state(input: *const co_api::RawCid, output: *mut co_api::RawCid) {
	co_api::reduce::<Counter, CounterAction>(unsafe { &*input }, unsafe { &mut *output })
}
