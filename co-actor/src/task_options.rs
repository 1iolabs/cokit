// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#[derive(Debug, Default)]
pub struct TaskOptions {
	pub name: Option<&'static str>,

	/// Untracked mode.
	///
	/// Use this for services which usually depend on callers and use own life-cycle.
	pub untracked: bool,
}
impl TaskOptions {
	pub fn new(name: &'static str) -> Self {
		Self { name: Some(name), untracked: false }
	}

	pub fn untracked() -> Self {
		Self { name: None, untracked: true }
	}

	pub fn with_untracked(mut self) -> Self {
		self.untracked = true;
		self
	}
}
