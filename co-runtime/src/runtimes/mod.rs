use self::wasmer::WasmerRuntime;
use crate::{co_v1::CoV1Api, RuntimeContext};

pub mod wasmer;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
	#[error("Invalid argument")]
	InvalidArgument(#[source] anyhow::Error),

	#[error("Internal core error")]
	Internal(#[source] anyhow::Error),

	#[error("Runtime core error")]
	Runtime(#[source] anyhow::Error),

	#[error("Invalid runtime state")]
	InvalidState(#[source] anyhow::Error),
}
impl From<wasmer::WasmerError> for RuntimeError {
	fn from(value: wasmer::WasmerError) -> Self {
		match value {
			wasmer::WasmerError::Compile(e) => Self::InvalidArgument(e.into()),
			wasmer::WasmerError::Instantiation(e) => Self::InvalidArgument(e.into()),
			wasmer::WasmerError::Export(e) => Self::InvalidArgument(e.into()),
			wasmer::WasmerError::Runtime(e) => Self::Runtime(e.into()),
		}
	}
}

pub trait Runtime {
	/// Execute state runtime with specified api.
	fn execute_state(&mut self, api: CoV1Api) -> Result<RuntimeContext, RuntimeError>;

	/// Execute guard runtime with specified api.
	fn execute_guard(&mut self, api: CoV1Api) -> Result<bool, RuntimeError>;
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
	/// Execute runtime with api and return new state `Cid`.
	fn execute_state(&mut self, api: CoV1Api) -> Result<RuntimeContext, RuntimeError> {
		// initialize
		let runtime: &mut WasmerRuntime = wasmer_runtime_with_api(&mut self.state, api)?;

		// execute
		runtime.execute_state()?;
		let result = runtime.api().context().clone();

		// result
		Ok(result)
	}

	fn execute_guard(&mut self, api: CoV1Api) -> Result<bool, RuntimeError> {
		// initialize
		let runtime: &mut WasmerRuntime = wasmer_runtime_with_api(&mut self.state, api)?;

		// execute
		let result = runtime.execute_guard()?;

		// result
		Ok(result)
	}
}

fn wasmer_runtime_with_api(state: &mut RuntimeState, mut api: CoV1Api) -> Result<&mut WasmerRuntime, RuntimeError> {
	// initialize
	let runtime: &mut WasmerRuntime = match state {
		RuntimeState::Unintialized(bytes) => {
			*state = RuntimeState::Intialized(wasmer::WasmerRuntime::new(api, bytes)?);
			if let RuntimeState::Intialized(runtime) = state {
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
	Ok(runtime)
}

pub fn create_runtime(bytes: Vec<u8>) -> Box<dyn Runtime + Send> {
	Box::new(Wasmer::new(bytes))
}

// #[deprecated]
// pub fn runtime_execute(api: CoV1Api, bytes: Vec<u8>) -> Result<Option<Cid>, RuntimeError> {
// 	let mut runtime = wasmer::WasmerRuntime::new(api, &bytes)?;
// 	runtime.execute()?;
// 	Ok(runtime.api().state().clone())
// }
