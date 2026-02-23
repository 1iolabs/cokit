use self::wasmer::WasmerRuntime;
use crate::{co_v1::CoV1Api, RuntimeContext};
use anyhow::anyhow;
use std::fmt::Debug;

// pub mod local;
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

	#[error("Deserialize binary error")]
	Deserialize(#[source] anyhow::Error),
}
impl From<wasmer::WasmerError> for RuntimeError {
	fn from(value: wasmer::WasmerError) -> Self {
		match value {
			wasmer::WasmerError::Compile(e) => Self::InvalidArgument(e.into()),
			wasmer::WasmerError::Instantiation(e) => Self::InvalidArgument(e.into()),
			wasmer::WasmerError::Export(e) => Self::InvalidArgument(e.into()),
			wasmer::WasmerError::Runtime(e) => Self::Runtime(e.into()),
			wasmer::WasmerError::Deserialize(e) => Self::Deserialize(e.into()),
			e @ wasmer::WasmerError::NoEngineAvailable => Self::InvalidArgument(e.into()),
		}
	}
}

pub trait Runtime: Debug + 'static {
	/// Execute state runtime with specified api.
	fn execute_state(&mut self, api: CoV1Api) -> Result<RuntimeContext, RuntimeError>;

	/// Execute guard runtime with specified api.
	fn execute_guard(&mut self, api: CoV1Api) -> Result<bool, RuntimeError>;
}

#[cfg(not(feature = "js"))]
pub type RuntimeBox = Box<dyn Runtime + Send>;
#[cfg(feature = "js")]
pub type RuntimeBox = Box<dyn Runtime>;

enum RuntimeState {
	Unintialized(bool, Vec<u8>),
	Intialized(Box<wasmer::WasmerRuntime>),
}
impl Debug for RuntimeState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Unintialized(arg0, arg1) => f.debug_tuple("Unintialized").field(arg0).field(&arg1.len()).finish(),
			Self::Intialized(arg0) => f.debug_tuple("Intialized").field(arg0).finish(),
		}
	}
}

#[derive(Debug)]
struct Wasmer {
	state: RuntimeState,
}
impl Wasmer {
	pub fn new(native: bool, bytes: Vec<u8>) -> Self {
		Self { state: RuntimeState::Unintialized(native, bytes) }
	}
}
impl Runtime for Wasmer {
	/// Execute runtime with api and return new state `Cid`.
	fn execute_state(&mut self, api: CoV1Api) -> Result<RuntimeContext, RuntimeError> {
		// initialize
		let runtime: &mut WasmerRuntime = wasmer_runtime(&mut self.state)?;

		// execute
		let result = runtime.execute_state(api)?;

		// result
		Ok(result)
	}

	fn execute_guard(&mut self, api: CoV1Api) -> Result<bool, RuntimeError> {
		// initialize
		let runtime: &mut WasmerRuntime = wasmer_runtime(&mut self.state)?;

		// execute
		let result = runtime.execute_guard(api)?;

		// result
		Ok(result)
	}
}

fn wasmer_runtime(state: &mut RuntimeState) -> Result<&mut WasmerRuntime, RuntimeError> {
	// initialize
	let runtime: &mut WasmerRuntime = match state {
		RuntimeState::Unintialized(native, bytes) => {
			tracing::trace!("deferred-runtime-new");
			*state = RuntimeState::Intialized(Box::new(wasmer::WasmerRuntime::new(*native, bytes)?));
			if let RuntimeState::Intialized(runtime) = state {
				runtime
			} else {
				return Err(RuntimeError::InvalidState(anyhow!("Uninitialized after initialize")));
			}
		},
		RuntimeState::Intialized(runtime) => {
			tracing::trace!("deferred-runtime-intialized");
			runtime
		},
	};
	Ok(runtime)
}

pub fn create_runtime(native: bool, bytes: Vec<u8>) -> RuntimeBox {
	Box::new(Wasmer::new(native, bytes))
}

// #[deprecated]
// pub fn runtime_execute(api: CoV1Api, bytes: Vec<u8>) -> Result<Option<Cid>, RuntimeError> {
// 	let mut runtime = wasmer::WasmerRuntime::new(api, &bytes)?;
// 	runtime.execute()?;
// 	Ok(runtime.api().state().clone())
// }
