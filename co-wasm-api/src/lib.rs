mod co_v1;
mod library;
mod types;

pub use libipld::Cid;
pub use library::reduce;
pub use types::{Block, Context, Date, Did, Reducer, ReducerAction, Storage};

#[cfg(test)]
mod example;
