mod crypto;
#[cfg(test)]
mod example;
mod library;
mod storage;
mod types;

// exports
pub use types::{
	cid::{Link, ResolveError},
	storage::{Storage, StorageError},
	Date, Did,
};
