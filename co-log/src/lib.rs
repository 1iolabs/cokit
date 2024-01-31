mod identity;
mod library;
mod types;

pub use identity::{
	did_key::{DidKeyIdentity, DidKeyIdentityResolver},
	join::JoinIdentityResolver,
	local::{LocalIdentity, LocalIdentityResolver},
};
pub use library::{
	clock::Clock,
	entry::{Entry, EntryBlock, EntryError, SignedEntry},
	log::Log,
	stream::LogIterator,
};
pub use types::{
	error::LogError,
	identity::{Identity, IdentityResolver, IdentityResolverError, PrivateIdentity, SignError},
};
