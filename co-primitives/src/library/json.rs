use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Serialize `value` to JSON string (using dag-json).
pub fn to_json<T: Serialize>(value: &T) -> Result<Vec<u8>, JsonError> {
	Ok(serde_ipld_dagjson::to_vec(value)
		.map_err(|err| JsonError::Serialize(std::any::type_name::<T>().to_owned(), err.to_string()))?)
}

/// Deserialize from JSON string (using dag-json).
pub fn from_json<'a, T: Deserialize<'a>>(value: &'a [u8]) -> Result<T, JsonError> {
	Ok(serde_ipld_dagjson::from_slice(value)
		.map_err(|err| JsonError::Deserialize(std::any::type_name::<T>().to_owned(), err.to_string()))?)
}

/// Serialize `value` to JSON string (using dag-json).
pub fn to_json_string<T: Serialize>(value: &T) -> Result<String, JsonError> {
	Ok(String::from_utf8(to_json(value)?)
		.map_err(|err| JsonError::Serialize(std::any::type_name::<T>().to_owned(), err.to_string()))?)
}

/// Deserialize from JSON string (using dag-json).
pub fn from_json_string<T: DeserializeOwned>(value: impl AsRef<[u8]>) -> Result<T, JsonError> {
	Ok(from_json(value.as_ref())
		.map_err(|err| JsonError::Deserialize(std::any::type_name::<T>().to_owned(), err.to_string()))?)
}

#[derive(Debug, thiserror::Error)]
pub enum JsonError {
	#[error("Serialize {0} to JSON failed: {1}")]
	Serialize(String, String),

	#[error("Deserialize {0} from JSON failed: {1}")]
	Deserialize(String, String),
}
