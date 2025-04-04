use std::{
	fmt::{Debug, Formatter},
	sync::{Arc, Mutex},
};
use uuid::Uuid;

pub trait CoUuid: Send + Sync + 'static {
	fn uuid(&self) -> String;

	fn boxed(self) -> DynamicCoUuid
	where
		Self: Sized,
	{
		DynamicCoUuid::new(self)
	}
}

#[derive(Debug, Default, Clone)]
pub struct MonotonicCoUuid(Arc<Mutex<i32>>);
impl CoUuid for MonotonicCoUuid {
	fn uuid(&self) -> String {
		let mut time = self.0.lock().unwrap();
		let result = *time;
		*time += 1;
		result.to_string()
	}
}

#[derive(Debug, Default, Clone)]
pub struct RandomCoUuid;
impl CoUuid for RandomCoUuid {
	fn uuid(&self) -> String {
		Uuid::new_v4().to_string()
	}
}

#[derive(Clone)]
pub struct DynamicCoUuid(Arc<dyn CoUuid>);
impl Debug for DynamicCoUuid {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DynamicCoUuid").field(&self.0.uuid()).finish()
	}
}
impl DynamicCoUuid {
	pub fn new(date: impl CoUuid) -> Self {
		Self(Arc::new(date))
	}
}
impl CoUuid for DynamicCoUuid {
	fn uuid(&self) -> String {
		self.0.uuid()
	}

	fn boxed(self) -> DynamicCoUuid
	where
		Self: Sized,
	{
		self
	}
}
