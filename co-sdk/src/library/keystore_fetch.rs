use crate::{state::find, CoReducer, CO_CORE_NAME_KEYSTORE};
use co_core_keystore::{Key, KeyStore, KeyStoreAction};
use co_identity::PrivateIdentity;
use std::fmt::Debug;

/// Get or create an key.
pub async fn keystore_fetch<F, I>(
	reducer: &CoReducer,
	identity: &I,
	key: &str,
	create: F,
	force_create: bool,
) -> Result<Key, anyhow::Error>
where
	F: FnOnce() -> Key,
	I: PrivateIdentity + Debug + Send + Sync,
{
	// get
	if !force_create {
		let keystore: KeyStore = reducer.state(CO_CORE_NAME_KEYSTORE).await?;
		if let Some((_, result)) = find(&reducer.storage(), &keystore.keys, |(k, _)| k == key).await? {
			return Ok(result.to_owned());
		}
	}

	// create
	let result = create();

	// store
	reducer
		.push(identity, CO_CORE_NAME_KEYSTORE, &KeyStoreAction::Set(result.clone()))
		.await?;

	// result
	Ok(result)
}
