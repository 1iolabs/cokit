use co_primitives::{ReducerAction, Tags};
use co_wasm_api::{reduce, Context, Reducer};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Key Store.
///
/// This COre should only be used in encrypted COs.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyStore {
	// #[co_api::Map]
	pub keys: BTreeMap<String, Key>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Secret {
	Password(Vec<u8>),
	PrivateKey(Vec<u8>),
	SharedKey(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyStoreAction {
	Set(Key),
	Remove(String),
}

impl Reducer for KeyStore {
	type Action = KeyStoreAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &mut dyn Context) -> Self {
		let mut result = self;
		match &event.payload {
			KeyStoreAction::Set(i) => {
				result.keys.insert(i.uri.clone(), i.clone());
			},
			KeyStoreAction::Remove(uri) => {
				result.keys.remove(uri);
			},
		}
		result
	}
}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<KeyStore>()
}
