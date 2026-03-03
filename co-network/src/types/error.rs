// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use tokio::sync::mpsc::error::SendError;
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
	#[error("Shutdown in progress. Operation canceled.")]
	Shutdown,
}
impl<T> From<SendError<T>> for NetworkError {
	fn from(_: SendError<T>) -> Self {
		Self::Shutdown
	}
}
