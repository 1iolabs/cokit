// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use ipld_core::ipld::Ipld;

pub trait ObjectAPI {
	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Result<Ipld, StorageError>;

	/// Inserts a block into storage.
	fn set(&mut self, object: Ipld) -> Result<Cid, StorageError>;
}
