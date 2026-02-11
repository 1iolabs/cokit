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
