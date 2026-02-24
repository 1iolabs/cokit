// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::BlockStorage;
use futures::Stream;

pub trait Streamable<S>
where
	S: BlockStorage + Clone + 'static,
{
	type Item;
	type Stream: Stream<Item = Self::Item> + 'static;

	fn stream(&self, storage: S) -> Self::Stream;
}
