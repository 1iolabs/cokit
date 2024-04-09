use crate::{CoContext, CoSettings};
use dioxus::prelude::*;

pub fn use_co_context_provider(settings: impl FnOnce() -> CoSettings) {
	use_context_provider(|| CoContext::new(settings()));
}
