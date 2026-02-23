use co_primitives::{CoDate, Date};
use std::fmt::Debug;

#[derive(Debug, Default, Clone)]
pub struct JsCoDate;
impl CoDate for JsCoDate {
	fn now(&self) -> Date {
		js_sys::Date::now() as u64
	}
}
