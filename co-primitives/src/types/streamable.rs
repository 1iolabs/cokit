// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
