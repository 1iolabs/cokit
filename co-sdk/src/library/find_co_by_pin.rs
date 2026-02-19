// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{CoContext, CoPinningKey, CoReducer, CoReducerFactory};

pub async fn find_co_by_pin(context: &CoContext, pin: String) -> Result<CoReducer, anyhow::Error> {
	let (_pinning_key, co_id) = CoPinningKey::parse(pin)?;
	let co = context.try_co_reducer(&co_id).await?;
	Ok(co)
}
