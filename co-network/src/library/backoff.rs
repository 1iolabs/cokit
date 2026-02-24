// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use rand::Rng;
use std::time::Duration;

pub fn backoff(retry: u32) -> Duration {
	let base = Duration::from_secs(3000);
	let max = Duration::from_secs(60);
	let backoff = base * 2u32.pow(retry.min(10)); // cap to avoid overflow
	std::cmp::min(backoff, max)
}

pub fn backoff_with_jitter(retry: u32) -> Duration {
	let duration = backoff(retry);
	let min_ns = 0;
	let max_ns = duration.as_nanos();
	let rand_ns = rand::thread_rng().gen_range(min_ns..=max_ns);
	Duration::from_nanos(rand_ns as u64)
}
