// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use super::{query_core, Query, QueryError};
use crate::{state, CoStorage, CO_CORE_NAME_CO};
use co_core_co::Co;
use co_primitives::{Network, OptionLink};
use futures::TryStreamExt;

/// Read network settings from an CO.
pub async fn networks(storage: &CoStorage, co_state: OptionLink<Co>) -> Result<Vec<Network>, QueryError> {
	let co = query_core(CO_CORE_NAME_CO).with_default().execute(storage, co_state).await?;
	Ok(state::stream(storage.clone(), &co.network).try_collect().await?)
}
