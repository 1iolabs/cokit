// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	find_membership,
	state::{self, QueryExt},
	CoReducer, CO_CORE_NAME_KEYSTORE,
};
use anyhow::anyhow;
use co_core_keystore::Key;
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
	let (_storage, co) = co.co().await?;
	if let Some(key_reference) = co.keys.as_ref().and_then(|keys| keys.first().map(|key| &key.id)) {
		Ok(Some(find_co_key_by_reference(parent, key_reference, None).await?))
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

/// Read current CO PSK from keystore core, if the CO is encrypted.
pub async fn find_co_key_by_membership(parent: &CoReducer, co: &CoId) -> Result<Option<Key>, anyhow::Error> {
	let membership = find_membership(parent, co).await?.ok_or(anyhow!("No membership: {}", co))?;
	if let Some(key_reference) = &membership.key {
		Ok(Some(find_co_key_by_reference(parent, key_reference, None).await?))
	} else {
		Ok(None)
	}
}

/// Read current CO PSK from keystore core, by using its reference.
pub async fn find_co_key_by_reference(
	parent: &CoReducer,
	key_reference: &str,
	keystore_core_name: Option<&str>,
) -> Result<Key, anyhow::Error> {
	let (parent_storage, key_store) = state::query_core(CO_CORE_NAME_KEYSTORE.with_name_opt(keystore_core_name))
		.execute_reducer(parent)
		.await?;
	let (_, key) = state::find(&parent_storage, &key_store.keys, |(k, _)| k == key_reference)
		.await?
		.ok_or(anyhow!("Key not found"))?;
	Ok(key)
}

/// Read current CO PSK from keystore core, by using its reference.
pub async fn find_co_secret_by_reference(
	parent: &CoReducer,
	key_reference: &str,
	keystore_core_name: Option<&str>,
) -> Result<Secret, anyhow::Error> {
	match find_co_key_by_reference(parent, key_reference, keystore_core_name)
		.await?
		.secret
	{
		co_core_keystore::Secret::SharedKey(sec) => Ok(sec),
		_ => Err(anyhow!("Invalid key")),
	}
}
