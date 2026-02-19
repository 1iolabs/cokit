// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

mod action;
pub mod config;
mod index;
mod library;
mod record;
mod state;
mod transaction;

pub use action::{
	IndexInsertAction, IndexRemoveAction, NamesAction, RecordInsertAction, RecordRemoveAction, RecordUpdateAction,
};
pub use config::{Config, RecordTypeConfig, RecordTypeLimit};
pub use index::{Index, IndexConfig, IndexKey};
pub use record::{
	name::{
		Endpoint, EndpointId, EndpointInsertAction, EndpointRemoveAction, EndpointScheme, NameRecord, NameRecordAction,
	},
	DelegateRecord, DynamicRecord, Record, RecordId, RecordType, CO_RECORD_TYPE, DELEGATE_RECORD_TYPE,
	NAME_RECORD_TYPE,
};
pub use state::Names;
