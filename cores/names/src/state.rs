// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::{guard::has_guard_access, reduce::reduce},
	transaction::NamesTransaction,
	Config, Index, IndexKey, NamesAction, Record, RecordId,
};
use co_api::{co, BlockStorageExt, CoMap, CoreBlockStorage, Guard, Link, OptionLink, Reducer, ReducerAction};

#[co(state, guard)]
pub struct Names {
	#[serde(rename = "c")]
	pub config: OptionLink<Config>,
	#[serde(rename = "r")]
	pub records: CoMap<RecordId, Link<Record>>,
	#[serde(rename = "i")]
	pub indexes: CoMap<Link<IndexKey>, Index>,
}
impl Reducer<NamesAction> for Names {
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<NamesAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let mut state = storage.get_value_or_default(&state_link).await?;
		let action = storage.get_value(&event_link).await?;
		let mut transaction = NamesTransaction::open(storage.clone(), &state).await?;
		reduce(&mut transaction, action.from, action.payload).await?;
		match (state_link.link(), transaction.store(&mut state).await?) {
			(None, _) | (_, true) => Ok(storage.set_value(&state).await?),
			(Some(previous_state_link), false) => Ok(previous_state_link),
		}
	}
}
impl Guard for Names {
	async fn verify(
		storage: &CoreBlockStorage,
		guard: String,
		state: cid::Cid,
		heads: std::collections::BTreeSet<cid::Cid>,
		next_head: cid::Cid,
	) -> Result<bool, anyhow::Error> {
		has_guard_access(storage, guard, state, heads, next_head).await
	}
}
