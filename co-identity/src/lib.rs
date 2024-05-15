mod library;
mod resolvers;
mod types;

pub use resolvers::{
	did_key::{DidKeyIdentity, DidKeyIdentityResolver},
	join::{JoinIdentityResolver, JoinPrivateIdentityResolver},
	local::{LocalIdentity, LocalIdentityResolver},
};
pub use types::{
	did_core::{Jwk, VerificationMethod, VerificationMethodTypes},
	didcomm::{
		context::{DidCommContext, DidCommPrivateContext, DidCommPublicContext},
		header::DidCommHeader,
	},
	identity::{Identity, IdentityBox},
	private_identity::{PrivateIdentity, PrivateIdentityBox, SignError},
	private_resolver::{PrivateIdentityResolver, PrivateIdentityResolverBox},
	receive_error::ReceiveError,
	resolver::{IdentityResolver, IdentityResolverBox, IdentityResolverError},
};
