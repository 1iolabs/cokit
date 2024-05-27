use crate::{EntryBlock, Log, LogError};
use co_identity::{Identity, IdentityResolver};
use co_storage::BlockStorage;

pub async fn verify_entry<S>(log: &Log<S>, entry: &EntryBlock<S::StoreParams>) -> Result<(), LogError>
where
	S: BlockStorage,
{
	// verify log
	if &entry.entry().id != log.id() {
		return Err(LogError::InvalidArgument(anyhow::anyhow!(
			"Invalid log: {:02X?} != {:02X?}",
			&entry.entry().id,
			log.id()
		)))
	}

	// verify signature
	let identity = log
		.identity_resolver()
		.resolve(&entry.signed_entry().identity, entry.signed_entry().public_key.as_ref().map(|v| v.as_slice()))
		.await?;
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
