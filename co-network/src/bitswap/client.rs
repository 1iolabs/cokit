// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	bitswap::Token,
	library::libipld_interop::{from_libipld_block, from_libipld_cid, to_libipld_cid},
};
use anyhow::Result;
use async_trait::async_trait;
use cid::Cid;
use co_actor::{ActorHandle, Response};
use co_primitives::Block;
use co_storage::StorageError;
use libp2p::PeerId;
use libp2p_bitswap::BitswapStore;

#[derive(Debug)]
pub enum BitswapMessage {
	Contains(Cid, PeerId, Vec<Token>, Response<Result<bool, StorageError>>),
	Get(Cid, PeerId, Vec<Token>, Response<Result<Option<Vec<u8>>, StorageError>>),
	Insert(Block, PeerId, Vec<Token>, Response<Result<(), StorageError>>),
	MissingBlocks(Cid, Vec<Token>, Response<Result<Vec<Cid>, StorageError>>),
}

/// Handle bitswap requests by sendings them to an actor.
pub struct BitswapStoreClient {
	handle: ActorHandle<BitswapMessage>,
}
impl BitswapStoreClient {
	pub fn new(handle: ActorHandle<BitswapMessage>) -> Self {
		Self { handle }
	}
}
#[async_trait]
impl BitswapStore for BitswapStoreClient {
	type Params = libipld::DefaultParams;

	#[tracing::instrument(level = tracing::Level::TRACE, ret, err(Debug), skip(self))]
	async fn contains(&mut self, cid: &libipld::Cid, remote_peer: &PeerId, tokens: &[Token]) -> Result<bool> {
		Ok(self
			.handle
			.request(|response| {
				BitswapMessage::Contains(from_libipld_cid(*cid), *remote_peer, tokens.to_vec(), response)
			})
			.await??)
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self))]
	async fn get(&mut self, cid: &libipld::Cid, remote_peer: &PeerId, tokens: &[Token]) -> Result<Option<Vec<u8>>> {
		Ok(self
			.handle
			.request(|response| BitswapMessage::Get(from_libipld_cid(*cid), *remote_peer, tokens.to_vec(), response))
			.await??)
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self, block), fields(cid = ?block.cid()))]
	async fn insert(
		&mut self,
		block: &libipld::Block<Self::Params>,
		remote_peer: &PeerId,
		tokens: &[Token],
	) -> Result<()> {
		Ok(self
			.handle
			.request(|response| {
				BitswapMessage::Insert(from_libipld_block(block.clone()), *remote_peer, tokens.to_vec(), response)
			})
			.await??)
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self))]
	async fn missing_blocks(&mut self, cid: &libipld::Cid, tokens: &[Token]) -> Result<Vec<libipld::Cid>> {
		Ok(self
			.handle
			.request(|response| BitswapMessage::MissingBlocks(from_libipld_cid(*cid), tokens.to_vec(), response))
			.await??
			.into_iter()
			.map(to_libipld_cid)
			.collect())
	}
}
