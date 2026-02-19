// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_api::{co, Did};
use std::collections::BTreeMap;

#[co]
#[derive(Default)]
pub struct Config {
	pub types: BTreeMap<String, RecordTypeConfig>,
}

#[co]
pub struct RecordTypeConfig {
	/// Only allow `Did` to create records of this type.
	/// If None every identity can create records (based on limits).
	pub creator: Option<Did>,

	/// Creation limits.
	pub limit: RecordTypeLimit,
}

#[co]
#[derive(Default)]
pub enum RecordTypeLimit {
	/// No limit.
	#[default]
	None,

	/// Only a specific count by identity (DID).
	ByIdentity(u16),

	/// Only a specific count per record (and identity) that already exists.
	/// Example: One CoRecord for every name record.
	ByRecord(u16, String),
}
