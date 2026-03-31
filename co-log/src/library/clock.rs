// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_primitives::Entry;
use std::cmp::max;

/// Finds the max clock time of the log.
/// The max clock time is equal to the tree height.
pub fn max_clock<'a>(heads: impl Iterator<Item = &'a Entry>) -> u64 {
	heads.map(|head| head.clock.time).reduce(max).unwrap_or(0)
}
