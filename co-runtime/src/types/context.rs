use libipld::Cid;

#[derive(Debug, Clone)]
pub struct RuntimeContext {
	/// The acutual state.
	pub state: Option<Cid>,

	/// The event to apply to the state.
	pub event: Cid,
}
