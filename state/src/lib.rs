// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

#![feature(impl_trait_in_assoc_type)]
// #![feature(associated_type_defaults)]
// #![feature(return_position_impl_trait_in_trait)]

mod library;
mod types;

// pub use types::sync_reducer::*;
pub use library::{
	combine_epics::CombineEpics,
	combine_reducers::CombineReducers,
	end_with::*,
	fn_reducer::FnReducer,
	log_middleware::LogMiddleware,
	store::{MiddlewareStore, Store},
	subject_middleware::SubjectMiddleware,
	sync_store::SyncStore,
};
pub use types::{epic::*, middleware::Middleware, reducer::*, store_api::StoreApi, sync_store_api::SyncStoreApi};
