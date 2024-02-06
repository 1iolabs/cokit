use crate::{CoReducer, Cores, CO_CORE_KEYSTORE};
use co_core_keystore::{Key, KeyStore, KeyStoreAction};

/// Get or create an key.
pub async fn keystore_fetch<F: FnOnce() -> Key>(
	reducer: &CoReducer,
	key: &str,
	create: F,
) -> Result<Key, anyhow::Error> {
	// get
	let keystore: KeyStore = reducer.state(Cores::to_core_name(CO_CORE_KEYSTORE)).await?;
	if let Some(result) = keystore.keys.get(key) {
		return Ok(result.to_owned())
	}

	// create
	let result = create();

	// store
	reducer
		.push(Cores::to_core_name(CO_CORE_KEYSTORE), &KeyStoreAction::Set(result.clone()))
		.await?;

	// result
	Ok(result)
}
