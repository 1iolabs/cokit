// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{co_v1::CoV1Api, RuntimeContext};
use std::fmt::Debug;

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

pub trait Runtime: Debug + 'static {
	/// Execute state runtime with specified api.
	fn execute_state(&mut self, api: CoV1Api) -> Result<RuntimeContext, RuntimeError>;

	/// Execute guard runtime with specified api.
	fn execute_guard(&mut self, api: CoV1Api) -> Result<(RuntimeContext, bool), RuntimeError>;
}

#[cfg(not(feature = "js"))]
pub type RuntimeBox = Box<dyn Runtime + Send>;
#[cfg(feature = "js")]
pub type RuntimeBox = Box<dyn Runtime>;
