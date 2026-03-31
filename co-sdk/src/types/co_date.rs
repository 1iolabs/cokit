// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
