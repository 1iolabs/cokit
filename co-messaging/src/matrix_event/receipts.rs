// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::EventContent;
use co_macros::co;
use schemars::JsonSchema;
use std::collections::BTreeMap;

/// Whether a receipt marks the sender's read cursor or received (delivered) cursor.
#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub enum ReceiptKind {
	Read,
	Received,
}

/// A public receipt sent into a room: the sender read (or received) all messages
/// up to `up_to`. Becomes visible to all CO participants.
#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct ReceiptContent {
	/// Read vs received cursor.
	pub kind: ReceiptKind,
	/// The ID of the latest event read/received by the user.
	pub up_to: String,
	/// The ID of the thread if the receipt is threaded.
	pub thread_id: Option<String>,
}

impl ReceiptContent {
	pub fn new(kind: ReceiptKind, up_to: impl Into<String>) -> Self {
		Self { kind, up_to: up_to.into(), thread_id: None }
	}
}

impl From<ReceiptContent> for EventContent {
	fn from(val: ReceiptContent) -> Self {
		EventContent::Receipt(val)
	}
}

// TODO move to another core as these should not be visible to other co participants
/// A read receipt for one specific room. Indicates that a user has read all messages up to the given event.
#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct PrivateReceipt {
	/// The ID of the event the receipt references
	pub event_id: String,
	/// The ID of the thread if receipt is threaded
	pub thread_id: String,
}

impl PrivateReceipt {
	pub fn new(event_id: impl Into<String>, thread_id: impl Into<String>) -> Self {
		Self { event_id: event_id.into(), thread_id: thread_id.into() }
	}
}

/// Private read receipts are saved in a users private CO so other users cannot infer the read status. The read map
/// in this event only needs to contain the delta on the users receipts. This means that there is no need to contain
/// the complete read receipt state in this event but only the changes.
#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct PrivateReceiptContent {
	/// Map of all room IDs to receipts
	#[serde(rename = "m.read.private")]
	pub read: BTreeMap<String, PrivateReceipt>,
}

impl PrivateReceiptContent {
	pub fn new(read: BTreeMap<String, PrivateReceipt>) -> Self {
		Self { read }
	}
}
