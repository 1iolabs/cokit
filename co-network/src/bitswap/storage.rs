// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	bitswap::Token,
	library::libipld_interop::to_libipld_cid,
	network::{Behaviour, NetworkEvent},
	types::network_task::{NetworkTask, NetworkTaskSpawner},
};
use cid::Cid;
use co_storage::StorageError;
use futures::channel::oneshot;
use libp2p::{swarm::SwarmEvent, PeerId, Swarm};
use libp2p_bitswap::{BitswapEvent, QueryId};
use std::{collections::BTreeSet, mem::swap};

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
	pub async fn get<N>(spawner: &N, cid: Cid, tokens: Vec<Token>, peers: BTreeSet<PeerId>) -> Result<(), StorageError>
	where
		N: NetworkTaskSpawner<Behaviour>,
	{
		let (tx, rx) = oneshot::channel();
		let task = GetNetworkTask::new(cid, tokens, peers, tx);
		spawner.spawn(task).map_err(|e| StorageError::Internal(e.into()))?;
		rx.await.map_err(|e| StorageError::Internal(e.into()))??;
		Ok::<(), StorageError>(())
	}
}
impl NetworkTask<Behaviour> for GetNetworkTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>) {
		let bitswap = &mut swarm.behaviour_mut().bitswap;

		// state
		let mut state = GetNetworkTaskState::Execute;
		swap(&mut self.state, &mut state);

		// execute
		if let GetNetworkTaskState::Pending(peers, result) = state {
			let query = bitswap.get(to_libipld_cid(self.cid), peers.clone(), self.tokens.clone());
			tracing::debug!(?self.cid, ?peers, ?query, "bitswap-get");
			self.state = GetNetworkTaskState::Query(query, result);
		}
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		match event {
			SwarmEvent::Behaviour(NetworkEvent::Bitswap(bitswap_event)) => {
				match (&self.state, &bitswap_event) {
					(GetNetworkTaskState::Query(query, _), BitswapEvent::Complete(event_query, _))
						if query == event_query =>
					{
						// consume event
						if let BitswapEvent::Complete(_, event_result) = bitswap_event {
							// log
							tracing::debug!(?self.cid, ?query, result = ?event_result, "bitswap-get-complete");

							// state
							let mut state = GetNetworkTaskState::Complete;
							swap(&mut self.state, &mut state);

							// result
							if let GetNetworkTaskState::Query(_, result) = state {
								match result.send(event_result.map_err(|e| StorageError::NotFound(self.cid, e))) {
									Ok(_) => {},
									Err(_) => {
										// cancelled
									},
								}
							}
						}
						None
					},
					(_, _) => Some(SwarmEvent::Behaviour(NetworkEvent::Bitswap(bitswap_event))),
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
