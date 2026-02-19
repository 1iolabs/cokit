// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::StorageError;
use async_trait::async_trait;
use cid::Cid;

/// Storage Pin API.
#[async_trait(?Send)]
pub trait PinApi {
	/// Create new pin.
	async fn add(&mut self, cid: &Cid, options: PinOptions) -> Result<(), StorageError>;

	/// Remove pin.
	async fn remove(&mut self, cid: &Cid) -> Result<(), StorageError>;

	/// Returns whether the CID is pinned.
	async fn is_pinned(&self, cid: &Cid) -> Option<PinKind>;

	/// List all pins.
	async fn iter(&self) -> Box<dyn Iterator<Item = (Cid, PinKind)>>;
}

#[derive(Debug, Default, Clone)]
pub struct PinOptions {
	pub kind: PinKind,
}

#[derive(Debug, Default, Clone)]
pub enum PinKind {
	/// Recursively PIN.
	/// Supported types:
	/// - dag-cbor
	/// - dag-json
	/// - dag-pb
	#[default]
	Recursive,

	/// Directly pinned.
	Direct,

	/// Indirectly pinned (through an recursive direct pin).
	Indirect,
}
