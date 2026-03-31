// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use std::{borrow::Borrow, marker::PhantomData};

/// Typed core name.
pub struct CoreName<'a, S> {
	name: &'a str,
	_core: PhantomData<S>,
}
impl<'a, S> Clone for CoreName<'a, S> {
	fn clone(&self) -> Self {
		*self
	}
}
impl<'a, S> Copy for CoreName<'a, S> {}
impl<'a, S> std::fmt::Debug for CoreName<'a, S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CoreName")
			.field("name", &self.name)
			.field("_core", &self._core)
			.finish()
	}
}
impl<'a, S> std::fmt::Display for CoreName<'a, S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.name)
	}
}
impl<'a, S> CoreName<'a, S> {
	pub const fn new(name: &'a str) -> Self {
		Self { name, _core: PhantomData }
	}

	pub fn name(&self) -> &'a str {
		self.name
	}

	pub fn with_name<'n>(&self, name: &'n str) -> CoreName<'n, S> {
		CoreName::new(name)
	}

	pub fn with_name_opt(&self, name: Option<&'a str>) -> CoreName<'a, S> {
		name.map(|name| CoreName::new(name)).unwrap_or(*self)
	}
}
impl<'a, S> From<&'a str> for CoreName<'a, S> {
	fn from(value: &'a str) -> Self {
		Self::new(value)
	}
}
impl<'a, S> From<CoreName<'a, S>> for String {
	fn from(value: CoreName<'a, S>) -> Self {
		value.name.to_string()
	}
}
impl<'a, S> AsRef<str> for CoreName<'a, S> {
	fn as_ref(&self) -> &str {
		self.name
	}
}
impl<'a, S> Borrow<str> for CoreName<'a, S> {
	fn borrow(&self) -> &str {
		self.name
	}
}
impl<'a, S> PartialEq<&str> for CoreName<'a, S> {
	fn eq(&self, other: &&str) -> bool {
		self.name == *other
	}
}
impl<'a, S> PartialEq<String> for CoreName<'a, S> {
	fn eq(&self, other: &String) -> bool {
		self.name == other
	}
}
