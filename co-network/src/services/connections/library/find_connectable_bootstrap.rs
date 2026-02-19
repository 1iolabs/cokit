// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::services::connections::{state::BootstrapPeer, ConnectionState};
use std::{
	cmp::min,
	time::{Duration, Instant},
};

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
