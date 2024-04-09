use crate::CoContext;
use dioxus::prelude::*;

pub fn use_co_context_provider() {
	use_context_provider(|| CoContext::new());
}
