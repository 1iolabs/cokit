use co_primitives::Date;
use std::{
	fmt::{Debug, Formatter},
	sync::{Arc, Mutex},
	time::{SystemTime, UNIX_EPOCH},
};

pub trait CoDate: Send + Sync + 'static {
	fn now(&self) -> Date;

	fn boxed(self) -> DynamicCoDate
	where
		Self: Sized,
	{
		DynamicCoDate::new(self)
	}
}

#[derive(Debug, Default, Clone)]
pub struct SystemCoDate;
impl CoDate for SystemCoDate {
	fn now(&self) -> Date {
		SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis()
	}
}

#[derive(Debug, Clone)]
pub struct StaticCoDate(pub Date);
impl CoDate for StaticCoDate {
	fn now(&self) -> Date {
		self.0
	}
}

#[derive(Debug, Default, Clone)]
pub struct MonotonicCoDate(Arc<Mutex<Date>>);
impl CoDate for MonotonicCoDate {
	fn now(&self) -> Date {
		let mut time = self.0.lock().unwrap();
		let result = *time;
		*time += 1;
		result
	}
}

#[derive(Clone)]
pub struct DynamicCoDate(Arc<dyn CoDate>);
impl Debug for DynamicCoDate {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DynamicCoDate").field(&self.0.now()).finish()
	}
}
impl DynamicCoDate {
	pub fn new(date: impl CoDate) -> Self {
		Self(Arc::new(date))
	}
}
impl CoDate for DynamicCoDate {
	fn now(&self) -> Date {
		self.0.now()
	}

	fn boxed(self) -> DynamicCoDate
	where
		Self: Sized,
	{
		self
	}
}
