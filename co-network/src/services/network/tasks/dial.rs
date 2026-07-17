// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::types::network_task::{NetworkTask, NetworkTaskSpawner};
use anyhow::anyhow;
use futures::channel::oneshot;
use libp2p::{
	swarm::{
		dial_opts::{DialOpts, PeerCondition},
		ConnectionId, DialError, NetworkBehaviour, SwarmEvent,
	},
	Multiaddr, PeerId, Swarm,
};
use std::mem::take;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialIntent {
	Reachability,
	Redundancy,
}

impl DialIntent {
	fn peer_condition(self) -> PeerCondition {
		match self {
			Self::Reachability => PeerCondition::DisconnectedAndNotDialing,
			Self::Redundancy => PeerCondition::NotDialing,
		}
	}

	fn uses_connected_shortcut(self) -> bool {
		matches!(self, Self::Reachability)
	}
}

pub(crate) fn is_concurrent_dial_rejection(err: &anyhow::Error) -> bool {
	matches!(err.downcast_ref::<DialError>(), Some(DialError::DialPeerConditionFalse(_)))
}

pub(crate) fn known_peer_dial_opts(peer_id: PeerId, addresses: Vec<Multiaddr>, intent: DialIntent) -> DialOpts {
	let opts = DialOpts::peer_id(peer_id).condition(intent.peer_condition());
	if addresses.is_empty() {
		opts.build()
	} else {
		opts.addresses(addresses).build()
	}
}

/// Dial and wait for a connection to be made or fail.
#[derive(Debug)]
pub struct DialNetworkTask {
	opts: Option<DialOpts>,
	complete: ConnectionId,
	tx: Option<oneshot::Sender<Result<PeerId, anyhow::Error>>>,
	intent: DialIntent,
}
impl DialNetworkTask {
	pub async fn dial<B, N>(
		spawner: &N,
		peer_id: Option<PeerId>,
		addresses: Vec<Multiaddr>,
	) -> Result<PeerId, anyhow::Error>
	where
		N: NetworkTaskSpawner<B>,
		B: NetworkBehaviour,
	{
		Self::dial_with_intent(spawner, peer_id, addresses, DialIntent::Reachability).await
	}

	pub(crate) async fn dial_with_intent<B, N>(
		spawner: &N,
		peer_id: Option<PeerId>,
		addresses: Vec<Multiaddr>,
		intent: DialIntent,
	) -> Result<PeerId, anyhow::Error>
	where
		N: NetworkTaskSpawner<B>,
		B: NetworkBehaviour,
	{
		let opts = match peer_id {
			Some(peer_id) => known_peer_dial_opts(peer_id, addresses, intent),
			None => {
				let address = addresses
					.into_iter()
					.next()
					.ok_or(anyhow!("Expected exactly one address if no peer is specified"))?;
				DialOpts::unknown_peer_id().address(address).build()
			},
		};
		let (tx, rx) = oneshot::channel();
		spawner.spawn(Self { complete: opts.connection_id(), opts: Some(opts), tx: Some(tx), intent })?;
		rx.await?
	}

	fn send_if_connected<B>(&mut self, swarm: &mut Swarm<B>, peer_id: Option<PeerId>) -> bool
	where
		B: NetworkBehaviour,
	{
		if let Some(peer_id) = peer_id {
			if swarm.is_connected(&peer_id) {
				if let Some(tx) = Option::take(&mut self.tx) {
					tx.send(Ok(peer_id)).ok();
				}
				true
			} else {
				false
			}
		} else {
			false
		}
	}
}
impl<B> NetworkTask<B> for DialNetworkTask
where
	B: NetworkBehaviour,
{
	fn execute(&mut self, swarm: &mut Swarm<B>) {
		if let Some(opts) = take(&mut self.opts) {
			let peer_id = opts.get_peer_id();

			// already connected?
			if self.intent.uses_connected_shortcut() && self.send_if_connected(swarm, peer_id) {
				return;
			}

			// dial
			tracing::trace!(?opts, "network-dial");
			match swarm.dial(opts) {
				Err(DialError::DialPeerConditionFalse(condition)) => {
					if !self.send_if_connected(swarm, peer_id) {
						if let Some(tx) = Option::take(&mut self.tx) {
							tx.send(Err(DialError::DialPeerConditionFalse(condition).into())).ok();
						}
					}
				},
				Err(e) => {
					if let Some(tx) = Option::take(&mut self.tx) {
						tx.send(Err(e.into())).ok();
					}
				},
				_ => (),
			}
		}
	}

	/// Handle swarm events.
	/// Events can be consumed by this handler or forwarded to next handler.
	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		match &event {
			SwarmEvent::ConnectionEstablished {
				peer_id,
				connection_id,
				endpoint: _,
				num_established: _,
				concurrent_dial_errors: _,
				established_in: _,
			} => {
				if connection_id == &self.complete {
					if let Some(tx) = Option::take(&mut self.tx) {
						tx.send(Ok(*peer_id)).ok();
					}
				}
			},
			SwarmEvent::OutgoingConnectionError { connection_id, peer_id: _, error } => {
				if connection_id == &self.complete {
					if let Some(tx) = Option::take(&mut self.tx) {
						tx.send(Err(anyhow!("{:?}", error))).ok();
					}
				}
			},
			_ => {},
		}
		Some(event)
	}

	/// Test if the task is complete and can be removed from the queue.
	/// This will be called only after execute has been called.
	fn is_complete(&mut self) -> bool {
		self.tx.is_none()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::network_task::{NetworkTaskBox, TokioNetworkTaskSpawner};
	use futures::{future, FutureExt, StreamExt};
	use libp2p::{
		core::{transport::PortUse, Endpoint},
		noise,
		swarm::{
			dial_opts::PeerCondition, dummy, ConnectionDenied, FromSwarm, THandler, THandlerInEvent, THandlerOutEvent,
			ToSwarm,
		},
		tcp, yamux, SwarmBuilder,
	};
	use std::{convert::Infallible, task::Poll, time::Duration};
	use tokio::sync::mpsc;

	struct AddressBehaviour {
		address: Option<Multiaddr>,
	}

	impl NetworkBehaviour for AddressBehaviour {
		type ConnectionHandler = dummy::ConnectionHandler;
		type ToSwarm = Infallible;

		fn handle_established_inbound_connection(
			&mut self,
			_connection_id: ConnectionId,
			_peer: PeerId,
			_local_addr: &Multiaddr,
			_remote_addr: &Multiaddr,
		) -> Result<THandler<Self>, ConnectionDenied> {
			Ok(dummy::ConnectionHandler)
		}

		fn handle_pending_outbound_connection(
			&mut self,
			_connection_id: ConnectionId,
			_maybe_peer: Option<PeerId>,
			_addresses: &[Multiaddr],
			_effective_role: Endpoint,
		) -> Result<Vec<Multiaddr>, ConnectionDenied> {
			Ok(self.address.iter().cloned().collect())
		}

		fn handle_established_outbound_connection(
			&mut self,
			_connection_id: ConnectionId,
			_peer: PeerId,
			_addr: &Multiaddr,
			_role_override: Endpoint,
			_port_use: PortUse,
		) -> Result<THandler<Self>, ConnectionDenied> {
			Ok(dummy::ConnectionHandler)
		}

		fn on_swarm_event(&mut self, _event: FromSwarm) {}

		fn on_connection_handler_event(
			&mut self,
			_peer_id: PeerId,
			_connection_id: ConnectionId,
			event: THandlerOutEvent<Self>,
		) {
			match event {}
		}

		fn poll(
			&mut self,
			_context: &mut std::task::Context<'_>,
		) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
			Poll::Pending
		}
	}

	fn test_swarm(address: Option<Multiaddr>) -> Swarm<AddressBehaviour> {
		SwarmBuilder::with_new_identity()
			.with_tokio()
			.with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
			.unwrap()
			.with_behaviour(|_| AddressBehaviour { address })
			.unwrap()
			.with_swarm_config(|config| config.with_idle_connection_timeout(Duration::from_secs(10)))
			.build()
	}

	fn capturing_spawner<B>() -> (TokioNetworkTaskSpawner<B>, mpsc::UnboundedReceiver<NetworkTaskBox<B>>)
	where
		B: NetworkBehaviour,
	{
		let (tasks, receiver) = mpsc::unbounded_channel();
		(TokioNetworkTaskSpawner { tasks }, receiver)
	}

	fn dial_task(
		peer_id: PeerId,
		addresses: Vec<Multiaddr>,
		intent: DialIntent,
	) -> (DialNetworkTask, oneshot::Receiver<Result<PeerId, anyhow::Error>>) {
		let opts = known_peer_dial_opts(peer_id, addresses, intent);
		let complete = opts.connection_id();
		let (tx, rx) = oneshot::channel();
		(DialNetworkTask { opts: Some(opts), complete, tx: Some(tx), intent }, rx)
	}

	fn task_is_complete(task: &mut DialNetworkTask) -> bool {
		<DialNetworkTask as NetworkTask<AddressBehaviour>>::is_complete(task)
	}

	async fn start_listener(swarm: &mut Swarm<AddressBehaviour>) -> Multiaddr {
		swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();
		loop {
			if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
				return address;
			}
		}
	}

	async fn wait_for_outbound_connection(
		dialer: &mut Swarm<AddressBehaviour>,
		listener: &mut Swarm<AddressBehaviour>,
		peer_id: PeerId,
	) {
		next_outbound_connection_event(dialer, listener, peer_id).await;
	}

	async fn next_outbound_connection_event(
		dialer: &mut Swarm<AddressBehaviour>,
		listener: &mut Swarm<AddressBehaviour>,
		peer_id: PeerId,
	) -> SwarmEvent<Infallible> {
		let (event, ()) = tokio::time::timeout(
			Duration::from_secs(5),
			future::join(
				async {
					loop {
						let event = dialer.select_next_some().await;
						if matches!(
							&event,
							SwarmEvent::ConnectionEstablished { peer_id: established, .. }
								if *established == peer_id
						) {
							break event;
						}
					}
				},
				async {
					loop {
						if matches!(listener.select_next_some().await, SwarmEvent::ConnectionEstablished { .. }) {
							break;
						}
					}
				},
			),
		)
		.await
		.expect("loopback peers should connect");
		event
	}

	#[test]
	fn dial_intents_select_shortcut_and_peer_condition() {
		assert!(DialIntent::Reachability.uses_connected_shortcut());
		assert!(!DialIntent::Redundancy.uses_connected_shortcut());
		assert!(matches!(DialIntent::Reachability.peer_condition(), PeerCondition::DisconnectedAndNotDialing));
		assert!(matches!(DialIntent::Redundancy.peer_condition(), PeerCondition::NotDialing));
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn known_peer_empty_addresses_use_behaviour_addresses() {
		let mut listener = test_swarm(None);
		let listener_addr = start_listener(&mut listener).await;
		let listener_peer = *listener.local_peer_id();
		let mut dialer = test_swarm(Some(listener_addr));

		let opts = known_peer_dial_opts(listener_peer, vec![], DialIntent::Reachability);
		dialer.dial(opts).expect("behaviour-provided address should be used");
		wait_for_outbound_connection(&mut dialer, &mut listener, listener_peer).await;
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn known_peer_explicit_addresses_stay_explicit() {
		let mut listener = test_swarm(None);
		let listener_addr = start_listener(&mut listener).await;
		let listener_peer = *listener.local_peer_id();

		let mut wrong_listener = test_swarm(None);
		let wrong_addr = start_listener(&mut wrong_listener).await;
		drop(wrong_listener);

		let mut dialer = test_swarm(Some(listener_addr));
		let opts = known_peer_dial_opts(listener_peer, vec![wrong_addr], DialIntent::Reachability);
		dialer.dial(opts).expect("explicit address should start a dial attempt");

		let error = tokio::time::timeout(Duration::from_secs(5), async {
			loop {
				tokio::select! {
					event = dialer.select_next_some() => match event {
						SwarmEvent::OutgoingConnectionError { error, .. } => break error,
						SwarmEvent::ConnectionEstablished { .. } => {
							panic!("behaviour address must not extend an explicit address list")
						},
						_ => {},
					},
					event = listener.select_next_some() => {
						if matches!(event, SwarmEvent::ConnectionEstablished { .. }) {
							panic!("explicit dial must not fall through to the behaviour address");
						}
					},
				}
			}
		})
		.await
		.expect("wrong explicit address should fail");
		assert!(!dialer.is_connected(&listener_peer));
		assert!(!matches!(error, DialError::NoAddresses));
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn dial_entrypoint_defaults_to_reachability() {
		let mut listener = test_swarm(None);
		let listener_addr = start_listener(&mut listener).await;
		let listener_peer = *listener.local_peer_id();
		let mut dialer = test_swarm(None);
		dialer
			.dial(DialOpts::peer_id(listener_peer).addresses(vec![listener_addr.clone()]).build())
			.unwrap();
		wait_for_outbound_connection(&mut dialer, &mut listener, listener_peer).await;

		let (spawner, mut tasks) = capturing_spawner();
		let dial = DialNetworkTask::dial(&spawner, Some(listener_peer), vec![listener_addr]);
		futures::pin_mut!(dial);
		assert!(dial.as_mut().now_or_never().is_none());
		let mut task = tasks.recv().await.expect("dial should spawn a network task");
		task.execute(&mut dialer);

		let result = tokio::time::timeout(Duration::from_secs(1), dial)
			.await
			.expect("reachability should shortcut an existing connection")
			.expect("connected peer should succeed");
		assert_eq!(result, listener_peer);
		assert_eq!(dialer.network_info().connection_counters().num_pending_outgoing(), 0);
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn reachability_shortcuts_but_redundancy_dials_connected_peer() {
		let mut listener = test_swarm(None);
		let listener_addr = start_listener(&mut listener).await;
		let listener_peer = *listener.local_peer_id();
		let mut dialer = test_swarm(None);
		dialer
			.dial(DialOpts::peer_id(listener_peer).addresses(vec![listener_addr.clone()]).build())
			.unwrap();
		wait_for_outbound_connection(&mut dialer, &mut listener, listener_peer).await;

		let (mut reachability, reachability_result) =
			dial_task(listener_peer, vec![listener_addr.clone()], DialIntent::Reachability);
		reachability.execute(&mut dialer);
		assert_eq!(reachability_result.await.unwrap().unwrap(), listener_peer);
		assert_eq!(dialer.network_info().connection_counters().num_pending_outgoing(), 0);

		let (mut redundancy, redundancy_result) = dial_task(listener_peer, vec![listener_addr], DialIntent::Redundancy);
		redundancy.execute(&mut dialer);
		assert_eq!(dialer.network_info().connection_counters().num_pending_outgoing(), 1);
		let event = next_outbound_connection_event(&mut dialer, &mut listener, listener_peer).await;
		redundancy.on_swarm_event(&mut dialer, event);
		assert_eq!(redundancy_result.await.unwrap().unwrap(), listener_peer);
		assert_eq!(dialer.network_info().connection_counters().num_established_outgoing(), 2);
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn condition_false_while_connected_completes_successfully() {
		let mut listener = test_swarm(None);
		let listener_addr = start_listener(&mut listener).await;
		let listener_peer = *listener.local_peer_id();
		let mut dialer = test_swarm(None);
		dialer
			.dial(DialOpts::peer_id(listener_peer).addresses(vec![listener_addr.clone()]).build())
			.unwrap();
		wait_for_outbound_connection(&mut dialer, &mut listener, listener_peer).await;

		dialer
			.dial(
				DialOpts::peer_id(listener_peer)
					.addresses(vec![listener_addr.clone()])
					.condition(PeerCondition::Always)
					.build(),
			)
			.unwrap();
		let (mut task, result) = dial_task(listener_peer, vec![listener_addr], DialIntent::Redundancy);
		task.execute(&mut dialer);

		assert_eq!(result.await.unwrap().unwrap(), listener_peer);
		assert!(task_is_complete(&mut task));
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn condition_false_while_disconnected_completes_with_error() {
		let mut listener = test_swarm(None);
		let listener_addr = start_listener(&mut listener).await;
		let listener_peer = *listener.local_peer_id();
		let mut dialer = test_swarm(None);
		dialer
			.dial(
				DialOpts::peer_id(listener_peer)
					.addresses(vec![listener_addr.clone()])
					.condition(PeerCondition::Always)
					.build(),
			)
			.unwrap();
		assert!(!dialer.is_connected(&listener_peer));

		let (mut task, result) = dial_task(listener_peer, vec![listener_addr], DialIntent::Redundancy);
		task.execute(&mut dialer);
		let result = tokio::time::timeout(Duration::from_millis(200), result)
			.await
			.expect("a rejected dial must complete instead of hanging")
			.expect("task response should be delivered");
		assert!(result.is_err());
		assert!(task_is_complete(&mut task));
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn connection_established_event_only_completes_matching_task() {
		let mut listener = test_swarm(None);
		let listener_addr = start_listener(&mut listener).await;
		let listener_peer = *listener.local_peer_id();
		let mut dialer = test_swarm(None);
		let (mut matching, matching_result) =
			dial_task(listener_peer, vec![listener_addr.clone()], DialIntent::Reachability);
		let (mut other, _other_result) = dial_task(listener_peer, vec![listener_addr], DialIntent::Reachability);
		matching.execute(&mut dialer);

		let event = next_outbound_connection_event(&mut dialer, &mut listener, listener_peer).await;
		let event = other
			.on_swarm_event(&mut dialer, event)
			.expect("unmatched event should be forwarded");
		assert!(!task_is_complete(&mut other));
		matching.on_swarm_event(&mut dialer, event);

		assert_eq!(matching_result.await.unwrap().unwrap(), listener_peer);
		assert!(task_is_complete(&mut matching));
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn outgoing_connection_error_event_only_completes_matching_task() {
		let mut failed_listener = test_swarm(None);
		let failed_addr = start_listener(&mut failed_listener).await;
		drop(failed_listener);
		let failed_peer = PeerId::random();
		let mut dialer = test_swarm(None);
		let (mut matching_error, matching_error_result) =
			dial_task(failed_peer, vec![failed_addr.clone()], DialIntent::Reachability);
		let (mut other_error, _other_error_result) =
			dial_task(failed_peer, vec![failed_addr], DialIntent::Reachability);
		matching_error.execute(&mut dialer);

		let event = tokio::time::timeout(Duration::from_secs(5), async {
			loop {
				let event = dialer.select_next_some().await;
				if matches!(event, SwarmEvent::OutgoingConnectionError { .. }) {
					break event;
				}
			}
		})
		.await
		.expect("dialing a closed loopback listener should fail");
		let event = other_error
			.on_swarm_event(&mut dialer, event)
			.expect("unmatched error should be forwarded");
		assert!(!task_is_complete(&mut other_error));
		matching_error.on_swarm_event(&mut dialer, event);

		assert!(matching_error_result.await.unwrap().is_err());
		assert!(task_is_complete(&mut matching_error));
	}
}
