use libipld::Cid;

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
