// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	library::permissions::{has_access, has_access_full, has_access_owner, has_record_insert_access},
	transaction::NamesTransaction,
	Names, NamesAction,
};
use anyhow::anyhow;
use cid::Cid;
use co_api::{reducer_action_core_from_storage, BlockStorageExt, CoreBlockStorage, ReducerAction, SignedEntry};
use co_core_co::Co;
use std::collections::BTreeSet;

pub async fn has_guard_access(
	storage: &CoreBlockStorage,
	guard: String,
	state: Cid,
	_heads: BTreeSet<Cid>,
	next_head: Cid,
) -> Result<bool, anyhow::Error> {
	let next_entry: SignedEntry = storage.get_deserialized(&next_head).await?;
	let co: Co = storage.get_deserialized(&state).await?;
	let guard = co.guards.get(&guard).ok_or(anyhow!("Guard not found: {}", guard))?;
	let names_core_name = guard.tags.string("core").unwrap_or("names");
	let action_core_name = reducer_action_core_from_storage(storage, next_entry.entry.payload).await?;
	if names_core_name == action_core_name {
		let names_state_link = co.cores.get(names_core_name).and_then(|core| core.state);
		if let Some(names_state_link) = names_state_link {
			let reducer_action: ReducerAction<NamesAction> =
				storage.get_deserialized(&next_entry.entry.payload).await?;
			let names: Names = storage.get_deserialized(&names_state_link).await?;
			let mut transaction = NamesTransaction::open(storage.clone(), &names).await?;
			return Ok(match &reducer_action.payload {
				NamesAction::RecordInsert(action) => {
					has_record_insert_access(&mut transaction, &reducer_action.from, action).await?
				},
				NamesAction::RecordUpdate(action) => {
					has_access_full(&mut transaction, &reducer_action.from, &action.id).await?
				},
				NamesAction::RecordRemove(action) => {
					has_access_owner(&mut transaction, &reducer_action.from, &action.id).await?
				},
				NamesAction::IndexInsert(_) => co.participants.contains(storage, &reducer_action.from).await?,
				NamesAction::IndexRemove(_) => co.participants.contains(storage, &reducer_action.from).await?,
				NamesAction::Name(action) => {
					has_access(&mut transaction, &reducer_action.from, &action.record(), action.scope()).await?
				},
			});
		}
	}
	Ok(true)
}
