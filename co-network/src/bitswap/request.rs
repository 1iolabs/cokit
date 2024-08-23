use crate::bitswap::Token;
use anyhow::Result;
use async_trait::async_trait;
use co_storage::StorageError;
use futures::{
	channel::{mpsc, oneshot},
	SinkExt,
};
use libipld::{store::StoreParams, Block, Cid};
use libp2p::PeerId;
use libp2p_bitswap::BitswapStore;

#[derive(Debug)]
pub enum BitswapRequest<P>
where
	P: StoreParams,
{
	Contains(Cid, PeerId, Vec<Token>, oneshot::Sender<Result<bool, StorageError>>),
	Get(Cid, PeerId, Vec<Token>, oneshot::Sender<Result<Option<Vec<u8>>, StorageError>>),
	Insert(Block<P>, PeerId, Vec<Token>, oneshot::Sender<Result<(), StorageError>>),
	MissingBlocks(Cid, Vec<Token>, oneshot::Sender<Result<Vec<Cid>, StorageError>>),
}

/// Handle bitswap request by sendings them to a channel.
pub struct BitswapRequestBlockStorage<P>
where
	P: StoreParams,
{
	sender: mpsc::Sender<BitswapRequest<P>>,
}
impl<P> BitswapRequestBlockStorage<P>
where
	P: StoreParams,
{
	pub fn new(buffer: usize) -> (Self, mpsc::Receiver<BitswapRequest<P>>) {
		let (tx, rx) = mpsc::channel(buffer);
		(Self { sender: tx }, rx)
	}
}
#[async_trait]
impl<P> BitswapStore for BitswapRequestBlockStorage<P>
where
	P: StoreParams,
{
	type Params = P;

	#[tracing::instrument(ret, err, skip(self))]
	async fn contains(&mut self, cid: &Cid, remote_peer: &PeerId, tokens: &[Token]) -> Result<bool> {
		let (tx, rx) = oneshot::channel();
		self.sender
			.clone()
			.send(BitswapRequest::Contains(*cid, *remote_peer, tokens.to_vec(), tx))
			.await?;
		Ok(rx.await??)
	}

	#[tracing::instrument(err, skip(self))]
	async fn get(&mut self, cid: &Cid, remote_peer: &PeerId, tokens: &[Token]) -> Result<Option<Vec<u8>>> {
		let (tx, rx) = oneshot::channel();
		self.sender
			.clone()
			.send(BitswapRequest::Get(*cid, *remote_peer, tokens.to_vec(), tx))
			.await?;
		Ok(rx.await??)
	}

	#[tracing::instrument(err, skip(self, block), fields(cid = ?block.cid()))]
	async fn insert(&mut self, block: &Block<Self::Params>, remote_peer: &PeerId, tokens: &[Token]) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.sender
			.clone()
			.send(BitswapRequest::Insert(block.clone(), *remote_peer, tokens.to_vec(), tx))
			.await?;
		Ok(rx.await??)
	}

	#[tracing::instrument(err, skip(self))]
	async fn missing_blocks(&mut self, cid: &Cid, tokens: &[Token]) -> Result<Vec<Cid>> {
		let (tx, rx) = oneshot::channel();
		self.sender
			.clone()
			.send(BitswapRequest::MissingBlocks(*cid, tokens.to_vec(), tx))
			.await?;
		Ok(rx.await??)
	}
}
