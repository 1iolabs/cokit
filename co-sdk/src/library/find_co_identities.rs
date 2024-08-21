use crate::{find_membership, find_memberships, CoContext};
use anyhow::anyhow;
use co_identity::{PrivateIdentityBox, PrivateIdentityResolver};
use co_primitives::{CoId, Did};

pub async fn find_co_identities(context: &CoContext, co: &CoId) -> anyhow::Result<Vec<Did>> {
	let local_co = context.local_co_reducer().await?;
	let memberships = find_memberships(&local_co, co).await?;
	Ok(memberships.into_iter().map(|membership| membership.did).collect())
}

pub async fn find_co_private_identity(context: &CoContext, co: &CoId) -> anyhow::Result<PrivateIdentityBox> {
	let local_co = context.local_co_reducer().await?;
	let membership = find_membership(&local_co, co)
		.await?
		.ok_or(anyhow!("Membership not found: {}", co))?;
	let identity = context
		.private_identity_resolver()
		.await?
		.resolve_private(&membership.did)
		.await?;
	Ok(identity)
}
