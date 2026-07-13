// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{DnsMessage, DnsSource, DnsState};
use crate::services::network::NetworkDns;
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle};
use co_primitives::Tags;

#[cfg(feature = "native")]
type Resolver = crate::dns::AutoResolver;
#[cfg(not(feature = "native"))]
type Resolver = ();

pub struct DnsInitialize {
	#[cfg(feature = "native")]
	resolver: Resolver,
}

impl DnsInitialize {
	pub fn new(dns: &NetworkDns, handle: ActorHandle<DnsMessage>) -> Self {
		#[cfg(feature = "native")]
		{
			let resolver = crate::dns::AutoResolver::new(dns, Box::new(crate::dns::RealSystemConf), handle);
			Self { resolver }
		}
		#[cfg(not(feature = "native"))]
		{
			let _ = dns;
			let _ = handle;
			Self {}
		}
	}

	#[cfg(feature = "native")]
	pub(crate) fn resolver(&self) -> Resolver {
		self.resolver.clone()
	}

	#[cfg(not(feature = "native"))]
	pub(crate) fn resolver(&self) -> Resolver {
		()
	}
}

#[derive(Debug, Default)]
pub struct DnsActor;

#[async_trait]
impl Actor for DnsActor {
	type Message = DnsMessage;
	type State = DnsActorState;
	type Initialize = DnsInitialize;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		#[cfg(not(feature = "native"))]
		let _ = initialize;
		Ok(DnsActorState {
			state: DnsState::default(),
			#[cfg(feature = "native")]
			resolver: initialize.resolver,
		})
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			DnsMessage::Source(response) => {
				response.respond(state.source());
			},
			DnsMessage::SourceStream(stream) => {
				let source = state.source();
				state.state.subscribe(stream, source);
			},
			DnsMessage::Refresh => {
				#[cfg(feature = "native")]
				state.resolver.refresh();
			},
			DnsMessage::SourceChanged(source) => {
				state.state.publish(source);
			},
		}
		Ok(())
	}
}

pub struct DnsActorState {
	state: DnsState,
	#[cfg(feature = "native")]
	resolver: Resolver,
}

impl DnsActorState {
	fn source(&self) -> Option<DnsSource> {
		#[cfg(feature = "native")]
		{
			Some(self.resolver.source())
		}
		#[cfg(not(feature = "native"))]
		{
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use co_actor::{Actor, ActorHandle};
	use co_primitives::tags;
	use futures::StreamExt;

	#[tokio::test]
	async fn actor_reports_source_changes() {
		let spawner = DnsActor::spawner(tags!("type": "dns-test"), DnsActor).unwrap();
		let handle = spawner.handle();
		let initialize = DnsInitialize::new(&NetworkDns::None, ActorHandle::new_closed());
		let actor = spawner.spawn(Default::default(), initialize);
		let api = crate::services::dns::DnsApi::from(&actor);

		#[cfg(feature = "native")]
		let expected = Some(DnsSource::Static);
		#[cfg(not(feature = "native"))]
		let expected = None;

		assert_eq!(api.source().await.unwrap(), expected);

		let mut stream = api.source_stream();
		assert_eq!(stream.next().await, Some(expected));

		handle.dispatch(DnsMessage::SourceChanged(DnsSource::System)).unwrap();
		assert_eq!(stream.next().await, Some(Some(DnsSource::System)));

		actor.shutdown();
	}
}
