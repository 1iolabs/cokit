use super::QueryError;
use crate::{
	state::{query_core, stream, Query},
	CoStorage, CO_CORE_NAME_KEYSTORE,
};
use co_core_keystore::Key;
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
	core_name: Option<&'_ str>,
) -> impl Stream<Item = Result<Identity, QueryError>> + '_ {
	let core_name = CO_CORE_NAME_KEYSTORE.with_name_opt(core_name);
	async_stream::try_stream! {
		// root
		let keystore = query_core(core_name)
			.with_default()
			.execute(&storage, co_state)
			.await?;
		for await key in stream(storage.clone(), &keystore.keys) {
			let key: Key = key?.1;
			if is_identity(&key) {
				yield Identity { did: key.uri, name: key.name, description: key.description };
			}
		}
	}
}

/// Test if the specified key is an CO identity.
pub fn is_identity(key: &Key) -> bool {
	key.tags.matches(tags!("type": "co-identity"))
}
