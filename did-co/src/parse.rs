use crate::{DidCo, DID_CO_METHOD_NAME, DID_CO_SUB_TYPE_ID, DID_CO_SUB_TYPE_REFERENCE};
use did_url::DID;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseError {
	InvalidUrl(did_url::Error),
	InvalidMethod,
	InvalidMethodId,
}
impl core::fmt::Display for ParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ParseError::InvalidUrl(e) => write!(f, "Invalid URL: {}", e),
			ParseError::InvalidMethod => write!(f, "Invalid Method. Expected 'co'"),
			ParseError::InvalidMethodId => write!(f, "Invalid Id. Expected <'ref'|'id'>:<reference>:<name>"),
		}
	}
}

pub fn parse(uri: impl AsRef<str>) -> Result<DidCo, ParseError> {
	let did: DID = DID::parse(uri).map_err(|e| ParseError::InvalidUrl(e))?;
	if did.method() != DID_CO_METHOD_NAME {
		return Err(ParseError::InvalidMethod)
	}
	let id: Vec<&str> = did.method_id().split(':').collect();
	match id.as_slice() {
		&[DID_CO_SUB_TYPE_ID, id, name] => Ok(DidCo::Id(id.to_string(), name.to_string())),
		&[DID_CO_SUB_TYPE_REFERENCE, id, name] => Ok(DidCo::Reference(id.to_string(), name.to_string())),
		_ => Err(ParseError::InvalidMethodId),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_id() {
		let url = "did:co:id:d260830e58864b949d500006fa3de3cb:alice";
		let result = parse(&url);
		assert_eq!(
			result.expect("to parse"),
			DidCo::Id("d260830e58864b949d500006fa3de3cb".to_string(), "alice".to_string())
		);
	}

	#[test]
	fn test_parse_ref() {
		let url = "did:co:ref:1io:alice";
		let result = parse(&url);
		assert_eq!(result.expect("to parse"), DidCo::Reference("1io".to_string(), "alice".to_string()));
	}

	#[test]
	fn test_parse_invalid_url() {
		let url = "not_an_did";
		let result = parse(&url);
		assert_eq!(result.expect_err("to parse"), ParseError::InvalidUrl(did_url::Error::InvalidScheme));
	}

	#[test]
	fn test_parse_invalid_method() {
		let url = "did:com:ref:1io:alice";
		let result = parse(&url);
		assert_eq!(result.expect_err("to parse"), ParseError::InvalidMethod);
	}

	#[test]
	fn test_parse_invalid_method_id() {
		let url = "did:co:reference:1io:alice";
		let result = parse(&url);
		assert_eq!(result.expect_err("to parse"), ParseError::InvalidMethodId);
	}

	#[test]
	fn test_parse_invalid_method_id_count() {
		let url = "did:co:ref:1io";
		let result = parse(&url);
		assert_eq!(result.expect_err("to parse"), ParseError::InvalidMethodId);
	}
}
