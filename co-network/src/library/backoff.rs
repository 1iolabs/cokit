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
