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
