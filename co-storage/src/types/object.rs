// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use ipld_core::ipld::Ipld;

pub trait ObjectAPI {
	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Result<Ipld, StorageError>;

	/// Inserts a block into storage.
	fn set(&mut self, object: Ipld) -> Result<Cid, StorageError>;
}
