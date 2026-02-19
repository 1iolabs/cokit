// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{use_co_error_provider, CoContext, CoSettings};
use dioxus::prelude::*;

/// Provide a new CoContext created from settings.
///
/// Note: Use [`CoContext::new`] and [`dioxus::LaunchBuilder::with_context`] instead.
pub fn use_co_context_provider(settings: impl FnOnce() -> CoSettings) {
	use_co_error_provider();
	use_context_provider(|| CoContext::new(settings()));
}
