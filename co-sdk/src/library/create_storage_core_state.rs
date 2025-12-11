use crate::{application::application::ApplicationSettings, types::co_pinning_key::CoPinningKey};
use cid::Cid;
use co_primitives::{AnyBlockStorage, CoId};

/// Create initial storage core state with CO pins.
pub async fn create_storage_core_state(
	storage: &impl AnyBlockStorage,
	settings: &ApplicationSettings,
	co: &CoId,
) -> Result<Option<Cid>, anyhow::Error> {
	Ok(co_core_storage::Storage::initial_state(
		storage,
		vec![
			co_core_storage::StorageAction::PinCreate(
				CoPinningKey::Root.to_string(co),
				settings.setting_co_default_max_state(),
				Default::default(),
			),
		],
	)
	.await?
	.into())
}
