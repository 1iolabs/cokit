// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	reducer::state_resolver::{StateResolver, StateResolverContext, StateStream},
	ReducerChangeContext,
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::AnyBlockStorage;
use std::{collections::BTreeSet, fmt::Debug};

// StateResolver Box
pub struct DynamicStateResolver<S>(Box<dyn StateResolver<S>>);
impl<S> Debug for DynamicStateResolver<S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DynamicStateResolver").field(&self.0).finish()
	}
}
impl<S: AnyBlockStorage> DynamicStateResolver<S> {
	pub fn new(inner: impl StateResolver<S>) -> Self {
		Self(Box::new(inner))
	}
}
#[async_trait]
impl<S: AnyBlockStorage> StateResolver<S> for DynamicStateResolver<S> {
	async fn resolve_state(
		&self,
		storage: &S,
		context: &StateResolverContext,
		heads: &BTreeSet<Cid>,
	) -> Result<Option<(Cid, BTreeSet<Cid>)>, anyhow::Error> {
		self.0.resolve_state(storage, context, heads).await
	}

	fn provide_roots(&mut self, storage: &S, context: &StateResolverContext) -> Option<StateStream> {
		self.0.provide_roots(storage, context)
	}

	async fn initialize(&mut self, storage: &S) -> Result<(), anyhow::Error> {
		self.0.initialize(storage).await
	}

	async fn push_state(
		&mut self,
		storage: &S,
		change_context: &ReducerChangeContext,
		state: Cid,
		heads: &BTreeSet<Cid>,
	) -> Result<(), anyhow::Error> {
		self.0.push_state(storage, change_context, state, heads).await
	}

	fn clear(&mut self) {
		self.0.clear();
	}
}
