// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;

/// State result.
///
/// Todo: make this Copy to be able to use `*signal.read()`.
/// Todo: Error handling.
#[derive(Debug, Clone, PartialEq)]
pub enum CoStateResult<T> {
	Pending,
	State(Option<Cid>, T),
	Error(String),
}
impl<T> CoStateResult<T> {
	pub fn state_or_default(&self) -> T
	where
		T: Default + Clone,
	{
		match self {
			CoStateResult::Pending => T::default(),
			CoStateResult::State(_, state) => state.clone(),
			CoStateResult::Error(_) => T::default(),
		}
	}
}
