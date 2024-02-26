mod library;
mod types;

pub use library::{
	clock::Clock,
	entry::{Entry, EntryBlock, EntryError, SignedEntry},
	log::Log,
	stream::LogIterator,
};
pub use types::error::LogError;
