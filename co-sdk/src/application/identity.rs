use crate::{Application, DidKeyProvider, CO_CORE_NAME_KEYSTORE};
use co_identity::{
	DidKeyIdentityResolver, IdentityResolver, IdentityResolverBox, JoinIdentityResolver, JoinPrivateIdentityResolver,
	LocalIdentityResolver, PrivateIdentityBox, PrivateIdentityResolver, PrivateIdentityResolverBox,
};

/// Create the default identity resolver.
pub fn create_identity_resolver() -> IdentityResolverBox {
	let mut resolvers: Vec<IdentityResolverBox> = Vec::new();
	resolvers.push(IdentityResolver::boxed(LocalIdentityResolver::new()));
	resolvers.push(DidKeyIdentityResolver::new().boxed());
	JoinIdentityResolver::new(resolvers).boxed()
}

/// Create the default private identity resolver.
pub async fn create_private_identity_resolver(
	application: &Application,
) -> Result<PrivateIdentityResolverBox, anyhow::Error> {
	let local = application.local_co_reducer().await?;
	let mut resolvers: Vec<PrivateIdentityResolverBox> = Vec::new();
	resolvers.push(PrivateIdentityResolver::boxed(LocalIdentityResolver::default()));
	resolvers.push(DidKeyProvider::new(local, CO_CORE_NAME_KEYSTORE).boxed());
	Ok(JoinPrivateIdentityResolver::new(resolvers).boxed())
}

/// Resolve a private identity.
///
/// Todo: Identity Permissions?
pub async fn resolve_private_identity(
	application: &Application,
	did: &co_primitives::Did,
) -> Result<PrivateIdentityBox, anyhow::Error> {
	let resolver = create_private_identity_resolver(application).await?;
	Ok(resolver.resolve_private(&did).await?)
}
