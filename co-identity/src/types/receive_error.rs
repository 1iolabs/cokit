// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_primitives::Did;

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
	#[error("Unknown format")]
	UnknownFormat(#[source] anyhow::Error),
	#[error("Missing skid property")]
	MissingSigningKeyId,
	#[error("Invalid skid property")]
	InvalidSigningKeyId(#[source] anyhow::Error),
	#[error("Decrypt failed")]
	Decrypt(#[source] anyhow::Error),
	#[error("Invalid argument")]
	InvalidArgument(#[source] anyhow::Error),
	#[error("Resolve DID failed: {0}")]
	ResolveDidFailed(Did, #[source] anyhow::Error),
	#[error("Bad DID: {0}")]
	BadDid(Did, #[source] anyhow::Error),
	#[error("No recipent")]
	NoRecipent,
}
