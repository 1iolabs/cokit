// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	record::KnownRecord, transaction::NamesTransaction, DynamicRecord, Record, RecordId, RecordInsertAction,
	RecordType, RecordTypeLimit, DELEGATE_RECORD_TYPE,
};
use anyhow::anyhow;
use co_api::{BlockStorageExt, Did, TagPattern, Tags, TagsExpr};
use futures::{pin_mut, TryStreamExt};
use std::borrow::Borrow;

/// Test if `from` is allowed to insert a record based on configuration.
pub async fn has_record_insert_access(
	state: &mut NamesTransaction,
	from: &Did,
	action: &RecordInsertAction,
) -> Result<bool, anyhow::Error> {
	// permissions
	let record: DynamicRecord = state.storage().get_deserialized(action.record.cid()).await?;
	if let Some(record_config) = state.config().types.get(record.record_type()).cloned() {
		if let Some(creator) = &record_config.creator {
			// creator
			if creator != from {
				return Ok(false);
			}
		}

		// limits
		if !match &record_config.limit {
			RecordTypeLimit::None => true,
			RecordTypeLimit::ByIdentity(max) => {
				let count = state.index_lookup_count(record.record_type(), "owner", from.clone()).await?;
				(*max as i32) - count > 0
			},
			RecordTypeLimit::ByRecord(max, by_record_type) => {
				let count = state.index_lookup_count(record.record_type(), "owner", from.clone()).await?;
				let by_record_count = state.index_lookup_count(by_record_type, "owner", from.clone()).await?;
				by_record_count * (*max as i32) - count > 0
			},
		} {
			return Ok(false);
		}

		// ok
		Ok(true)
	} else {
		Ok(false)
	}
}

pub async fn has_access(
	state: &mut NamesTransaction,
	did: &Did,
	record_id: &RecordId,
	scope: impl Borrow<Tags>,
) -> Result<bool, anyhow::Error> {
	let access = get_access(state, did, record_id, true).await?;
	Ok(access.test(Some(scope.borrow())))
}

pub async fn check_access(
	state: &mut NamesTransaction,
	did: &Did,
	record_id: &RecordId,
	scope: impl Borrow<Tags>,
) -> Result<(), anyhow::Error> {
	if has_access(state, did, record_id, scope).await? {
		Ok(())
	} else {
		Err(anyhow!("Permission denied"))
	}
}

pub async fn has_access_full(
	state: &mut NamesTransaction,
	did: &Did,
	record_id: &RecordId,
) -> Result<bool, anyhow::Error> {
	Ok(get_access(state, did, record_id, true).await?.is_full())
}

pub async fn check_access_full(
	state: &mut NamesTransaction,
	did: &Did,
	record_id: &RecordId,
) -> Result<(), anyhow::Error> {
	if has_access_full(state, did, record_id).await? {
		Ok(())
	} else {
		Err(anyhow!("Permission denied"))
	}
}

pub async fn has_access_owner(
	state: &mut NamesTransaction,
	did: &Did,
	record_id: &RecordId,
) -> Result<bool, anyhow::Error> {
	Ok(matches!(get_access(state, did, record_id, false).await?, Access::Owner))
}

pub async fn check_access_owner(
	state: &mut NamesTransaction,
	did: &Did,
	record_id: &RecordId,
) -> Result<(), anyhow::Error> {
	if has_access_owner(state, did, record_id).await? {
		Ok(())
	} else {
		Err(anyhow!("Permission denied"))
	}
}

pub async fn get_access(
	state: &mut NamesTransaction,
	did: &Did,
	record_id: &RecordId,
	delegate: bool,
) -> Result<Access, anyhow::Error> {
	let record = state.record(record_id).await?;

	// access
	let access = check_direct_access(did, &record);
	if access.is_some() {
		return Ok(access);
	}

	// is delegated?
	let mut access = Access::None;
	if delegate {
		let delegate_records = state.index_lookup_records(DELEGATE_RECORD_TYPE, "to", did.to_owned()).await?;
		pin_mut!(delegate_records);
		while let Some(delegate_record) = delegate_records.try_next().await? {
			if let Record::Known(KnownRecord::Delegate(delegate_record)) = delegate_record {
				if &delegate_record.to == did && check_direct_access(&delegate_record.owner, &record).is_some() {
					access = access.merge(Access::Delegate(delegate_record.scope));
					if access.is_full_delegate() {
						break;
					}
				}
			}
		}
	}
	Ok(access)
}

pub enum Access {
	None,
	Owner,
	Controller,

	/// Delegated access.
	Delegate(Option<TagsExpr>),
}
impl Access {
	pub fn is_some(&self) -> bool {
		matches!(self, Access::Owner | Access::Controller | Access::Delegate(_))
	}

	pub fn is_full(&self) -> bool {
		matches!(self, Access::Owner | Access::Controller | Access::Delegate(None))
	}

	pub fn is_full_delegate(&self) -> bool {
		matches!(self, Access::Delegate(None))
	}

	pub fn merge(self, other: Access) -> Access {
		match (self, other) {
			(Access::Owner, _) | (_, Access::Owner) => Access::Owner,
			(Access::Controller, _) | (_, Access::Controller) => Access::Controller,
			(Access::Delegate(None), _) | (_, Access::Delegate(None)) => Access::Delegate(None),
			(Access::Delegate(Some(a)), Access::Delegate(Some(b))) => Access::Delegate(Some(a.or(b))),
			(Access::Delegate(Some(tags)), _) | (_, Access::Delegate(Some(tags))) => Access::Delegate(Some(tags)),
			(Access::None, Access::None) => Access::None,
		}
	}

	pub fn test(&self, scope: Option<&Tags>) -> bool {
		match (self, scope) {
			(Access::None, _) => false,
			(Access::Owner, _) => true,
			(Access::Controller, _) => true,
			(Access::Delegate(None), _) => true,
			(Access::Delegate(Some(_)), None) => false,
			(Access::Delegate(Some(expr)), Some(scope)) => expr.matches_pattern(scope),
		}
	}
}

/// Check if `did` has direct access to `record`.
fn check_direct_access(did: &Did, record: &Record) -> Access {
	// is owner?
	if record.owner() == Some(did) {
		return Access::Owner;
	}

	// is controller?
	if record.controller().contains(did) {
		return Access::Controller;
	}

	// none
	Access::None
}
