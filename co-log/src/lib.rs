mod library;
mod types;

pub use library::{
	clock::Clock,
	did_key::{DidKeyIdentity, DidKeyIdentityResolver},
	entry::{Entry, EntryBlock, EntryError, SignedEntry},
	identity::{Identity, IdentityResolver, IdentityResolverError, JoinIdentityResolver, SignError},
	log::Log,
	stream::LogIterator,
};
pub use types::error::LogError;
