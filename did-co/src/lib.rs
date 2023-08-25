mod parse;
pub use parse::{parse, ParseError};

pub const DID_CO_METHOD_NAME: &str = "co";
pub const DID_CO_SUB_TYPE_ID: &str = "id";
pub const DID_CO_SUB_TYPE_REFERENCE: &str = "ref";

#[derive(Debug, PartialEq)]
pub enum DidCo {
	/// String: `did:co:id:<genesis_hash>:<name>`
	Id(String, String),
	/// String: `did:co:ref:<reference>:<name>`
	Reference(String, String),
}
