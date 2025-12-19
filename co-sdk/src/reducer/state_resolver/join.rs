use crate::{
	reducer::state_resolver::{DynamicStateResolver, StateResolver, StateResolverContext},
	ReducerChangeContext,
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::AnyBlockStorage;
use futures::{
	stream::{self, BoxStream, TryStreamExt},
	StreamExt,
};
use std::{collections::BTreeSet, fmt::Debug};

// Join multipe state resolvers
pub struct JoinStateResolver<S>(Vec<DynamicStateResolver<S>>);
impl<S> Debug for JoinStateResolver<S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("JoinStateResolver").field(&self.0).finish()
	}
}
impl<S: AnyBlockStorage> JoinStateResolver<S> {
	pub fn new(next: impl StateResolver<S>) -> Self {
		Self(vec![DynamicStateResolver::new(next)])
	}

	pub fn from_iter<T: IntoIterator<Item = DynamicStateResolver<S>>>(iter: T) -> Option<Self> {
		let items = iter.into_iter().collect::<Vec<_>>();
		if items.is_empty() {
			None
		} else {
			Some(JoinStateResolver(items))
		}
	}

	pub fn join(mut self, next: impl StateResolver<S>) -> Self {
		self.0.push(DynamicStateResolver::new(next));
		self
	}

	pub fn join_box(mut self, next: DynamicStateResolver<S>) -> Self {
		self.0.push(next);
		self
	}
}
#[async_trait]
impl<S: AnyBlockStorage> StateResolver<S> for JoinStateResolver<S> {
	async fn resolve_state(
		&self,
		storage: &S,
		context: &StateResolverContext,
		heads: &BTreeSet<Cid>,
	) -> Result<Option<(Cid, BTreeSet<Cid>)>, anyhow::Error> {
		let mut result = Ok(None);
		for next in self.0.iter() {
			match next.resolve_state(storage, context, heads).await {
				Ok(Some(result)) => return Ok(Some(result)),
				Err(err) => {
					if result.is_ok() {
						result = Err(err);
					}
				},
				_ => {},
			}
		}
		result
	}

	fn provide_roots(
		&mut self,
		storage: &S,
		context: &StateResolverContext,
	) -> Option<BoxStream<'static, Result<(Option<Cid>, BTreeSet<Cid>), anyhow::Error>>> {
		let streams = self
			.0
			.iter_mut()
			.filter_map(|next| next.provide_roots(storage, context))
			.collect::<Vec<_>>();
		if !streams.is_empty() {
			Some(
				stream::iter(streams)
					.map(Result::<_, anyhow::Error>::Ok)
					.try_flatten_unordered(None)
					.boxed(),
			)
		} else {
			None
		}
	}

	async fn initialize(&mut self, storage: &S) -> Result<(), anyhow::Error> {
		for next in self.0.iter_mut() {
			next.initialize(storage).await?;
		}
		Ok(())
	}

	async fn push_state(
		&mut self,
		storage: &S,
		change_context: &ReducerChangeContext,
		state: Cid,
		heads: &BTreeSet<Cid>,
	) -> Result<(), anyhow::Error> {
		for next in self.0.iter_mut() {
			next.push_state(storage, change_context, state, heads).await?;
		}
		Ok(())
	}

	fn clear(&mut self) {
		for next in self.0.iter_mut() {
			next.clear();
		}
	}
}
