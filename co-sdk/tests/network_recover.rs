// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

pub mod helper;

use helper::instance::Instances;

/// `recover()` is callable, idempotent, preserves the NetworkApi + PeerId, and leaves a
/// live listener.
#[co_test::timeout(10000)]
#[tokio::test]
async fn network_recover_is_idempotent_and_preserves_identity() {
	let mut instances = Instances::new("network-recover");
	let mut peer = instances.create().await;

	// start network on loopback
	// mDNS on (overriding with_localhost's default-off) so recover() actually exercises the
	// in-place mDNS swap; this peer never dials, so same-machine mDNS cannot wedge it.
	peer.application
		.create_network(co_sdk::NetworkSettings::default().with_localhost().with_mdns(true))
		.await
		.unwrap();

	let network = peer.application.context().network().await.unwrap();
	let peer_id_before = network.local_peer_id();
	let listeners_before = network.listeners(true, false).await.unwrap();
	assert!(!listeners_before.is_empty(), "expected a listener before recovery");

	// recover (twice, to assert repeat-safety)
	peer.application.network_recover().await.unwrap();
	peer.application.network_recover().await.unwrap();

	// NetworkApi is preserved (same handle still answers), PeerId is stable
	let network_after = peer.application.context().network().await.unwrap();
	assert_eq!(network_after.local_peer_id(), peer_id_before, "PeerId must be stable across recovery");

	// a listener is present after recovery (re-listen produced a fresh one)
	let listeners_after = network_after.listeners(true, false).await.unwrap();
	assert!(!listeners_after.is_empty(), "expected a listener after recovery");
}

/// After recovery, a peer can still establish a fresh connection and the network is usable.
#[co_test::timeout(10000)]
#[tokio::test]
async fn network_usable_after_recover() {
	let mut instances = Instances::new("network-recover-sync");
	let mut peer1 = instances.create().await;
	let mut peer2 = instances.create().await;

	// start both, no initial dial
	let (network1, _network2) = Instances::networking(&mut peer1, &mut peer2, false, false).await;

	// recover peer1 (its listener address changes to a fresh ephemeral port)
	peer1.application.network_recover().await.unwrap();

	// peer2 dials peer1 on its current (post-recovery) listener addresses
	let network1_after = peer1.application.context().network().await.unwrap();
	let network2 = peer2.application.context().network().await.unwrap();
	let addrs: Vec<_> = network1_after.listeners(true, false).await.unwrap().into_iter().collect();
	network2
		.dial(Some(network1_after.local_peer_id()), addrs)
		.await
		.expect("dial after recovery should succeed");

	// sanity: peer1 identity is unchanged
	assert_eq!(network1_after.local_peer_id(), network1.local_peer_id());
}
