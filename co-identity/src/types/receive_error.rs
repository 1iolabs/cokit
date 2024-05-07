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
}
