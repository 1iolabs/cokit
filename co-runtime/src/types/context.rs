use cid::Cid;

#[derive(Debug, Clone)]
pub struct RuntimeContext {
	/// The acutual state.
	pub state: Option<Cid>,

	/// The event to apply to the state.
	pub event: Cid,

	/// Diagnostics returned from the COre.
	pub diagnostics: Vec<Cid>,
}
impl RuntimeContext {
	pub fn new(state: Option<Cid>, event: Cid) -> Self {
		Self { state, event, diagnostics: Default::default() }
	}
}
