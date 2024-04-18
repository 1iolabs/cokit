use crate::{use_co_error_provider, CoContext, CoSettings};
use dioxus::prelude::*;

pub fn use_co_context_provider(settings: impl FnOnce() -> CoSettings) {
	use_co_error_provider();
	use_context_provider(|| CoContext::new(settings()));
}
