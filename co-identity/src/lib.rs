// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
