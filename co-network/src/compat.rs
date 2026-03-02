use std::time::Duration;
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

/// Platform-agnostic timeout. Send-safe on all platforms.
#[cfg(not(feature = "web"))]
pub async fn timeout<F: std::future::Future>(
	duration: Duration,
	future: F,
) -> Result<F::Output, tokio::time::error::Elapsed> {
	tokio::time::timeout(duration, future).await
}

/// Platform-agnostic timeout. Send-safe on all platforms.
#[cfg(feature = "web")]
pub async fn timeout<F: std::future::Future>(duration: Duration, future: F) -> Result<F::Output, Elapsed> {
	use futures::future::Either;
	use std::pin::pin;

	let delay = pin!(futures_timer::Delay::new(duration));
	let future = pin!(future);

	match futures::future::select(future, delay).await {
		Either::Left((output, _)) => Ok(output),
		Either::Right((_, _)) => Err(Elapsed),
	}
}

/// Error returned when a timeout expires.
#[cfg(feature = "web")]
#[derive(Debug)]
pub struct Elapsed;

#[cfg(feature = "web")]
impl std::fmt::Display for Elapsed {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "deadline has elapsed")
	}
}

#[cfg(feature = "web")]
impl std::error::Error for Elapsed {}
