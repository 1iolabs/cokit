// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_primitives::DynamicCoDate;

/// Get co date for the current environment
#[allow(unreachable_code)]
pub fn co_date_env() -> DynamicCoDate {
	// js
	#[cfg(feature = "js")]
	return DynamicCoDate::new(crate::JsCoDate);

	// native
	#[cfg(feature = "native")]
	return DynamicCoDate::new(crate::SystemCoDate);

	// unknown
	unreachable!("CoDate not supported on this platform")
}
