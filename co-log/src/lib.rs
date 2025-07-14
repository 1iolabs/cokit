mod library;
mod types;

pub use library::{
	entry::{EntryBlock, EntryError},
	log::Log,
	stream::{create_stream, LogIterator},
	verify_entry::{EntryVerifier, IdentityEntryVerifier, NoEntryVerifier, ReadOnlyEntryVerifier},
};
pub use types::error::LogError;
