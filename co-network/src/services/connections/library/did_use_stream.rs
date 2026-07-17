// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::super::action::{DidPeersChangedAction, DidReleaseAction};
use crate::connections::ConnectionMessage;
use co_actor::{ActorError, ActorHandle, ResponseStreamReceiver};
use futures::Stream;
use std::{
	pin::Pin,
	task::{Context, Poll},
};

pub struct DidUseStream {
	pub(crate) actor: ActorHandle<ConnectionMessage>,
	pub(crate) release: Option<DidReleaseAction>,
	pub(crate) response: Option<ResponseStreamReceiver<DidPeersChangedAction>>,
	pub(crate) start_error: Option<ActorError>,
	pub(crate) registered: bool,
}
impl Stream for DidUseStream {
	type Item = Result<DidPeersChangedAction, ActorError>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if let Some(error) = self.start_error.take() {
			self.response.take();
			return Poll::Ready(Some(Err(error)));
		}
		let Some(response) = self.response.as_mut() else {
			return Poll::Ready(None);
		};
		match Pin::new(response).poll_next(cx) {
			Poll::Ready(Some(value)) => Poll::Ready(Some(Ok(value))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}
impl Drop for DidUseStream {
	fn drop(&mut self) {
		self.response.take();
		if self.registered {
			self.registered = false;
			let release = self.release.take().expect("registered DID use has a release action");
			let to = release.to.clone();
			if let Err(error) = self.actor.dispatch(release) {
				tracing::warn!(?error, %to, "did-use-release-failed");
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connections::{ConnectionOverview, ConnectionState, DisconnectReason, DisconnectedAction};
	use async_trait::async_trait;
	use co_actor::{Actor, Reducer};
	use co_primitives::{Did, Network, NetworkRendezvous, Tags};

	struct TestConnections;

	#[async_trait]
	impl Actor for TestConnections {
		type Message = ConnectionMessage;
		type State = ConnectionState;
		type Initialize = ();

		async fn initialize(
			&self,
			_handle: &ActorHandle<Self::Message>,
			_tags: &Tags,
			_initialize: Self::Initialize,
		) -> Result<Self::State, ActorError> {
			Ok(ConnectionState::default())
		}

		async fn handle(
			&self,
			_handle: &ActorHandle<Self::Message>,
			message: Self::Message,
			state: &mut Self::State,
		) -> Result<(), ActorError> {
			match message {
				ConnectionMessage::DidUse(action, _response) => {
					state.reduce(action.into());
				},
				ConnectionMessage::Action(action) => {
					state.reduce(action);
				},
				ConnectionMessage::Overview(response) => {
					response.respond(ConnectionOverview::from(&*state));
				},
				_ => {},
			}
			Ok(())
		}
	}

	#[tokio::test]
	async fn did_use_streams_release_their_own_reference_on_drop() {
		let actor = Actor::spawn(Default::default(), TestConnections, ()).unwrap();
		let handle = actor.handle();
		let from = Did::from("did:local:alice");
		let to = Did::from("did:local:bob");
		let network =
			Network::Rendezvous(NetworkRendezvous { namespace: "scoped-did-use".to_owned(), addresses: vec![] });

		let first = ConnectionMessage::did_use(handle.clone(), from.clone(), to.clone(), [network.clone()]);
		let second = ConnectionMessage::did_use(handle.clone(), from, to.clone(), [network]);
		let overview = handle.request(ConnectionMessage::Overview).await.unwrap();
		assert!(overview.dids.iter().any(|entry| entry.to == to));

		drop(first);
		let overview = handle.request(ConnectionMessage::Overview).await.unwrap();
		assert!(overview.dids.iter().any(|entry| entry.to == to));

		drop(second);
		let overview = handle.request(ConnectionMessage::Overview).await.unwrap();
		assert!(overview.dids.iter().all(|entry| entry.to != to));
	}

	async fn assert_stale_stream_drop_preserves_reacquired_did(reason: DisconnectReason) {
		let actor = Actor::spawn(Default::default(), TestConnections, ()).unwrap();
		let handle = actor.handle();
		let from = Did::from("did:local:alice");
		let to = Did::from("did:local:bob");
		let network =
			Network::Rendezvous(NetworkRendezvous { namespace: "failed-did-use".to_owned(), addresses: vec![] });

		let first = ConnectionMessage::did_use(handle.clone(), from.clone(), to.clone(), [network.clone()]);
		let overview = handle.request(ConnectionMessage::Overview).await.unwrap();
		assert!(overview.dids.iter().any(|entry| entry.to == to));

		handle
			.dispatch(DisconnectedAction { network: network.clone(), reason })
			.unwrap();
		let overview = handle.request(ConnectionMessage::Overview).await.unwrap();
		assert!(overview.dids.iter().all(|entry| entry.to != to));

		let second = ConnectionMessage::did_use(handle.clone(), from, to.clone(), [network]);
		let overview = handle.request(ConnectionMessage::Overview).await.unwrap();
		assert!(overview.dids.iter().any(|entry| entry.to == to));

		drop(first);
		let overview = handle.request(ConnectionMessage::Overview).await.unwrap();
		assert!(overview.dids.iter().any(|entry| entry.to == to));

		drop(second);
		let overview = handle.request(ConnectionMessage::Overview).await.unwrap();
		assert!(overview.dids.iter().all(|entry| entry.to != to));
	}

	#[tokio::test]
	async fn stale_stream_drop_does_not_release_a_reacquired_did_after_failure() {
		assert_stale_stream_drop_preserves_reacquired_did(DisconnectReason::Failure("route failed".to_owned())).await;
	}

	#[tokio::test]
	async fn stale_stream_drop_does_not_release_a_reacquired_did_after_timeout() {
		assert_stale_stream_drop_preserves_reacquired_did(DisconnectReason::Timeout).await;
	}
}
