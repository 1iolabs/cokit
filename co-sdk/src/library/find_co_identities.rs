// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{find_membership, library::network_identity::network_identity_by_id, CoContext, CO_ID_LOCAL};
use co_identity::PrivateIdentityBox;
use co_primitives::{CoId, Did};

pub async fn find_co_identities(context: &CoContext, co: &CoId) -> anyhow::Result<Vec<Did>> {
	let local_co = context.local_co_reducer().await?;
	let membership = find_membership(&local_co, co).await?;
	Ok(membership.into_iter().flat_map(|m| m.did.into_keys()).collect())
}

pub async fn find_co_private_identity(context: &CoContext, co: &CoId) -> anyhow::Result<PrivateIdentityBox> {
	network_identity_by_id(context, &CoId::from(CO_ID_LOCAL), co, None).await
}
