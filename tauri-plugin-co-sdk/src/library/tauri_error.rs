use tauri::ipc::InvokeError;

/// Error type that builds a bridge between anyhow and tauri::ipc::InvokeError
#[derive(Debug)]
pub struct CoTauriError {
	error: anyhow::Error,
}

impl From<CoTauriError> for InvokeError {
	fn from(val: CoTauriError) -> Self {
		InvokeError::from_anyhow(val.error)
	}
}

impl<T> From<T> for CoTauriError
where
	T: Into<anyhow::Error>,
{
	fn from(error: T) -> Self {
		Self { error: error.into() }
	}
}
