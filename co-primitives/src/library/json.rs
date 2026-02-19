// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use ipld_core::{ipld::Ipld, serde::to_ipld};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Serialize `value` to JSON string (using dag-json).
pub fn to_json<T: Serialize>(value: &T) -> Result<Vec<u8>, JsonError> {
	let ipld =
		to_ipld(value).map_err(|err| JsonError::Serialize(std::any::type_name::<T>().to_owned(), err.to_string()))?;
	serde_ipld_dagjson::to_vec(&ipld)
		.map_err(|err| JsonError::Serialize(std::any::type_name::<T>().to_owned(), err.to_string()))
}

/// Deserialize from JSON string (using dag-json).
pub fn from_json<'a, T: Deserialize<'a>>(value: &'a [u8]) -> Result<T, JsonError> {
	// because of an bug in `serde_ipld_dagjson` we take an extra step via Ipld to make bytes work correctly
	let ipld: Ipld = serde_ipld_dagjson::from_slice(value)
		.map_err(|err| JsonError::Deserialize(std::any::type_name::<T>().to_owned(), err.to_string()))?;
	T::deserialize(ipld).map_err(|err| JsonError::Deserialize(std::any::type_name::<T>().to_owned(), err.to_string()))
}

/// Serialize `value` to JSON string (using dag-json).
pub fn to_json_string<T: Serialize>(value: &T) -> Result<String, JsonError> {
	String::from_utf8(to_json(value)?)
		.map_err(|err| JsonError::Serialize(std::any::type_name::<T>().to_owned(), err.to_string()))
}

/// Deserialize from JSON string (using dag-json).
pub fn from_json_string<T: DeserializeOwned>(value: impl AsRef<[u8]>) -> Result<T, JsonError> {
	from_json(value.as_ref())
		.map_err(|err| JsonError::Deserialize(std::any::type_name::<T>().to_owned(), err.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum JsonError {
	#[error("Serialize {0} to JSON failed: {1}")]
	Serialize(String, String),

	#[error("Deserialize {0} from JSON failed: {1}")]
	Deserialize(String, String),
}

#[cfg(test)]
mod tests {
	use crate::{from_json, to_json};
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Serialize, Deserialize, PartialEq)]
	struct Test {
		#[serde(with = "serde_bytes")]
		hello: Vec<u8>,
	}

	#[test]
	fn test_bytes() {
		let payload = Test { hello: "world".as_bytes().to_vec() };
		let json = to_json(&payload).unwrap();
		assert_eq!(std::str::from_utf8(&json).unwrap(), "{\"hello\":{\"/\":{\"bytes\":\"d29ybGQ\"}}}");
		let deserialized: Test = from_json(&json).unwrap();
		assert_eq!(deserialized, payload);
	}
}
