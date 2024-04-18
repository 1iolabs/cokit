use crate::{state::find, CoReducer};
use async_trait::async_trait;
use co_identity::{DidKeyIdentity, IdentityResolverError, PrivateIdentityBox, PrivateIdentityResolver};

#[derive(Debug, Clone)]
pub struct DidKeyProvider {
	reducer: CoReducer,
	keystore_core: String,
}
impl DidKeyProvider {
	pub fn new(reducer: CoReducer, keystore_core: impl Into<String>) -> Self {
		Self { reducer, keystore_core: keystore_core.into() }
	}

	pub async fn store(&self, identity: &DidKeyIdentity, name: Option<String>) -> Result<(), anyhow::Error> {
		let mut key = identity.export()?;
		if let Some(name) = name {
			key.name = name;
		}
		self.reducer
			.push(identity, &self.keystore_core, &co_core_keystore::KeyStoreAction::Set(key))
			.await?;
		Ok(())
	}
}
#[async_trait]
impl PrivateIdentityResolver for DidKeyProvider {
	async fn resolve_private(
		&self,
		identity: &str,
		_public_key: Option<&[u8]>,
	) -> Result<PrivateIdentityBox, IdentityResolverError> {
		let keystore: co_core_keystore::KeyStore = self
			.reducer
			.state(&self.keystore_core)
			.await
			.map_err(|err| IdentityResolverError::Other(err.into()))?;
		let (_name, key) = find(&self.reducer.storage(), &keystore.keys, |(name, _key)| name == identity)
			.await
			.map_err(|err| IdentityResolverError::Other(err.into()))?
			.ok_or(IdentityResolverError::NotFound)?;
		Ok(Box::new(DidKeyIdentity::import(&key).map_err(|err| IdentityResolverError::Other(err.into()))?))
	}
}
