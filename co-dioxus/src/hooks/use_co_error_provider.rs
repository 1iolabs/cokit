use crate::CoErrorSignal;
use dioxus::prelude::*;

pub fn use_co_error_provider() -> CoErrorSignal {
	use_context_provider(|| CoErrorSignal::new_maybe_sync(Vec::new()))
}
