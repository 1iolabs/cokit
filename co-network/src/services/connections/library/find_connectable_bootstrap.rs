// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::services::connections::{state::BootstrapPeer, ConnectionState};
use co_actor::time::Instant;
use std::{cmp::min, time::Duration};

pub fn find_connectable_bootstrap(
	state: &ConnectionState,
	now: Instant,
	backoff: impl Fn(u32) -> Duration,
) -> Result<BootstrapPeer, Option<Instant>> {
	let mut next_attempt = None;
	for (peer, bootstrap) in state.bootstrap.iter() {
		// connecting?
		if bootstrap.connecting {
			continue;
		}

		// connected?
		let connected = state
			.peers
			.get(peer)
			.map(|peer_connection| peer_connection.connected)
			.unwrap_or(false);
		if connected {
			continue;
		}

		// backoff
		if let Some(failed_at) = bootstrap.failed_at {
			let blocked_until = failed_at + backoff(bootstrap.failed);
			if blocked_until > now {
				next_attempt = Some(match next_attempt {
					None => blocked_until,
					Some(next_attempt) => min(blocked_until, next_attempt),
				});
				continue;
			}
		}

		// ok
		return Ok(bootstrap.clone());
	}
	Err(next_attempt)
}
