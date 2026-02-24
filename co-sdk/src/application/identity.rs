// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{CoContext, CoReducer, DidKeyProvider, CO_CORE_NAME_KEYSTORE};
use co_identity::{
	DidKeyIdentityResolver, IdentityResolver, IdentityResolverBox, JoinIdentityResolver, JoinPrivateIdentityResolver,
	LocalIdentityResolver, PrivateIdentityBox, PrivateIdentityResolver, PrivateIdentityResolverBox,
};

/// Create the default identity resolver.
pub fn create_identity_resolver() -> IdentityResolverBox {
	JoinIdentityResolver::new(vec![
		IdentityResolver::boxed(LocalIdentityResolver::new()),
		DidKeyIdentityResolver::new().boxed(),
	])
	.boxed()
}

/// Create the default private identity resolver.
pub async fn create_private_identity_resolver(local: CoReducer) -> Result<PrivateIdentityResolverBox, anyhow::Error> {
	Ok(JoinPrivateIdentityResolver::new(vec![
		PrivateIdentityResolver::boxed(LocalIdentityResolver::default()),
		DidKeyProvider::new(local, CO_CORE_NAME_KEYSTORE).boxed(),
	])
	.boxed())
}

/// Resolve a private identity.
///
/// Todo: Identity Permissions?
pub async fn resolve_private_identity(
	context: &CoContext,
	did: &co_primitives::Did,
) -> Result<PrivateIdentityBox, anyhow::Error> {
	let resolver = create_private_identity_resolver(context.local_co_reducer().await?).await?;
	Ok(resolver.resolve_private(did).await?)
}
