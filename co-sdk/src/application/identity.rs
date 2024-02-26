use co_identity::{DidKeyIdentityResolver, IdentityResolverBox, JoinIdentityResolver, LocalIdentityResolver};

/// Create the default identity resolver.
pub fn create_identity_resolver() -> IdentityResolverBox {
	let mut resolvers: Vec<IdentityResolverBox> = Vec::new();
	resolvers.push(Box::new(LocalIdentityResolver::new()));
	resolvers.push(Box::new(DidKeyIdentityResolver::new()));
	Box::new(JoinIdentityResolver::new(resolvers))
}

// pub async fn get_private_identity_from_key_store(
// 	co: CoReducer,
// 	identity: &str,
// ) -> Result<Option<PrivateIdentityBox>, anyhow::Error> {
// 	todo!()
// }
