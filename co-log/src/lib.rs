// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod library;
mod types;

pub use library::{
	entry::{EntryBlock, EntryError},
	log::Log,
	stream::{create_stream, LogIterator},
	verify_entry::{EntryVerifier, IdentityEntryVerifier, NoEntryVerifier, ReadOnlyEntryVerifier},
};
pub use types::error::LogError;
