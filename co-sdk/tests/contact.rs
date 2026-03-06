// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_primitives::{Network, NetworkPeer};
use co_sdk::{Action, Identity};
use futures::StreamExt;
use helper::instance::Instances;
use std::{collections::BTreeMap, time::Duration};
use tokio::time::timeout;

pub mod helper;

/// Contact Request (DID-to-DID)
///
/// Steps:
/// - P1 & P2: Create identities
/// - P1: Send contact request to P2's DID via explicit NetworkPeer
/// - P2: Receive DIDComm message with type "co-contact"
/// - P1: ContactSent succeeds
#[tokio::test]
async fn test_contact() {
	let timeout_duration = Duration::from_secs(60);

	let mut instances = Instances::new("test_contact");
	let mut peer1 = instances.create().await;
	let mut peer2 = instances.create().await;

	// network: start but do NOT dial — the contact flow will connect via did_use
	let (_network1, network2) = Instances::networking(&mut peer1, &mut peer2, false, false).await;

	// create identities
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// build explicit NetworkPeer network pointing to peer2
	let peer2_listeners: Vec<String> = network2
		.listeners(true, false)
		.await
		.unwrap()
		.into_iter()
		.map(|addr| addr.to_string())
		.collect();
	let peer2_network =
		Network::Peer(NetworkPeer { peer: network2.local_peer_id().to_bytes(), addresses: peer2_listeners });

	// peer2: listen for DidCommReceive with "co-contact" message type
	let peer2_receive = {
		let actions = peer2.application.actions();
		async move {
			timeout(
				timeout_duration,
				actions
					.filter_map(|action| async move {
						match action {
							Action::DidCommReceive { peer: _, message }
								if message.header().message_type == "co-contact" =>
							{
								Some(message)
							},
							_ => None,
						}
					})
					.take(1)
					.collect::<Vec<_>>(),
			)
			.await
			.expect("peer2 to receive contact in time")
			.into_iter()
			.next()
			.expect("received contact message")
		}
	};

	// peer1: send contact request
	let peer1_contact = async {
		peer1
			.application
			.context()
			.contact(
				identity1.identity().to_owned(),
				identity2.identity().to_owned(),
				Some("test-token".to_string()),
				BTreeMap::new(),
				[peer2_network],
			)
			.await
			.expect("contact send to succeed")
	};

	// run both concurrently
	let (received_message, ()) = futures::join!(peer2_receive, peer1_contact);

	// verify received message
	let header = received_message.header();
	assert_eq!(header.message_type, "co-contact");
	assert!(header.from.as_ref().is_some_and(|from| from == identity1.identity()));
	assert!(header.to.iter().any(|to| to == identity2.identity()));
}

/// Contact request with no peers available should fail.
#[tokio::test]
async fn test_contact_no_peers() {
	let timeout_duration = Duration::from_secs(30);

	let mut instances = Instances::new("test_contact_no_peers");
	let mut peer1 = instances.create().await;
	let mut peer2 = instances.create().await;

	// start networking but do NOT dial (no connectivity between peers)
	let (_network1, network2) = Instances::networking(&mut peer1, &mut peer2, false, false).await;

	// create identities
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// use a NetworkPeer with no valid addresses (unreachable)
	let unreachable_network =
		Network::Peer(NetworkPeer { peer: network2.local_peer_id().to_bytes(), addresses: vec![] });

	// peer1: send contact request — should fail
	let result = timeout(
		timeout_duration,
		peer1.application.context().contact(
			identity1.identity().to_owned(),
			identity2.identity().to_owned(),
			None,
			BTreeMap::new(),
			[unreachable_network],
		),
	)
	.await
	.expect("should complete in time");

	assert!(result.is_err(), "contact to unreachable peer should fail");
}
