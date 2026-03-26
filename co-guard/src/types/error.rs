// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_runtime::RuntimeContext;

#[derive(Debug, thiserror::Error)]
pub enum GuardError {
	/// Guard rejected in Skip mode — caller should return this context with diagnostic.
	#[error("Guard skipped: {0}")]
	Skipped(String, RuntimeContext),

	/// Guard rejected in Fail mode.
	#[error("Guard rejected: {0}")]
	Rejected(String),

	/// Guard execution error.
	#[error("Guard execution failed")]
	Execute(#[source] anyhow::Error),
}
