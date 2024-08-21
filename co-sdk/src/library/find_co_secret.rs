use crate::{find_membership, state, CoReducer, CO_CORE_NAME_KEYSTORE};
use anyhow::anyhow;
use co_core_keystore::{Key, KeyStore};
use co_primitives::{CoId, Secret};

/// Read current CO PSK from keychain core, if the CO is encrypted.
pub async fn find_co_secret(parent: &CoReducer, co: &CoReducer) -> Result<Option<Secret>, anyhow::Error> {
	if let Some(key) = find_co_key(parent, co).await? {
		match key.secret {
			co_core_keystore::Secret::SharedKey(sec) => Ok(Some(sec)),
			_ => Err(anyhow!("Invalid secret")),
		}
	} else {
		Ok(None)
	}
}

/// Read current CO PSK from keychain core, if the CO is encrypted.
pub async fn find_co_key(parent: &CoReducer, co: &CoReducer) -> Result<Option<Key>, anyhow::Error> {
	let co = co.co().await?;
	let key_store: KeyStore = parent.state(CO_CORE_NAME_KEYSTORE).await.unwrap();
	if let Some(key_reference) = co.keys.as_ref().and_then(|keys| keys.first().map(|key| &key.id)) {
		let (_, key) = state::find(&parent.storage(), &key_store.keys, |(k, _)| k == key_reference)
			.await?
			.ok_or(anyhow!("Key not found"))?;
		Ok(Some(key))
	} else {
		Ok(None)
	}
}

/// Read current CO PSK from keychain core, if the CO is encrypted.
pub async fn find_co_secret_by_membership(parent: &CoReducer, co: &CoId) -> Result<Option<Secret>, anyhow::Error> {
	if let Some(key) = find_co_key_by_membership(parent, co).await? {
		match key.secret {
			co_core_keystore::Secret::SharedKey(sec) => Ok(Some(sec)),
			_ => Err(anyhow!("Invalid secret")),
		}
	} else {
		Ok(None)
	}
}

/// Read current CO PSK from keychain core, if the CO is encrypted.
pub async fn find_co_key_by_membership(parent: &CoReducer, co: &CoId) -> Result<Option<Key>, anyhow::Error> {
	let membership = find_membership(parent, co).await?.ok_or(anyhow!("No membership: {}", co))?;
	let key_store: KeyStore = parent.state(CO_CORE_NAME_KEYSTORE).await.unwrap();
	if let Some(key_reference) = &membership.key {
		let (_, key) = state::find(&parent.storage(), &key_store.keys, |(k, _)| k == key_reference)
			.await?
			.ok_or(anyhow!("Key not found"))?;
		Ok(Some(key))
	} else {
		Ok(None)
	}
}
