use crate::{
	backoff_with_jitter,
	bitswap::Token,
	library::libipld_interop::to_libipld_cid,
	types::network_task::{NetworkTask, NetworkTaskSpawner},
	BitswapBehaviourProvider, PeerProvider,
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorageSettings, CloneWithBlockStorageSettings, MappedCid};
use co_storage::{
	BlockStat, BlockStorage, BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage, StorageError,
};
use futures::{channel::oneshot, pin_mut, stream, StreamExt};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use libp2p_bitswap::{BitswapEvent, QueryId};
use std::{collections::BTreeSet, marker::PhantomData, mem::swap, time::Duration};
use tokio::time::{sleep, timeout_at};

pub struct NetworkBlockStorage<S, B, C, N, P> {
	next: S,
	spawner: N,
	peer_provider: P,
	_behaviour: PhantomData<fn(B)>,
	_context: PhantomData<fn(C)>,
	mapping: bool,
	timeout: Duration,
	concurrent: usize,
	tokens: Vec<Token>,
	settings: BlockStorageSettings,
}
impl<S, B, C, N, P> NetworkBlockStorage<S, B, C, N, P>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + Clone + 'static,
	B: NetworkBehaviour + BitswapBehaviourProvider,
	P: PeerProvider + Send + Sync + 'static,
	N: NetworkTaskSpawner<B, C> + Send + Sync + 'static,
{
	pub fn new(next: S, spawner: N, peer_provider: P, timeout: Duration) -> Self {
		Self {
			next,
			spawner,
			peer_provider,
			timeout,
			concurrent: 4,
			mapping: false,
			tokens: Default::default(),
			_behaviour: Default::default(),
			_context: Default::default(),
			settings: Default::default(),
		}
	}

	pub fn set_mapping(&mut self, mapping: bool) {
		self.mapping = mapping;
	}

	pub fn set_tokens(&mut self, tokens: Vec<Token>) {
		self.tokens = tokens;
	}

	pub fn set_peers(&mut self, peers: P) {
		self.peer_provider = peers;
	}

	/// Get block from co network.
	/// Ask `concurrent` peers in parallel for a block.
	/// Use the first block that will be received.
	///
	/// ```mermaid
	/// sequenceDiagram
	/// 		participant A as Caller
	/// 		participant S as Storage
	/// 		participant C as Connections
	/// 		participant B as Bitswap
	/// 		A ->> S: get(cid_a)
	/// 		activate A
	/// 		loop
	/// 			S ->>+ C: connect (use via peers_added)
	/// 			loop with concurrency
	/// 				C -->+ S: peer
	/// 				S ->>+ B: get `cid_a` from `peer`
	/// 				B -->- S: Result<(), Error>
	/// 				alt ok
	/// 					S ->> A: Ok
	/// 					deactivate A
	/// 				else
	/// 					S -->- S: Error
	/// 				end
	/// 			end
	/// 			C --x- S: close
	/// 			opt timeout
	/// 				S --x A: Err(Insufficient peers)
	/// 			end
	/// 		end
	/// ```
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self))]
	async fn get_network(&self, cid: Cid) -> Result<(), StorageError> {
		let mapped = self.to_network_cid(cid).await;
		let deadline = tokio::time::Instant::now() + self.timeout;
		let mut retry = 1;
		loop {
			// start network task for every peer.
			let get_stream = self
				.peer_provider
				.peers_added()
				.flat_map(stream::iter)
				.map(|peer| GetNetworkTask::get(&self.spawner, mapped, self.tokens.clone(), [peer].into()))
				.buffer_unordered(self.concurrent);
			pin_mut!(get_stream);
			loop {
				let result = timeout_at(deadline, get_stream.next()).await;
				match result {
					// no more peers
					Ok(None) => {
						break;
					},
					// some `GetNetworkTask` reported ok
					Ok(Some(Ok(()))) => {
						// done
						return Ok(());
					},
					// some `GetNetworkTask` reported a error
					Ok(Some(Err(err))) => {
						// log
						tracing::warn!(?err, ?cid, "get-network-failed");
					},
					// timeout
					Err(err) => {
						return Err(StorageError::NotFound(cid, err.into()));
					},
				}
			}

			// backoff
			sleep(backoff_with_jitter(retry)).await;

			// timeout?
			if tokio::time::Instant::now() > deadline {
				break;
			}

			// retry
			retry += 1;
		}
		Err(StorageError::NotFound(cid, anyhow::anyhow!("Insufficent peers")))
	}

	async fn to_network_cid(&self, cid: Cid) -> Cid {
		if self.mapping && self.next.is_content_mapped().await {
			self.next.to_plain(&cid).await.unwrap_or(cid)
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
			settings: self.settings.clone(),
			_behaviour: Default::default(),
			_context: Default::default(),
			concurrent: self.concurrent,
		}
	}
}
#[async_trait]
impl<S, B, C, N, P> BlockStorage for NetworkBlockStorage<S, B, C, N, P>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + Clone + 'static,
	B: NetworkBehaviour + BitswapBehaviourProvider,
	P: PeerProvider + Clone + Send + Sync + 'static,
	N: NetworkTaskSpawner<B, C> + Clone + Send + Sync + 'static,
{
	type StoreParams = S::StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		match self.next.get(cid).await {
			Ok(block) => Ok(block),
			Err(StorageError::NotFound(_, _)) if !self.settings.disallow_networking => {
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
			Err(StorageError::NotFound(_, _)) if !self.settings.disallow_networking => {
				self.get_network(*cid).await?;
				self.next.stat(cid).await
			},
			result => result,
		}
	}
}
#[async_trait]
impl<S, B, C, N, P> ExtendedBlockStorage for NetworkBlockStorage<S, B, C, N, P>
where
	S: BlockStorage + ExtendedBlockStorage + BlockStorageContentMapping + Send + Sync + Clone + 'static,
	B: NetworkBehaviour + BitswapBehaviourProvider,
	P: PeerProvider + Clone + Send + Sync + 'static,
	N: NetworkTaskSpawner<B, C> + Clone + Send + Sync + 'static,
{
	async fn set_extended(&self, block: ExtendedBlock<Self::StoreParams>) -> Result<Cid, StorageError> {
		self.next.set_extended(block).await
	}

	async fn clear(&self) -> Result<(), StorageError> {
		self.next.clear().await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		self.next.exists(cid).await
	}
}
impl<S, B, C, N, P> CloneWithBlockStorageSettings for NetworkBlockStorage<S, B, C, N, P>
where
	S: CloneWithBlockStorageSettings,
	P: Clone,
	N: Clone,
{
	fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self {
		Self {
			next: self.next.clone_with_settings(settings.clone()),
			spawner: self.spawner.clone(),
			peer_provider: self.peer_provider.clone(),
			mapping: self.mapping.clone(),
			timeout: self.timeout.clone(),
			concurrent: self.concurrent,
			tokens: self.tokens.clone(),
			settings,
			_behaviour: Default::default(),
			_context: Default::default(),
		}
	}
}
#[async_trait]
impl<S, B, C, N, P> BlockStorageContentMapping for NetworkBlockStorage<S, B, C, N, P>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + Clone + 'static,
	B: NetworkBehaviour + BitswapBehaviourProvider,
	P: PeerProvider + Clone + Send + Sync + 'static,
	N: NetworkTaskSpawner<B, C> + Clone + Send + Sync + 'static,
{
	async fn is_content_mapped(&self) -> bool {
		self.next.is_content_mapped().await
	}

	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.next.to_plain(mapped).await
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.next.to_mapped(plain).await
	}

	async fn insert_mappings(&self, mappings: BTreeSet<MappedCid>) {
		self.next.insert_mappings(mappings).await
	}
}

// #[derive(Debug, thiserror::Error)]
// #[error("Receive block from peer failed: {0:?}")]
// struct GetNetworkError(PeerId, #[source] anyhow::Error);

/// Try to get block using specified peers.
/// Canceled when the result receiver is dropped.
#[derive(Debug)]
pub struct GetNetworkTask {
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

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(spawner, tokens))]
	pub async fn get<B, C, N>(
		spawner: &N,
		cid: Cid,
		tokens: Vec<Token>,
		peers: BTreeSet<PeerId>,
	) -> Result<(), StorageError>
	where
		N: NetworkTaskSpawner<B, C>,
		B: NetworkBehaviour + BitswapBehaviourProvider,
	{
		let (tx, rx) = oneshot::channel();
		let task = GetNetworkTask::new(cid, tokens, peers, tx);
		spawner.spawn(task).map_err(|e| StorageError::Internal(e.into()))?;
		rx.await.map_err(|e| StorageError::Internal(e.into()))??;
		Ok::<(), StorageError>(())
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
