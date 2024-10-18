use crate::bitswap::Token;
use anyhow::Result;
use async_trait::async_trait;
use co_actor::{ActorHandle, Response};
use co_storage::StorageError;
use libipld::{store::StoreParams, Block, Cid};
use libp2p::PeerId;
use libp2p_bitswap::BitswapStore;

#[derive(Debug)]
pub enum BitswapMessage<P>
where
	P: StoreParams,
{
	Contains(Cid, PeerId, Vec<Token>, Response<Result<bool, StorageError>>),
	Get(Cid, PeerId, Vec<Token>, Response<Result<Option<Vec<u8>>, StorageError>>),
	Insert(Block<P>, PeerId, Vec<Token>, Response<Result<(), StorageError>>),
	MissingBlocks(Cid, Vec<Token>, Response<Result<Vec<Cid>, StorageError>>),
}

/// Handle bitswap request by sendings them to a actor.
pub struct BitswapStoreClient<P>
where
	P: StoreParams,
{
	handle: ActorHandle<BitswapMessage<P>>,
}
impl<P> BitswapStoreClient<P>
where
	P: StoreParams,
{
	pub fn new(handle: ActorHandle<BitswapMessage<P>>) -> Self {
		Self { handle }
	}
}
#[async_trait]
impl<P> BitswapStore for BitswapStoreClient<P>
where
	P: StoreParams,
{
	type Params = P;

	#[tracing::instrument(ret, err, skip(self))]
	async fn contains(&mut self, cid: &Cid, remote_peer: &PeerId, tokens: &[Token]) -> Result<bool> {
		Ok(self
			.handle
			.request(|response| BitswapMessage::Contains(*cid, *remote_peer, tokens.to_vec(), response))
			.await??)
	}

	#[tracing::instrument(err, skip(self))]
	async fn get(&mut self, cid: &Cid, remote_peer: &PeerId, tokens: &[Token]) -> Result<Option<Vec<u8>>> {
		Ok(self
			.handle
			.request(|response| BitswapMessage::Get(*cid, *remote_peer, tokens.to_vec(), response))
			.await??)
	}

	#[tracing::instrument(err, skip(self, block), fields(cid = ?block.cid()))]
	async fn insert(&mut self, block: &Block<Self::Params>, remote_peer: &PeerId, tokens: &[Token]) -> Result<()> {
		Ok(self
			.handle
			.request(|response| BitswapMessage::Insert(block.clone(), *remote_peer, tokens.to_vec(), response))
			.await??)
	}

	#[tracing::instrument(err, skip(self))]
	async fn missing_blocks(&mut self, cid: &Cid, tokens: &[Token]) -> Result<Vec<Cid>> {
		Ok(self
			.handle
			.request(|response| BitswapMessage::MissingBlocks(*cid, tokens.to_vec(), response))
			.await??)
	}
}
