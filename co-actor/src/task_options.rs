// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
