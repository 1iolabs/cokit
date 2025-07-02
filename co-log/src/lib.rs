mod library;
mod types;

pub use library::{
	entry::{EntryBlock, EntryError},
	log::Log,
	stream::{create_stream, LogIterator},
};
pub use types::error::LogError;
