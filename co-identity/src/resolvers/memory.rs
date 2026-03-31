// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	Identity, IdentityBox, IdentityResolver, IdentityResolverError, PrivateIdentityBox, PrivateIdentityResolver,
};
use async_trait::async_trait;
use co_primitives::Did;
use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Default)]
pub struct MemoryIdentityResolver {
	identites: Arc<Mutex<HashMap<Did, IdentityBox>>>,
}
impl MemoryIdentityResolver {
	pub async fn insert(&mut self, identity: IdentityBox) {
		self.identites.lock().unwrap().insert(identity.identity().to_owned(), identity);
	}
}
#[async_trait]
impl IdentityResolver for MemoryIdentityResolver {
	async fn resolve(&self, identity: &str) -> Result<IdentityBox, IdentityResolverError> {
		self.identites
			.lock()
			.unwrap()
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
	pub fn from(iter: impl IntoIterator<Item = PrivateIdentityBox>) -> Self {
		Self {
			identites: Arc::new(Mutex::new(
				iter.into_iter()
					.map(|identity| (identity.identity().to_owned(), identity))
					.collect(),
			)),
		}
	}

	pub async fn insert(&self, identity: PrivateIdentityBox) {
		self.identites.lock().unwrap().insert(identity.identity().to_owned(), identity);
	}
}
#[async_trait]
impl PrivateIdentityResolver for MemoryPrivateIdentityResolver {
	async fn resolve_private(&self, identity: &str) -> Result<PrivateIdentityBox, IdentityResolverError> {
		self.identites
			.lock()
			.unwrap()
			.get(identity)
			.cloned()
			.ok_or(IdentityResolverError::NotFound)
	}
}
