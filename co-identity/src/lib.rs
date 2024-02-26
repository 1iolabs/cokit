mod resolvers;
mod types;

pub use resolvers::{
	did_key::{DidKeyIdentity, DidKeyIdentityResolver},
	join::JoinIdentityResolver,
	local::{LocalIdentity, LocalIdentityResolver},
};
pub use types::{
	identity::{Identity, IdentityBox},
	private_identity::{PrivateIdentity, PrivateIdentityBox, SignError},
	resolver::{IdentityResolver, IdentityResolverBox, IdentityResolverError},
};
