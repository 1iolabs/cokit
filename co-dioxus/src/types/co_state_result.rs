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
