use libipld::Cid;

use crate::{co_v1::CoV1Api, library::pin::PinMapping};

use self::wasmer::WasmerRuntime;

pub mod wasmer;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
	#[error("Invalid argument")]
	InvalidArgument,

	#[error("Internal error")]
	Internal,

	#[error("Runtime error")]
	Runtime,

	#[error("Invalid runtime state")]
	InvalidState,
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

pub trait Runtime {
	/// Execute runtime with specified api.
	fn execute(&mut self, api: CoV1Api) -> Result<Option<Cid>, RuntimeError>;

	/// Create pin mapping for the last executed state.
	fn pin(&mut self, pin: Option<Cid>) -> Result<Cid, RuntimeError>;
}

enum RuntimeState {
	Unintialized(Vec<u8>),
	Intialized(wasmer::WasmerRuntime),
}

struct Wasmer {
	state: RuntimeState,
}
impl Wasmer {
	pub fn new(bytes: Vec<u8>) -> Self {
		Self { state: RuntimeState::Unintialized(bytes) }
	}
}
impl Runtime for Wasmer {
	fn execute(&mut self, mut api: CoV1Api) -> Result<Option<Cid>, RuntimeError> {
		// initialize
		let runtime: &mut WasmerRuntime = match &mut self.state {
			RuntimeState::Unintialized(bytes) => {
				self.state = RuntimeState::Intialized(wasmer::WasmerRuntime::new(api, bytes)?);
				if let RuntimeState::Intialized(runtime) = &mut self.state {
					runtime
				} else {
					unreachable!("invalid state");
				}
			},
			RuntimeState::Intialized(runtime) => {
				runtime.api_mut().swap(&mut api);
				runtime
			},
		};

		// execute
		runtime.execute()?;
		let result = runtime.api().state().clone();

		// result
		Ok(result)
	}

	fn pin(&mut self, pin: Option<Cid>) -> Result<Cid, RuntimeError> {
		let api = match &mut self.state {
			RuntimeState::Unintialized(_) => return Err(RuntimeError::InvalidState),
			RuntimeState::Intialized(runtime) => runtime.api_mut(),
		};
		let state = api.state().ok_or(RuntimeError::InvalidState)?;
		let mapping = PinMapping::from_state(api.storage_mut(), state, pin).map_err(|_| RuntimeError::Internal)?;
		Ok(mapping.pin)
	}
}

pub fn create_runtime(bytes: Vec<u8>) -> Box<dyn Runtime> {
	Box::new(Wasmer::new(bytes))
}

// #[deprecated]
// pub fn runtime_execute(api: CoV1Api, bytes: Vec<u8>) -> Result<Option<Cid>, RuntimeError> {
// 	let mut runtime = wasmer::WasmerRuntime::new(api, &bytes)?;
// 	runtime.execute()?;
// 	Ok(runtime.api().state().clone())
// }
