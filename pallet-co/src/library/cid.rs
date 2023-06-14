#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = std)]
pub use cid::Cid;

#[cfg(not(feature = "std"))]
pub use sp_cid::Cid;
