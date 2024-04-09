use crate::CoContext;
use dioxus::prelude::*;

pub fn use_co_context() -> CoContext {
	use_context()
}
