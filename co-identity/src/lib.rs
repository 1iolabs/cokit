// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod library;
mod resolvers;
mod types;

pub use library::network_did_discovery::network_did_discovery;
pub use resolvers::{
	did_key::{DidKeyIdentity, DidKeyIdentityResolver},
	join::{JoinIdentityResolver, JoinPrivateIdentityResolver},
	local::{LocalIdentity, LocalIdentityResolver},
	memory::{MemoryIdentityResolver, MemoryPrivateIdentityResolver},
};
pub use types::{
	did_core::{Jwk, VerificationMethod, VerificationMethodTypes},
	didcomm::{
		context::{DidCommContext, DidCommPrivateContext, DidCommPublicContext},
		header::{DidCommHeader, PeerDidCommHeader},
		message::Message,
	},
	identity::{Identity, IdentityBox},
	private_identity::{PrivateIdentity, PrivateIdentityBox, SignError},
	private_resolver::{PrivateIdentityResolver, PrivateIdentityResolverBox},
	receive_error::ReceiveError,
	resolver::{IdentityResolver, IdentityResolverBox, IdentityResolverError},
};
