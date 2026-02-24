// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	state::{find, query_core, QueryExt},
	CoReducer, CO_CORE_NAME_KEYSTORE,
};
use co_core_keystore::{Key, KeyStoreAction};
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
	I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
{
	// get
	if !force_create {
		let (storage, keystore) = query_core(CO_CORE_NAME_KEYSTORE).execute_reducer(reducer).await?;
		if let Some((_, result)) = find(&storage, &keystore.keys, |(k, _)| k == key).await? {
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
