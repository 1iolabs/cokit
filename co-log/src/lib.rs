// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

mod library;
mod types;

pub use library::{
	entry::{EntryBlock, EntryError},
	log::{Log, PushPending, PushPendingStored},
	stream::{create_stream, LogIterator},
	verify_entry::{EntryVerifier, IdentityEntryVerifier, NoEntryVerifier, ReadOnlyEntryVerifier},
};
pub use types::error::LogError;
