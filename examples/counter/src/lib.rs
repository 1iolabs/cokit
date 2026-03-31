// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_api::{co, BlockStorageExt, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction};

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
