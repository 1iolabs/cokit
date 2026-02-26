// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_actor::ActorError;
use std::{fmt::Debug, sync::Arc};

#[derive(Clone, thiserror::Error)]
#[error(transparent)]
pub struct CoError(Arc<anyhow::Error>);
impl CoError {
	pub fn new<E: Into<anyhow::Error>>(error: E) -> Self {
		Self(Arc::new(error.into()))
	}
}
impl From<anyhow::Error> for CoError {
	fn from(err: anyhow::Error) -> Self {
		Self::new(err)
	}
}
impl From<ActorError> for CoError {
	fn from(err: ActorError) -> Self {
		Self::new(err)
	}
}
impl Debug for CoError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}
