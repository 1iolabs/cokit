use crate::{
	Identity, IdentityBox, IdentityResolver, IdentityResolverError, PrivateIdentityBox, PrivateIdentityResolver,
};
use async_trait::async_trait;
use co_primitives::Did;
use futures::lock::Mutex;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone, Default)]
pub struct MemoryIdentityResolver {
	identites: Arc<Mutex<HashMap<Did, IdentityBox>>>,
}
impl MemoryIdentityResolver {
	pub async fn insert(&mut self, identity: IdentityBox) {
		self.identites.lock().await.insert(identity.identity().to_owned(), identity);
	}
}
#[async_trait]
impl IdentityResolver for MemoryIdentityResolver {
	async fn resolve(&self, identity: &str) -> Result<IdentityBox, IdentityResolverError> {
		self.identites
			.lock()
			.await
			.get(identity)
			.cloned()
			.ok_or(IdentityResolverError::NotFound)
	}
}

#[derive(Debug, Clone, Default)]
pub struct MemoryPrivateIdentityResolver {
	identites: Arc<Mutex<HashMap<Did, PrivateIdentityBox>>>,
}
impl MemoryPrivateIdentityResolver {
	pub async fn insert(&self, identity: PrivateIdentityBox) {
		self.identites.lock().await.insert(identity.identity().to_owned(), identity);
	}
}
#[async_trait]
impl PrivateIdentityResolver for MemoryPrivateIdentityResolver {
	async fn resolve_private(&self, identity: &str) -> Result<PrivateIdentityBox, IdentityResolverError> {
		self.identites
			.lock()
			.await
			.get(identity)
			.cloned()
			.ok_or(IdentityResolverError::NotFound)
	}
}
