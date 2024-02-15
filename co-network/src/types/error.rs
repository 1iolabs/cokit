use tokio::sync::mpsc::error::SendError;

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
