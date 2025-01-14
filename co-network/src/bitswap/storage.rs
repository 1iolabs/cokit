use crate::{
	bitswap::Token, library::libipld_interop::to_libipld_cid, BitswapBehaviourProvider, NetworkTask,
	NetworkTaskSpawner, PeerProvider,
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::Block;
use co_storage::{BlockStat, BlockStorage, BlockStorageContentMapping, StorageError};
use futures::{channel::oneshot, pin_mut};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use libp2p_bitswap::{BitswapEvent, QueryId};
use std::{collections::BTreeSet, marker::PhantomData, mem::swap, sync::Arc, time::Duration};
use tokio_stream::StreamExt;

pub struct NetworkBlockStorage<S, B, C, N, P> {
	next: S,
	spawner: N,
	peer_provider: P,
	_behaviour: PhantomData<fn(B)>,
	_context: PhantomData<fn(C)>,
	mapping: Option<Arc<dyn BlockStorageContentMapping + Send + Sync + 'static>>,
	timeout: Duration,
	tokens: Vec<Token>,
}
impl<S, B, C, N, P> NetworkBlockStorage<S, B, C, N, P>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	B: NetworkBehaviour + BitswapBehaviourProvider,
	P: PeerProvider + Send + Sync + 'static,
	N: NetworkTaskSpawner<B, C> + Send + Sync + 'static,
{
	pub fn new(next: S, spawner: N, peer_provider: P, timeout: Duration) -> Self {
		Self {
			next,
			spawner,
			peer_provider,
			mapping: None,
			timeout,
			tokens: Default::default(),
			_behaviour: Default::default(),
			_context: Default::default(),
		}
	}

	pub fn set_mapping<M: BlockStorageContentMapping + Send + Sync + 'static>(&mut self, mapping: M) {
		self.mapping = Some(Arc::new(mapping));
	}

	pub fn set_tokens(&mut self, tokens: Vec<Token>) {
		self.tokens = tokens;
	}

	pub fn set_peers(&mut self, peers: P) {
		self.peer_provider = peers;
	}

	async fn get_network(&self, cid: Cid) -> Result<(), StorageError> {
		let mapped = self.to_network_cid(cid).await;
		let result_stream = self
			.peer_provider
			.peers_added()
			.filter(|peers| !peers.is_empty())
			// start network task for every peer(s).
			.then(|peers| async move {
				let (tx, rx) = oneshot::channel();
				let task = GetNetworkTask::new(mapped, self.tokens.clone(), peers, tx);
				self.spawner.spawn(task).map_err(|e| StorageError::Internal(e.into()))?;
				rx.await.map_err(|e| StorageError::Internal(e.into()))??;
				Ok::<(), StorageError>(())
			})
			.timeout(self.timeout)
			.filter_map(|result| match result {
				Ok(network_result) => match network_result {
					// success - return ok
					// drop will cancel others
					Ok(_) => Some(Ok(())),
					// error - ignore other errors
					Err(_) => None,
				},
				// timeout - return err
				Err(e) => {
					return Some(Err(e));
				},
			})
			.take(1);
		pin_mut!(result_stream);
		while let Some(result) = result_stream.next().await {
			return result.map_err(|e| StorageError::NotFound(cid, e.into()));
		}
		Err(StorageError::NotFound(cid, anyhow::anyhow!("Insufficent peers")))
	}

	async fn to_network_cid(&self, cid: Cid) -> Cid {
		if let Some(mapping) = &self.mapping {
			mapping.to_plain(&cid).await.unwrap_or(cid)
		} else {
			cid
		}
	}
}
impl<S, B, C, N, P> Clone for NetworkBlockStorage<S, B, C, N, P>
where
	S: Clone,
	P: Clone,
	N: Clone,
{
	fn clone(&self) -> Self {
		Self {
			next: self.next.clone(),
			spawner: self.spawner.clone(),
			peer_provider: self.peer_provider.clone(),
			mapping: self.mapping.clone(),
			timeout: self.timeout.clone(),
			tokens: self.tokens.clone(),
			_behaviour: Default::default(),
			_context: Default::default(),
		}
	}
}
#[async_trait]
impl<S, B, C, N, P> BlockStorage for NetworkBlockStorage<S, B, C, N, P>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	B: NetworkBehaviour + BitswapBehaviourProvider,
	P: PeerProvider + Send + Sync + 'static,
	N: NetworkTaskSpawner<B, C> + Send + Sync + 'static,
{
	type StoreParams = S::StoreParams;

	/// Returns a block from storage.
	#[tracing::instrument(err, skip(self))]
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		match self.next.get(cid).await {
			Ok(block) => Ok(block),
			Err(StorageError::NotFound(_, _)) => {
				self.get_network(*cid).await?;
				self.next.get(cid).await
			},
			Err(e) => Err(e),
		}
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	#[tracing::instrument(err, skip(self, block), fields(cid = ?block.cid()))]
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		self.next.set(block).await
	}

	/// Remove a block.
	#[tracing::instrument(err, skip(self))]
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.next.remove(cid).await
	}

	/// Stat a block.
	#[tracing::instrument(err, skip(self))]
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		match self.next.stat(cid).await {
			Err(StorageError::NotFound(_, _)) => {
				self.get_network(*cid).await?;
				self.next.stat(cid).await
			},
			result => result,
		}
	}
}

// #[derive(Debug, thiserror::Error)]
// #[error("Receive block from peer failed: {0:?}")]
// struct GetNetworkError(PeerId, #[source] anyhow::Error);

/// Try to get block using specified peers.
/// Canceled when the result receiver is dropped.
#[derive(Debug)]
struct GetNetworkTask {
	cid: Cid,
	tokens: Vec<Token>,
	state: GetNetworkTaskState,
}
impl GetNetworkTask {
	pub fn new(
		cid: Cid,
		tokens: Vec<Token>,
		peers: BTreeSet<PeerId>,
		result: oneshot::Sender<Result<(), StorageError>>,
	) -> Self {
		Self { cid, tokens, state: GetNetworkTaskState::Pending(peers, result) }
	}
}
impl<B, C> NetworkTask<B, C> for GetNetworkTask
where
	B: NetworkBehaviour + BitswapBehaviourProvider,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, _context: &mut C) {
		let bitswap = swarm.behaviour_mut().bitswap_mut();

		// state
		let mut state = GetNetworkTaskState::Execute;
		swap(&mut self.state, &mut state);

		// execute
		if let GetNetworkTaskState::Pending(peers, result) = state {
			let query = bitswap.get(to_libipld_cid(self.cid), peers.clone().into_iter(), self.tokens.clone());
			tracing::debug!(?self.cid, ?peers, ?query, "bitswap-get");
			self.state = GetNetworkTaskState::Query(query, result);
		}
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		match event {
			SwarmEvent::Behaviour(behaviour_event) => {
				match (&self.state, B::bitswap_event(&behaviour_event)) {
					(GetNetworkTaskState::Query(query, _), Some(BitswapEvent::Complete(event_query, _)))
						if query == event_query =>
					{
						// consume event
						let bitswap_event = B::into_bitswap_event(behaviour_event);
						if let Ok(BitswapEvent::Complete(_, event_result)) = bitswap_event {
							// log
							tracing::debug!(?self.cid, ?query, result = ?event_result, "bitswap-get-complete");

							// state
							let mut state = GetNetworkTaskState::Complete;
							swap(&mut self.state, &mut state);

							// result
							if let GetNetworkTaskState::Query(_, result) = state {
								match result.send(event_result.map_err(|e| StorageError::NotFound(self.cid, e.into())))
								{
									Ok(_) => {},
									Err(_) => {
										// cancelled
									},
								}
							}
						}
						None
					},
					(_, _) => Some(SwarmEvent::Behaviour(behaviour_event)),
				}
			},
			event => Some(event),
		}
	}

	fn is_complete(&mut self) -> bool {
		match &self.state {
			GetNetworkTaskState::Complete => true,
			GetNetworkTaskState::Query(_, result) => result.is_canceled(),
			_ => false,
		}
	}
}

#[derive(Debug)]
enum GetNetworkTaskState {
	Pending(BTreeSet<PeerId>, oneshot::Sender<Result<(), StorageError>>),
	Execute,
	Query(QueryId, oneshot::Sender<Result<(), StorageError>>),
	Complete,
}
