use async_trait::async_trait;
use cid::Cid;
use co_primitives::AnyBlockStorage;
use futures::stream::BoxStream;
use std::{collections::BTreeSet, fmt::Debug};

/// Resolve state for given heads.
///
/// Responsibilities:
/// - Resolve persisted states.
/// - Remember new states.
///
/// Notes:
/// - All methods must return internal/mapped Cid only.
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

	/// Provide a stream of known roots (state/heads or just heads).
	/// The states/heads are not required to be sorted.
	/// Called once per initialize.
	fn provide_roots(
		&mut self,
		storage: &S,
		context: &StateResolverContext,
	) -> Option<BoxStream<'static, Result<(Option<Cid>, BTreeSet<Cid>), anyhow::Error>>>;

	/// Initialize the resolver.
	async fn initialize(&mut self, storage: &S) -> Result<(), anyhow::Error> {
		let _storage = storage;
		Ok(())
	}

	/// Push a new latest state that we calculated.
	async fn push_state(&mut self, storage: &S, state: Cid, heads: BTreeSet<Cid>) -> Result<(), anyhow::Error> {
		let _storage = storage;
		let _state = state;
		let _heads = heads;
		Ok(())
	}

	/// Clear the resolver.
	fn clear(&mut self) {}
}

/// Context informations that may used by the resolver to help state resolving.
#[derive(Debug, Default, Clone)]
pub struct StateResolverContext {
	/// Latest state.
	pub state: Option<Cid>,

	/// Latest heads.
	pub heads: BTreeSet<Cid>,
}
