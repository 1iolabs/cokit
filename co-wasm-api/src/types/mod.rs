mod reducer;
mod storage;

pub use reducer::{Context, Reducer, ReducerAction};
pub use storage::Storage;

pub type Block = libipld::Block<libipld::DefaultParams>;
pub type Did = String;
pub type Date = u64; // unix timestamp
