use crate::{
	state::{core_state_or_default, stream},
	CoReducerError, CoStorage, CO_CORE_NAME_KEYSTORE,
};
use co_core_keystore::{Key, KeyStore};
use co_primitives::{tags, Did, OptionLink};
use futures::Stream;

#[derive(Debug, Clone)]
pub struct Identity {
	pub did: Did,
	pub name: String,
	pub description: String,
}

/// Find identities contained in a (single) CO.
/// Returns an empty stream is the core not exists.
pub fn identities(
	storage: CoStorage,
	co_state: OptionLink<co_core_co::Co>,
) -> impl Stream<Item = Result<Identity, CoReducerError>> {
	async_stream::try_stream! {
		// root
		let keystore: KeyStore = core_state_or_default(&storage, co_state, CO_CORE_NAME_KEYSTORE).await?;
		for await key in stream(storage.clone(), &keystore.keys) {
			let key: Key = key?.1;
			if key.tags.matches(tags!("type": "co-identity")) {
				yield Identity { did: key.uri, name: key.name, description: key.description };
			}
		}
	}
}
