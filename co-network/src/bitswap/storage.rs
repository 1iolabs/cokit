use crate::{BitswapBehaviourProvider, NetworkTask, NetworkTaskSpawner};
use async_trait::async_trait;
use co_storage::{BlockStat, BlockStorage, BlockStorageContentMapping, StorageError};
use futures::channel::oneshot;
use libipld::{Block, Cid};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use libp2p_bitswap::{BitswapEvent, QueryId};
use std::{collections::BTreeSet, mem::swap, sync::Arc};

#[async_trait]
pub trait PeerProvider {
	async fn peers(&self) -> Result<BTreeSet<PeerId>, StorageError>;
}

pub struct NetworkBlockStorage<S, B, C> {
	next: S,
	spawner: NetworkTaskSpawner<B, C>,
	peers: Option<Arc<dyn PeerProvider + Send + Sync + 'static>>,
	mapping: Option<Arc<dyn BlockStorageContentMapping + Send + Sync + 'static>>,
}
impl<S, B, C> NetworkBlockStorage<S, B, C>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	B: NetworkBehaviour + BitswapBehaviourProvider<StoreParams = S::StoreParams>,
{
	pub fn new(next: S, spawner: NetworkTaskSpawner<B, C>) -> Self {
		Self { next, spawner, peers: Default::default(), mapping: None }
	}

	pub fn set_mapping<M: BlockStorageContentMapping + Send + Sync + 'static>(&mut self, mapping: M) {
		self.mapping = Some(Arc::new(mapping));
	}

	pub fn set_peers<P: PeerProvider + Send + Sync + 'static>(&mut self, peers: P) {
		self.peers = Some(Arc::new(peers));
	}

	async fn get_network(&self, cid: Cid) -> Result<(), StorageError> {
		let mapped = self.to_network_cid(cid).await;
		let peers = match &self.peers {
			Some(p) => p.peers().await?,
			None => Default::default(),
		};
		let (tx, rx) = oneshot::channel();
		let task = GetNetworkTask::new(mapped, peers, tx);
		self.spawner.spawn(task).map_err(|e| StorageError::Internal(e.into()))?;
		rx.await.map_err(|e| StorageError::Internal(e.into()))?
	}

	async fn to_network_cid(&self, cid: Cid) -> Cid {
		if let Some(mapping) = &self.mapping {
			mapping.to_plain(&cid).await.unwrap_or(cid)
		} else {
			cid
		}
	}
}
impl<S, B, C> Clone for NetworkBlockStorage<S, B, C>
where
	S: Clone,
{
	fn clone(&self) -> Self {
		Self {
			next: self.next.clone(),
			spawner: self.spawner.clone(),
			peers: self.peers.clone(),
			mapping: self.mapping.clone(),
		}
	}
}
#[async_trait]
impl<S, B, C> BlockStorage for NetworkBlockStorage<S, B, C>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	B: NetworkBehaviour + BitswapBehaviourProvider<StoreParams = S::StoreParams>,
{
	type StoreParams = S::StoreParams;

	/// Returns a block from storage.
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
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		self.next.set(block).await
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.next.remove(cid).await
	}

	/// Stat a block.
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

struct GetNetworkTask {
	cid: Cid,
	state: GetNetworkTaskState,
}
impl GetNetworkTask {
	pub fn new(cid: Cid, peers: BTreeSet<PeerId>, result: oneshot::Sender<Result<(), StorageError>>) -> Self {
		Self { cid, state: GetNetworkTaskState::Pending(peers, result) }
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
			self.state = GetNetworkTaskState::Query(bitswap.get(self.cid, peers.into_iter()), result);
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
							// state
							let mut state = GetNetworkTaskState::Complete;
							swap(&mut self.state, &mut state);

							// result
							if let GetNetworkTaskState::Query(_, result) = state {
								match result.send(event_result.map_err(|e| StorageError::NotFound(self.cid, e.into())))
								{
									Ok(_) => {},
									Err(result) => tracing::warn!(?result, "result-dropped"),
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
		matches!(self.state, GetNetworkTaskState::Complete)
	}
}
enum GetNetworkTaskState {
	Pending(BTreeSet<PeerId>, oneshot::Sender<Result<(), StorageError>>),
	Execute,
	Query(QueryId, oneshot::Sender<Result<(), StorageError>>),
	Complete,
}
