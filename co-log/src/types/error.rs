// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::EntryError;
use co_identity::IdentityResolverError;
use co_primitives::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum LogError {
	#[error("Storage error")]
	Storage(#[from] StorageError),

	#[error("Entry error")]
	Entry(#[from] EntryError),

	#[error("Identity resolver error")]
	IdentityResolver(#[from] IdentityResolverError),

	#[error("Invalid argument")]
	InvalidArgument(#[from] anyhow::Error),

	#[error("Internal error")]
	Internal(#[source] anyhow::Error),
}
