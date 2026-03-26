// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

/// Guard rejection mode.
#[derive(Debug, Clone, Copy)]
pub enum GuardRejectionMode {
	/// Ignore rejection and just trace a warning.
	Ignore,
	/// Skip the computation and insert a diagnostic message.
	Skip,
	/// Fail the operation hard.
	Fail,
}
