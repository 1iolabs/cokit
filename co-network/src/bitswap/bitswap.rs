use crate::bitswap::Token;
use anyhow::Result;
use async_trait::async_trait;
use co_storage::{BlockStorage, StorageError};
use libipld::{Block, Cid, DefaultParams};
use libp2p::PeerId;
use libp2p_bitswap::BitswapStore;
use std::marker::PhantomData;

/// Wrap BlockStorage as BitswapStore
pub struct BitswapBlockStorage<S, R> {
	storage_resolver: R,
	_storage: PhantomData<S>,
}
impl<S, R> BitswapBlockStorage<S, R> {
	pub fn new(storage_resolver: R) -> Self {
		Self { storage_resolver, _storage: Default::default() }
	}
}

#[async_trait]
pub trait StorageResolver<S> {
	/// Resolve storage using the token and remote peer address.
	/// For local operations the remote_peer is None.
	async fn resolve_storage(&self, remote_peer: Option<&PeerId>, tokens: &[Token]) -> Result<S, anyhow::Error>;
}

pub struct StaticStorageResolver<S> {
	storage: S,
}
#[async_trait]
impl<S> StorageResolver<S> for StaticStorageResolver<S>
where
	S: BlockStorage<StoreParams = DefaultParams> + Clone + Send + Sync + 'static,
{
	async fn resolve_storage(&self, _remote_peer: Option<&PeerId>, _tokens: &[Token]) -> Result<S, anyhow::Error> {
		Ok(self.storage.clone())
	}
}

/// Handle (external) peer requests.
#[async_trait]
impl<S, R> BitswapStore for BitswapBlockStorage<S, R>
where
	S: BlockStorage<StoreParams = DefaultParams> + Send + Sync + 'static,
	R: StorageResolver<S> + Send + Sync + 'static,
{
	type Params = S::StoreParams;

	#[tracing::instrument(ret, err, skip(self))]
	async fn contains(&mut self, cid: &Cid, remote_peer: &PeerId, tokens: &[Token]) -> Result<bool> {
		match self
			.storage_resolver
			.resolve_storage(Some(remote_peer), tokens)
			.await?
			.stat(cid)
			.await
		{
			Ok(_) => Ok(true),
			Err(StorageError::NotFound(_, _)) => Ok(false),
			Err(e) => Err(e.into()),
		}
	}

	#[tracing::instrument(err, skip(self))]
	async fn get(&mut self, cid: &Cid, remote_peer: &PeerId, tokens: &[Token]) -> Result<Option<Vec<u8>>> {
		match self
			.storage_resolver
			.resolve_storage(Some(remote_peer), tokens)
			.await?
			.get(cid)
			.await
		{
			Ok(block) => Ok(Some(block.into_inner().1)),
			Err(StorageError::NotFound(_, _)) => Ok(None),
			Err(e) => Err(e.into()),
		}
	}

	#[tracing::instrument(err, skip(self, block), fields(cid = ?block.cid()))]
	async fn insert(&mut self, block: &Block<Self::Params>, remote_peer: &PeerId, tokens: &[Token]) -> Result<()> {
		tracing::info!(cid = ?block.cid(), "bitswap-insert");
		self.storage_resolver
			.resolve_storage(Some(remote_peer), tokens)
			.await?
			.set(block.clone())
			.await?;
		Ok(())
	}

	#[tracing::instrument(err, skip(self))]
	async fn missing_blocks(&mut self, cid: &Cid, tokens: &[Token]) -> Result<Vec<Cid>> {
		let storage = self.storage_resolver.resolve_storage(None, tokens).await?;
		let mut stack = vec![*cid];
		let mut missing = vec![];
		while let Some(cid) = stack.pop() {
			match storage.get(&cid).await {
				Ok(block) => {
					block.references(&mut stack)?;
				},
				Err(StorageError::NotFound(_, _)) => {
					missing.push(cid);
				},
				Err(e) => return Err(e.into()),
			}
		}
		Ok(missing)
	}
}
