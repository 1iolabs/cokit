// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{from_cbor, AnyBlockStorage, MultiCodec};
use cid::Cid;
use serde::Deserialize;

/// Read `core` property from a `ReducerAction`.
pub fn reducer_action_core(cbor: &[u8]) -> Result<String, anyhow::Error> {
	let core_reducer_action: CoreReducerAction = from_cbor(cbor)?;
	Ok(core_reducer_action.core)
}

/// Read `core` property from a `ReducerAction`.
pub async fn reducer_action_core_from_storage(
	storage: &impl AnyBlockStorage,
	reducer_action: Cid,
) -> Result<String, anyhow::Error> {
	let block = storage.get(MultiCodec::with_cbor(&reducer_action)?).await?;
	reducer_action_core(block.data())
}

/// Only extracts the core of an reducer action.
/// See: [`co_primitives::ReducerAction`]
#[derive(Debug, Deserialize)]
struct CoreReducerAction {
	#[serde(rename = "c")]
	core: String,
}

#[cfg(test)]
mod tests {
	use super::CoreReducerAction;
	use crate::{from_cbor, to_cbor, ReducerAction};
	use cid::Cid;

	#[test]
	fn test_core_reducer_action() {
		let reducer_action: ReducerAction<Option<Cid>> =
			ReducerAction { core: "test-core".into(), from: "did:test".into(), payload: None, time: 1 };
		let reducer_action_cbor = to_cbor(&reducer_action).unwrap();
		let core_reducer_action: CoreReducerAction = from_cbor(&reducer_action_cbor).unwrap();
		assert_eq!(core_reducer_action.core.as_str(), "test-core");
	}
}
