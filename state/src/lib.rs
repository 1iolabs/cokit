#![feature(type_alias_impl_trait)]
// #![feature(associated_type_defaults)]
// #![feature(return_position_impl_trait_in_trait)]

mod library;
mod types;

// pub use types::sync_reducer::*;
pub use library::combine_epics::CombineEpics;
pub use library::combine_reducers::CombineReducers;
pub use library::end_with::*;
pub use library::fn_reducer::FnReducer;
pub use library::log_middleware::LogMiddleware;
pub use library::store::{MiddlewareStore, Store};
pub use library::subject_middleware::SubjectMiddleware;
pub use library::sync_store::SyncStore;
pub use types::epic::*;
pub use types::middleware::Middleware;
pub use types::reducer::*;
pub use types::store_api::StoreApi;
pub use types::sync_store_api::SyncStoreApi;
