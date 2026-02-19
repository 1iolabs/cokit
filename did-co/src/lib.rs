// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
