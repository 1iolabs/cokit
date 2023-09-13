// #![feature(type_alias_impl_trait)] // used in older nightly
#![feature(impl_trait_in_assoc_type)]
//#![feature(associated_type_defaults)]
//#![feature(return_position_impl_trait_in_trait)]

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
