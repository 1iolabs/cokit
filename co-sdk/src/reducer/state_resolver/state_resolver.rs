use async_trait::async_trait;
use cid::Cid;
use co_primitives::AnyBlockStorage;
use futures::stream::BoxStream;
use std::{collections::BTreeSet, fmt::Debug};

/// Try to resolve state for given heads.
#[async_trait]
pub trait StateResolver<S>: Debug + Send + Sync + 'static
where
	S: AnyBlockStorage,
{
	/// Resolve state/heads for `heads`.
	/// Called multiple times (for each checked head) and should use internal caching.
	async fn resolve_state(
		&self,
		storage: &S,
		context: &StateResolverContext,
		heads: &BTreeSet<Cid>,
	) -> Result<Option<(Cid, BTreeSet<Cid>)>, anyhow::Error>;

	/// Provide a stream of known root states.
	/// The states/heads are not required to be sorted.
	/// Called once per initialize.
	fn provide_roots(
		&self,
		storage: &S,
		context: &StateResolverContext,
	) -> Option<BoxStream<'static, Result<(Cid, BTreeSet<Cid>), anyhow::Error>>>;

	/// Push a new latest state that we calculated.
	async fn push_state(&mut self, storage: &S, context: &StateResolverContext) -> Result<(), anyhow::Error>;
}

/// Context informations that may used by the resolver to help state resolving.
pub struct StateResolverContext {
	/// Latest state.
	pub state: Option<Cid>,

	/// Latest heads.
	pub heads: BTreeSet<Cid>,
}
