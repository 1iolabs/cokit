use crate::StorageError;
use libipld::Cid;

/// Storage Pin API.
pub trait PinApi {
	/// Create new pin.
	fn add(&mut self, cid: &Cid, options: PinOptions) -> Result<(), StorageError>;

	/// Remove pin.
	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError>;

	/// Returns whether the CID is pinned.
	fn is_pinned(&self, cid: &Cid) -> Option<PinKind>;

	/// List all pins.
	fn iter(&self) -> Box<dyn Iterator<Item = (Cid, PinKind)>>;
}

#[derive(Debug, Default, Clone)]
pub struct PinOptions {
	pub kind: PinKind,
}

#[derive(Debug, Default, Clone)]
pub enum PinKind {
	#[default]
	Recursive,
	Direct,
	Indirect,
}
