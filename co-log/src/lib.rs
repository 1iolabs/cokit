mod library;
mod types;

pub use library::{
	clock::Clock,
	entry::{Entry, EntryBlock, EntryError, SignedEntry},
	log::Log,
	stream::{create_stream, LogIterator},
};
pub use types::error::LogError;
