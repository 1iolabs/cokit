use co_api::{Context, DagMap, Reducer, ReducerAction, Tags};
use serde::{Deserialize, Serialize};

/// Key Store.
///
/// This COre should only be used in encrypted COs.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyStore {
	// #[co_api::Map]
	pub keys: DagMap<String, Key>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Secret {
	Password(co_api::Secret),
	PrivateKey(co_api::Secret),
	SharedKey(co_api::Secret),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyStoreAction {
	Set(Key),
	Remove(String),
}
impl Reducer for KeyStore {
	type Action = KeyStoreAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
		let mut result = self;
		match &event.payload {
			KeyStoreAction::Set(i) => {
				result.keys.insert(context, i.uri.clone(), i.clone());
			},
			KeyStoreAction::Remove(uri) => {
				result.keys.remove(context, uri);
			},
		}
		result
	}
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[no_mangle]
pub extern "C" fn state() {
	co_api::reduce::<KeyStore>()
}
