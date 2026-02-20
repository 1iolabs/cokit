use co_primitives::{CoDate, Date};
use std::{
	fmt::Debug,
	time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "js")]
#[derive(Debug, Default, Clone)]
pub struct JsCoDate;
#[cfg(feature = "js")]
impl CoDate for JsCoDate {
	fn now(&self) -> Date {
		js_sys::Date::now() as u64
	}
}

#[cfg(feature = "native")]
#[derive(Debug, Default, Clone)]
pub struct SystemCoDate;
#[cfg(feature = "native")]
impl CoDate for SystemCoDate {
	fn now(&self) -> Date {
		SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis() as u64
	}
}
