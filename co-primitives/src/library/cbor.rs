use serde::{Deserialize, Serialize};

/// Serialize `value` to CBOR string (using dag-json).
pub fn to_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, CborError> {
	serde_ipld_dagcbor::to_vec(value)
		.map_err(|err| CborError::Serialize(std::any::type_name::<T>().to_owned(), err.to_string()))
}

/// Deserialize from CBOR (using dag-json).
pub fn from_cbor<'a, T: Deserialize<'a>>(value: &'a [u8]) -> Result<T, CborError> {
	serde_ipld_dagcbor::from_slice(value)
		.map_err(|err| CborError::Deserialize(std::any::type_name::<T>().to_owned(), err.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum CborError {
	#[error("Serialize {0} to CBOR failed: {1}")]
	Serialize(String, String),

	#[error("Deserialize {0} from CBOR failed: {1}")]
	Deserialize(String, String),
}
