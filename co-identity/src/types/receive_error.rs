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
	BadDid(Did),
}
