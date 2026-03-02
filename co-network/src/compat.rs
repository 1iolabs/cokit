#[cfg(not(feature = "web"))]
pub use std::time::Instant;
#[cfg(feature = "web")]
pub use web_time::Instant;

/// platform-agnostic sleep until a deadline
#[cfg(not(feature = "web"))]
pub async fn sleep_until(deadline: Instant) {
	tokio::time::sleep_until(deadline.into()).await;
}

/// platform-agnostic sleep until a deadline (tokio_with_wasm Sleep is !Send)
#[cfg(feature = "web")]
pub async fn sleep_until(deadline: Instant) {
	let now = Instant::now();
	if let Some(duration) = deadline.checked_duration_since(now) {
		futures_timer::Delay::new(duration).await;
	}
}
