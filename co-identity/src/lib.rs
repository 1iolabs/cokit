mod resolvers;
mod types;

pub use resolvers::{
	did_key::{DidKeyIdentity, DidKeyIdentityResolver},
	join::{JoinIdentityResolver, JoinPrivateIdentityResolver},
	local::{LocalIdentity, LocalIdentityResolver},
};
pub use types::{
	identity::{Identity, IdentityBox},
	private_identity::{PrivateIdentity, PrivateIdentityBox, SignError},
	private_resolver::{PrivateIdentityResolver, PrivateIdentityResolverBox},
	resolver::{IdentityResolver, IdentityResolverBox, IdentityResolverError},
};
