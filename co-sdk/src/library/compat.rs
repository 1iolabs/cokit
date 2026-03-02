// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;
#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;

/// Platform-agnostic sleep for a duration. Send-safe on all platforms.
#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
	tokio::time::sleep(duration).await;
}

/// Platform-agnostic sleep for a duration. Send-safe on all platforms.
#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: Duration) {
	futures_timer::Delay::new(duration).await;
}

/// Platform-agnostic timeout. Send-safe on all platforms.
#[cfg(not(target_arch = "wasm32"))]
pub async fn timeout<F: std::future::Future>(duration: Duration, future: F) -> Result<F::Output, Elapsed> {
	tokio::time::timeout(duration, future).await.map_err(|_| Elapsed)
}

/// Platform-agnostic timeout. Send-safe on all platforms.
#[cfg(target_arch = "wasm32")]
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
#[derive(Debug)]
pub struct Elapsed;

impl std::fmt::Display for Elapsed {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "deadline has elapsed")
	}
}

impl std::error::Error for Elapsed {}
