use libipld::Cid;

use crate::co_v1::CoV1Api;

pub mod wasmer;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
	#[error("Invalid argument")]
	InvalidArgument,

	#[error("Internal error")]
	Internal,

	#[error("Runtime error")]
	Runtime,
}
impl From<wasmer::WasmerError> for RuntimeError {
	fn from(value: wasmer::WasmerError) -> Self {
		match value {
			wasmer::WasmerError::Compile(_) => Self::InvalidArgument,
			wasmer::WasmerError::Instantiation(_) => Self::InvalidArgument,
			wasmer::WasmerError::Export(_) => Self::InvalidArgument,
			wasmer::WasmerError::Runtime(_) => Self::Runtime,
		}
	}
}

pub fn runtime_execute(api: CoV1Api, bytes: Vec<u8>) -> Result<Option<Cid>, RuntimeError> {
	let mut runtime = wasmer::WasmerRuntime::new(api, bytes)?;
	runtime.execute()?;
	Ok(runtime.api().state().clone())
}
