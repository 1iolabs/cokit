// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_primitives::Entry;
use std::cmp::max;

/// Finds the max clock time of the log.
/// The max clock time is equal to the tree height.
pub fn max_clock<'a>(heads: impl Iterator<Item = &'a Entry>) -> u64 {
	heads.map(|head| head.clock.time).reduce(max).unwrap_or(0)
}
