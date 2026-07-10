// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

pub mod helper;

use helper::instance::Instances;
use std::time::{Duration, Instant};

/// A peer that shuts down while its counterparty keeps the connection open must still complete
/// `shutdown_application()` promptly. Previously this hung until the outer test timeout because
/// the network and runtime actors were never torn down on app-cancel. A clean drain completes
/// well under the sub-3s bound; the tight bound fails loudly if shutdown ever regresses to a
/// slow fallback or a hang.
#[co_test::timeout(10000)]
#[tokio::test]
async fn shutdown_completes_while_counterparty_stays_connected() {
	let mut instances = Instances::new("shutdown-hang");
	let mut peer1 = instances.create().await;
	let mut peer2 = instances.create().await;

	// establish a live connection: peer1 dials peer2; peer2 does not dial back
	let (_network1, _network2) = Instances::networking(&mut peer1, &mut peer2, true, false).await;

	// shut peer1 down while peer2 stays up, holding its side of the connection open
	let started = Instant::now();
	peer1.application.shutdown_application().await;
	let elapsed = started.elapsed();

	// keep peer2 alive across the assertion so the connection stayed open during shutdown
	assert!(elapsed < Duration::from_secs(3), "shutdown_application() took {elapsed:?}; expected a clean drain in ~1s");
	drop(peer2);
}

/// The all-peers-at-once path (mirrors `SharedCo::shutdown`): shutting every connected peer
/// down concurrently must also complete promptly. Like the lone-peer case, this hung before the
/// fix (the runtime actor blocked every shutdown path, regardless of connection symmetry); it
/// guards the concurrent/mutual-close path against regression.
#[co_test::timeout(10000)]
#[tokio::test]
async fn shutdown_completes_for_all_peers_concurrently() {
	let mut instances = Instances::new("shutdown-all");
	let mut peer1 = instances.create().await;
	let mut peer2 = instances.create().await;

	let (_network1, _network2) = Instances::networking(&mut peer1, &mut peer2, true, false).await;

	let started = Instant::now();
	tokio::join!(peer1.application.shutdown_application(), peer2.application.shutdown_application(),);
	let elapsed = started.elapsed();

	assert!(
		elapsed < Duration::from_secs(3),
		"concurrent shutdown_application() took {elapsed:?}; expected a clean drain in ~1s"
	);
}

/// A peer that never started networking must also shut down promptly. This isolates the runtime
/// actor: it is spawned on the tracked task spawner and blocks `tasks.tracker().wait()` even with
/// no network at all, so this guards the unconditional runtime teardown independently of any
/// network path.
#[co_test::timeout(10000)]
#[tokio::test]
async fn shutdown_completes_with_no_network() {
	let mut instances = Instances::new("shutdown-no-network");
	let peer = instances.create().await;

	let started = Instant::now();
	peer.application.shutdown_application().await;
	let elapsed = started.elapsed();

	assert!(
		elapsed < Duration::from_secs(3),
		"shutdown_application() with no network took {elapsed:?}; expected a clean drain"
	);
}
