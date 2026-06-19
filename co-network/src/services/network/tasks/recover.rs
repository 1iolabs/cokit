// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{network::Behaviour, types::network_task::NetworkTask};
use libp2p::Swarm;

/// Network recovery task.
#[derive(Debug)]
pub struct RecoverTask;

impl NetworkTask<Behaviour> for RecoverTask {
	#[cfg_attr(not(feature = "native"), allow(unused_variables))]
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>) {
		// mDNS is a discovery-only behaviour with no per-connection state, so replacing it rebinds
		// the socket and re-announces without disturbing existing connections.
		#[cfg(feature = "native")]
		if swarm.behaviour().mdns.is_enabled() {
			use libp2p::mdns::{self, tokio::Behaviour as MdnsBehaviour};

			let local_peer_id = *swarm.local_peer_id();
			match MdnsBehaviour::new(mdns::Config::default(), local_peer_id) {
				Ok(mdns) => {
					swarm.behaviour_mut().mdns = Some(mdns).into();
					tracing::info!("network-recover-mdns");
				},
				Err(err) => tracing::warn!(?err, "network-recover-mdns-failed"),
			}
		}
	}
}
