use super::provider::BitswapBehaviourProvider;
use crate::{NetworkTask, NetworkTaskSpawner};
use async_trait::async_trait;
use co_storage::{BlockStat, BlockStorage, StorageError};
use futures::channel::oneshot;
use libipld::{Block, Cid};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use libp2p_bitswap::{BitswapEvent, QueryId};
use std::{collections::BTreeSet, mem::swap};

pub struct NetworkBlockStorage<S, B> {
	next: S,
	spawner: NetworkTaskSpawner<B>,
	peers: BTreeSet<PeerId>,
}
impl<S, B> NetworkBlockStorage<S, B>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	B: NetworkBehaviour<ToSwarm = BitswapEvent> + BitswapBehaviourProvider<StoreParams = S::StoreParams>,
{
	pub fn new(next: S, spawner: NetworkTaskSpawner<B>, peers: BTreeSet<PeerId>) -> Self {
		Self { next, spawner, peers }
	}

	pub fn set_peers(&mut self, peers: BTreeSet<PeerId>) {
		self.peers = peers;
	}

	async fn get_network(&self, cid: Cid) -> Result<(), StorageError> {
		let (tx, rx) = oneshot::channel();
		let task = GetNetworkTask::new(cid, self.peers.clone(), tx);
		self.spawner.spawn(task).map_err(|e| StorageError::Internal(e.into()))?;
		rx.await.map_err(|e| StorageError::Internal(e.into()))?
	}
}
#[async_trait]
impl<S, B> BlockStorage for NetworkBlockStorage<S, B>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	B: NetworkBehaviour<ToSwarm = BitswapEvent> + BitswapBehaviourProvider<StoreParams = S::StoreParams>,
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
impl<B> NetworkTask<B> for GetNetworkTask
where
	B: NetworkBehaviour<ToSwarm = BitswapEvent> + BitswapBehaviourProvider,
{
	fn execute(&mut self, swarm: &mut Swarm<B>) {
		let bitswap = swarm.behaviour_mut().bitswap_mut();

		// state
		let mut state = GetNetworkTaskState::Execute;
		swap(&mut self.state, &mut state);

		// execute
		if let GetNetworkTaskState::Pending(peers, result) = state {
			self.state = GetNetworkTaskState::Query(bitswap.get(self.cid, peers.into_iter()), result);
		}
	}

	fn on_swarm_event(&mut self, event: SwarmEvent<B::ToSwarm>) -> Option<SwarmEvent<BitswapEvent>> {
		match (&self.state, event) {
			(
				GetNetworkTaskState::Query(query, _),
				SwarmEvent::Behaviour(BitswapEvent::Complete(event_query, event_result)),
			) if query == &event_query => {
				// state
				let mut state = GetNetworkTaskState::Complete;
				swap(&mut self.state, &mut state);

				// result
				if let GetNetworkTaskState::Query(_, result) = state {
					match result.send(event_result.map_err(|e| StorageError::NotFound(self.cid, e.into()))) {
						Ok(_) => {},
						Err(result) => tracing::warn!(?result, "result-dropped"),
					}
				}
				None
			},
			(_, event) => Some(event),
		}
	}

	fn is_complete(&self) -> bool {
		matches!(self.state, GetNetworkTaskState::Complete)
	}
}
enum GetNetworkTaskState {
	Pending(BTreeSet<PeerId>, oneshot::Sender<Result<(), StorageError>>),
	Execute,
	Query(QueryId, oneshot::Sender<Result<(), StorageError>>),
	Complete,
}
