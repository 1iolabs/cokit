// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{EntryBlock, Log, LogError};
use anyhow::anyhow;
use async_trait::async_trait;
use co_identity::{Identity, IdentityResolver, IdentityResolverBox};
use std::fmt::Debug;

#[async_trait]
pub trait EntryVerifier: Debug + Send + Sync + 'static {
	async fn verify_entry(&self, log: &Log, entry: &EntryBlock) -> Result<(), LogError>;
}

pub async fn verify_entry(log: &Log, entry: &EntryBlock) -> Result<(), LogError> {
	// verify log
	if entry.entry().id != log.id() {
		return Err(LogError::InvalidArgument(anyhow::anyhow!(
			"Invalid log: {:02X?} != {:02X?}",
			&entry.entry().id,
			log.id()
		)));
	}

	// verifier
	log.entry_verifier().verify_entry(log, entry).await?;

	// ok
	Ok(())
}

#[derive(Debug, Clone)]
pub struct IdentityEntryVerifier {
	identity_resolver: IdentityResolverBox,
}
impl IdentityEntryVerifier {
	pub fn new(identity_resolver: IdentityResolverBox) -> Self {
		Self { identity_resolver }
	}
}
impl From<IdentityResolverBox> for IdentityEntryVerifier {
	fn from(value: IdentityResolverBox) -> Self {
		Self { identity_resolver: value }
	}
}
#[async_trait]
impl EntryVerifier for IdentityEntryVerifier {
	async fn verify_entry(&self, _log: &Log, entry: &EntryBlock) -> Result<(), LogError> {
		// verify signature
		let identity = self.identity_resolver.resolve(&entry.signed_entry().identity).await?;
		if !entry.verify(&identity)? {
			// log
			tracing::info!(
				entry_identity = entry.signed_entry().identity,
				resolved_identity = identity.identity(),
				entry_signature = ?entry.signed_entry().signature.iter().map(|c| format!("{:02X}", c)).collect::<String>(),
				"verify-failed"
			);

			// error
			return Err(LogError::InvalidArgument(anyhow::anyhow!("Invalid entry signature")));
		}

		// ok
		Ok(())
	}
}

#[derive(Debug, Default, Clone)]
pub struct NoEntryVerifier {}
#[async_trait]
impl EntryVerifier for NoEntryVerifier {
	async fn verify_entry(&self, _log: &Log, _entry: &EntryBlock) -> Result<(), LogError> {
		Ok(())
	}
}

#[derive(Debug, Default, Clone)]
pub struct ReadOnlyEntryVerifier {}
#[async_trait]
impl EntryVerifier for ReadOnlyEntryVerifier {
	async fn verify_entry(&self, _log: &Log, _entry: &EntryBlock) -> Result<(), LogError> {
		Err(LogError::InvalidArgument(anyhow!("This log is read only.")))
	}
}
