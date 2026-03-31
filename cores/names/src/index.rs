// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::RecordId;
use co_api::{co, CoMap, CoSet, Link, TagValue};

#[co]
pub struct IndexKey {
	pub record_type: String,
	pub name: String,
}

#[co]
pub struct IndexConfig {
	pub field: String,
	pub unique: bool,
}

#[co]
pub struct Index {
	/// The index config.
	/// Because of the (possibly) frequent changes of `index` we store this as link.
	pub config: Link<IndexConfig>,

	/// The index.
	pub index: CoMap<TagValue, CoSet<RecordId>>,
}
