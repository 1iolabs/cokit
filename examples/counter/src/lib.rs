// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_api::{async_api::Reducer, co, BlockStorageExt, CoreBlockStorage, Link, OptionLink, ReducerAction};

#[co(state)]
pub struct Counter(pub i64);

#[co]
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
impl Reducer<CounterAction> for Counter {
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<CounterAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let event = storage.get_value(&event).await?;
		let current = storage.get_value_or_default(&state).await?;
		let next = match event.payload {
			CounterAction::Increment(value) => Counter(current.0 + value),
			CounterAction::Decrement(value) => Counter(current.0 - value),
			CounterAction::Multiply(value) => Counter(current.0 * value),
			CounterAction::Set(value) => Counter(value),
		};
		Ok(storage.set_value(&next).await?)
	}
}
