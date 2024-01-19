use did_url::DID;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ResolveError {
	/// Invalid URI format.
	#[error("Invalid URI format.")]
	InvalidUri,

	/// The method has not been implemented.
	#[error("The method has not been implemented.")]
	UnsupportedMethod,

	/// The method reported an error while resolving the DID Document.
	#[error("The method reported an error while resolving the DID Document.")]
	Resolve,
}

pub enum ResolveResult {
	Key(did_key::PatchedKeyPair),
}

/// Resolve `did` rendenzvoud point string, if one.
pub async fn resolve(did: &str) -> Result<ResolveResult, ResolveError> {
	let uri = DID::parse(did).map_err(|_e| ResolveError::InvalidUri)?;
	match uri.method() {
		"key" => {
			let result = did_key::resolve(did).map_err(|e| {
				// log
				tracing::warn!(err = ?e, "did-resolve-failed");

				// err
				ResolveError::Resolve
			})?;
			Ok(ResolveResult::Key(result))
		},
		_ => Err(ResolveError::UnsupportedMethod),
	}
}
