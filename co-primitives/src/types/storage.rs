// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::Block;
use cid::Cid;

/// Storage interface.
pub trait Storage {
	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Block;

	/// Inserts a block into storage.
	fn set(&mut self, block: Block) -> Cid;
}
