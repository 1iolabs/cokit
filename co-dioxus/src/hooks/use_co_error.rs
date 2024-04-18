use crate::CoErrorSignal;
use dioxus::hooks::use_context;

pub fn use_co_error() -> CoErrorSignal {
	use_context::<CoErrorSignal>()
}
