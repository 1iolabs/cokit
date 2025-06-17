use crate::{use_co_error_provider, CoContext, CoSettings};
use dioxus::prelude::*;

/// Provide a new CoContext created from settings.
///
/// Note: Use [`CoContext::new`] and [`dioxus::LaunchBuilder::with_context`] instead.
pub fn use_co_context_provider(settings: impl FnOnce() -> CoSettings) {
	use_co_error_provider();
	use_context_provider(|| CoContext::new(settings()));
}
