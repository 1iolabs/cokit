// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_sdk::CoId;

/// Builds the ID for a core in a co. Can be used to then identify a unique core in a specified co.
/// Can be used to specify a subscription target
pub fn build_core_id(co: CoId, core: Option<&str>) -> String {
	match core {
		Some(core) => format!("{co}/{core}"),
		None => co.to_string(),
	}
}
