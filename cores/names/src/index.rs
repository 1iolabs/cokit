// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::RecordId;
use co_api::{co, CoMap, CoSet, Link, TagValue};

#[co]
pub struct IndexKey {
	pub record_type: String,
	pub name: String,
}

#[co]
pub struct IndexConfig {
	pub field: String,
	pub unique: bool,
}

#[co]
pub struct Index {
	/// The index config.
	/// Because of the (possibly) frequent changes of `index` we store this as link.
	pub config: Link<IndexConfig>,

	/// The index.
	pub index: CoMap<TagValue, CoSet<RecordId>>,
}
