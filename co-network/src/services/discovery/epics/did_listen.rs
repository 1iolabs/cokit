// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::services::discovery::{
	action::{DidDecryptedAction, DiscoveryAction, GossipMessageAction},
	actor::DiscoveryContext,
	state::{DidDiscoverySubscription, DiscoveryState},
	DidDiscoveryMessageType, DiscoverMessage,
};
use anyhow::anyhow;
use co_actor::{Actions, Epic};
use co_identity::{DidCommContext, DidCommHeader, DidCommPrivateContext, IdentityResolver, PrivateIdentity};
use co_primitives::{from_json_string, CoDateRef, DynamicCoDate};
use futures::{FutureExt, Stream, StreamExt};
use libp2p::PeerId;
use std::{collections::BTreeSet, str::from_utf8};

/// Handles `GossipMessage` by decrypting incoming DID discovery messages.
pub struct DidListenEpic;
impl DidListenEpic {
	pub fn new() -> Self {
		Self
	}
}
impl Epic<DiscoveryAction, DiscoveryState, DiscoveryContext> for DidListenEpic {
	fn epic(
		&mut self,
		_actions: &Actions<DiscoveryAction, DiscoveryState, DiscoveryContext>,
		action: &DiscoveryAction,
		state: &DiscoveryState,
		context: &DiscoveryContext,
	) -> Option<impl Stream<Item = Result<DiscoveryAction, anyhow::Error>> + Send + 'static> {
		let DiscoveryAction::GossipMessage(GossipMessageAction { topic, source, data }) = action else {
			return None;
		};

		let from_peer = (*source)?;

		// check if this topic has subscriptions.
		let subscriptions = state.did_subscriptions.get(topic)?;

		// parse data as UTF-8.
		let data_str = match from_utf8(data) {
			Ok(s) => s.to_owned(),
			Err(_err) => {
				#[cfg(debug_assertions)]
				tracing::debug!(err = ?_err, "discovery-receive-invalid-message");
				return None;
			},
		};

		// extract private contexts from subscriptions.
		let contexts: Vec<DidCommPrivateContext> = subscriptions
			.iter()
			.filter_map(|subscription| match subscription {
				DidDiscoverySubscription::Default => None,
				DidDiscoverySubscription::Identity(_, identity) => identity.didcomm_private(),
			})
			.collect();

		if contexts.is_empty() {
			return None;
		}

		let date = context.date.clone();
		let resolver = context.resolver.clone();

		Some(
			async move { did_discovery_receive(date, data_str, from_peer, resolver, contexts).await }
				.into_stream()
				.filter_map(|action| async move { action.map(Ok) }),
		)
	}
}

/// Accept DID discovery messages and respond with a resolve response.
async fn did_discovery_receive<R: IdentityResolver>(
	date: DynamicCoDate,
	data: String,
	request_from_peer: PeerId,
	resolver: R,
	contexts: Vec<DidCommPrivateContext>,
) -> Option<DiscoveryAction> {
	let result = didcomm_receive(&data, resolver, contexts.into_iter()).await;
	if let Some((request_header, request_body, didcomm_private)) = result {
		if DidDiscoveryMessageType::from_str(&request_header.message_type) == Some(DidDiscoveryMessageType::Discover) {
			let body: Option<DiscoverMessage> = from_json_string(&request_body).ok();
			match did_discovery_resolve(
				&date,
				&didcomm_private,
				request_from_peer,
				body.map(|body| body.endpoints).unwrap_or_default(),
				request_header,
			) {
				Ok(action) => return Some(action),
				Err(err) => {
					tracing::warn!(?err, "discovery-did-resolve-failed");
				},
			}
		}
	}
	None
}

/// Create a resolve response for a DID discovery request.
fn did_discovery_resolve(
	date: &CoDateRef,
	identity: &DidCommPrivateContext,
	request_from_peer: PeerId,
	request_from_endpoints: BTreeSet<libp2p::Multiaddr>,
	request: DidCommHeader,
) -> Result<DiscoveryAction, anyhow::Error> {
	let request_from = request.from.ok_or(anyhow!("Missing from header field"))?;

	let mut response = DidCommHeader::new(date, DidDiscoveryMessageType::Resolve.to_string());
	response.thid = Some(request.id);
	response.from = Some(identity.did().to_owned());
	response.to.insert(request_from.clone());

	let message = identity.jws(response, "null")?;

	Ok(DiscoveryAction::DidDecrypted(DidDecryptedAction {
		from_did: Some(request_from.clone()),
		from_peer: request_from_peer,
		from_endpoints: request_from_endpoints,
		response: message,
	}))
}

/// Try to receive a message with one of the supplied identities.
async fn didcomm_receive<R: IdentityResolver>(
	data: &str,
	resolver: R,
	contexts: impl Iterator<Item = DidCommPrivateContext>,
) -> Option<(DidCommHeader, String, DidCommPrivateContext)> {
	for didcomm_private in contexts {
		match didcomm_private.receive(&resolver, data).await {
			Ok((header, body)) => return Some((header, body, didcomm_private)),
			Err(_err) => {
				#[cfg(debug_assertions)]
				tracing::debug!(err = ?_err, ?data, "jwe-receive-failed");
			},
		}
	}
	None
}
