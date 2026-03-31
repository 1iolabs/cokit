// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_api::{co, BlockStorageExt, CoMap, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction, Tags};
use schemars::JsonSchema;

/// Key Store.
///
/// This COre should only be used in encrypted COs.
#[co(state)]
pub struct KeyStore {
	/// Keys by URI.
	pub keys: CoMap<String, Key>,
}

#[co]
#[derive(JsonSchema)]
pub struct Key {
	/// URI which uniquely identifies this key.
	pub uri: String,

	/// Key Name. Usually the service name.
	pub name: String,

	/// Key description.
	pub description: String,

	/// The secret.
	pub secret: Secret,

	/// Optional tags.
	pub tags: Tags,
}

#[co]
#[derive(JsonSchema)]
pub enum Secret {
	Password(co_api::Secret),
	PrivateKey(co_api::Secret),
	SharedKey(co_api::Secret),
}

#[co]
pub enum KeyStoreAction {
	Set(Key),
	Remove(String),
}

impl Reducer<KeyStoreAction> for KeyStore {
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<KeyStoreAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let mut state = storage.get_value_or_default(&state_link).await?;
		let action = storage.get_value(&event_link).await?;
		match &action.payload {
			KeyStoreAction::Set(i) => {
				state.keys.insert(storage, i.uri.clone(), i.clone()).await?;
			},
			KeyStoreAction::Remove(uri) => {
				state.keys.remove(storage, uri.clone()).await?;
			},
		}
		Ok(storage.set_value(&state).await?)
	}
}
