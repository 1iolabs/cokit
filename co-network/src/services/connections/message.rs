// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{
	action::{ConnectionAction, DidPeersChangedAction, DidUseAction, PeersChangedAction, UseAction},
	overview::{CoConnectionOverview, ConnectionOverview},
};
use co_actor::{time::Instant, ActorError, ActorHandle, Response, ResponseStream};
use co_primitives::{CoId, Did, Network};
use futures::Stream;

#[derive(Debug)]
pub enum ConnectionMessage {
	/// Use a CO by utilizing the specified networks.
	Use(UseAction, ResponseStream<PeersChangedAction>),

	/// Use a DID connection by utilizing the specified networks.
	DidUse(DidUseAction, ResponseStream<DidPeersChangedAction>),

	/// Action.
	Action(ConnectionAction),

	/// Get the current connection overview.
	Overview(Response<ConnectionOverview>),

	/// Subscribe to a live stream of connection snapshots.
	/// The current snapshot is sent immediately, then a fresh one after every state change.
	OverviewStream(ResponseStream<ConnectionOverview>),

	/// Get the current connection overview scoped to a single CO.
	CoOverview(CoId, Response<CoConnectionOverview>),

	/// Subscribe to a live stream of CO-scoped connection overviews.
	/// The current overview is sent immediately, then a fresh one after every state change.
	CoOverviewStream(CoId, ResponseStream<CoConnectionOverview>),
}
impl<T> From<T> for ConnectionMessage
where
	T: Into<ConnectionAction>,
{
	fn from(value: T) -> Self {
		Self::Action(value.into())
	}
}
impl ConnectionMessage {
	pub fn co_use(
		actor: ActorHandle<Self>,
		id: CoId,
		from: Did,
		networks: impl IntoIterator<Item = Network>,
	) -> impl Stream<Item = Result<PeersChangedAction, ActorError>> {
		let action = UseAction { id, from, time: Instant::now(), networks: networks.into_iter().collect() };
		actor.stream(|response| Self::Use(action, response))
	}

	/// Use connections to a DID.
	///
	/// # Args
	/// - `from` - The source of the connection attempt.
	/// - `to` - The target of the connection attempt.
	/// - `networks` - The networks to connect `to`. Required to be non empty.
	pub fn did_use(
		actor: ActorHandle<Self>,
		from: Did,
		to: Did,
		networks: impl IntoIterator<Item = Network>,
	) -> impl Stream<Item = Result<DidPeersChangedAction, ActorError>> {
		let action = DidUseAction { from, to, time: Instant::now(), networks: networks.into_iter().collect() };
		actor.stream(|response| Self::DidUse(action, response))
	}
}
