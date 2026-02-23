use co_primitives::{CoDate, Date};
use std::{
	fmt::Debug,
	time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Default, Clone)]
pub struct SystemCoDate;
impl CoDate for SystemCoDate {
	fn now(&self) -> Date {
		SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis() as u64
	}
}
