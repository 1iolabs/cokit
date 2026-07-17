// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

pub use crate::services::application::{
	HeadsDeliveryCompleteAction, HeadsDeliveryOutcome, HeadsDeliveryPhase, HeadsRecipient, PushHeadsToDidsAction,
};
use crate::{Action, CoContext};
use co_actor::ActorError;
use co_primitives::{CoConnectivity, CoId, Did};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct HeadsDeliveryIntent {
	pub co: CoId,
	pub from: Did,
	pub recipient: Did,
	pub connectivity: CoConnectivity,
}

pub fn push_heads_to_dids(
	context: &CoContext,
	co: CoId,
	from: Did,
	recipients: impl IntoIterator<Item = HeadsRecipient>,
) -> Result<(), ActorError> {
	let recipients = merge_heads_recipients(recipients);
	if recipients.is_empty() {
		return Ok(());
	}
	context
		.inner
		.application()
		.dispatch(Action::PushHeadsToDids(PushHeadsToDidsAction { co, from, recipients }))
}

pub(crate) fn merge_heads_recipients(recipients: impl IntoIterator<Item = HeadsRecipient>) -> Vec<HeadsRecipient> {
	let mut merged: BTreeMap<Did, CoConnectivity> = BTreeMap::new();
	for recipient in recipients {
		let connectivity = merged.entry(recipient.did).or_default();
		connectivity.network.extend(recipient.connectivity.network);
		connectivity.participants.extend(recipient.connectivity.participants);
	}
	merged
		.into_iter()
		.map(|(did, connectivity)| HeadsRecipient { did, connectivity })
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use co_primitives::{BlockSerializer, Network, NetworkPeer};

	#[test]
	fn intent_round_trips() {
		let intent = HeadsDeliveryIntent {
			co: CoId::from("co-test"),
			from: Did::from("did:key:sender"),
			recipient: Did::from("did:key:target"),
			connectivity: CoConnectivity::default(),
		};
		let serializer = BlockSerializer::default();
		let block = serializer.serialize(&intent).unwrap();
		let decoded = serializer.deserialize::<HeadsDeliveryIntent>(&block).unwrap();

		assert_eq!(decoded, intent);
	}

	#[test]
	fn duplicate_recipients_merge_connectivity() {
		let did = Did::from("did:key:target");
		let peer_a = Network::Peer(NetworkPeer { peer: vec![1], addresses: vec![] });
		let peer_b = Network::Peer(NetworkPeer { peer: vec![2], addresses: vec![] });
		let recipients = merge_heads_recipients([
			HeadsRecipient {
				did: did.clone(),
				connectivity: CoConnectivity {
					network: [peer_a.clone()].into_iter().collect(),
					participants: [Did::from("did:key:hint-a")].into_iter().collect(),
				},
			},
			HeadsRecipient {
				did: did.clone(),
				connectivity: CoConnectivity {
					network: [peer_b.clone()].into_iter().collect(),
					participants: [Did::from("did:key:hint-b")].into_iter().collect(),
				},
			},
		]);
		assert_eq!(recipients.len(), 1);
		assert_eq!(recipients[0].did, did);
		assert_eq!(recipients[0].connectivity.network, [peer_a, peer_b].into_iter().collect());
		assert_eq!(
			recipients[0].connectivity.participants,
			[Did::from("did:key:hint-a"), Did::from("did:key:hint-b")].into_iter().collect()
		);
	}
}
