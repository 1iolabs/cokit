use crate::EventContent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/**
 * Receipt events are used to indicate that all messages up to a specific event have been read by a user.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ReceiptType {
	#[serde(untagged)]
	Public(PublicReceiptContent),
	#[serde(untagged)]
	Private(PrivateReceiptContent),
}

impl From<ReceiptType> for EventContent {
	fn from(val: ReceiptType) -> Self {
		EventContent::Receipt(val)
	}
}

/**
 * These receipts are always sent into a room and indicate to all users that the messages sent up to the indicated
 * event were read by the user that sent this receipt event. This becomes public knowledge to all users
 * participating in the CO.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PublicReceiptContent {
	#[serde(rename = "m.read")]
	pub read: String, // The ID of the latest event read by the user
	pub thread_id: Option<String>, // The ID of the thread if receipt is threaded
}

impl From<PublicReceiptContent> for EventContent {
	fn from(val: PublicReceiptContent) -> Self {
		ReceiptType::Public(val).into()
	}
}

/**
 * A read receipt for one specific room. Indicates that a user has read all messages up to the given event.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PrivateReceipt {
	pub event_id: String,  // The ID of the event the receipt references
	pub thread_id: String, // The ID of the thread if receipt is threaded
}

/**
 * Private read receipts are saved in a users private CO so other users cannot infer the read status. The read map
 * in this event only needs to contain the delta on the users receipts. This means that there is no need to contain
 * the complete read receipt state in this event but only the changes.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PrivateReceiptContent {
	#[serde(rename = "m.read.private")]
	pub read: HashMap<String, PrivateReceipt>, // Map of all room IDs to receipts
}

impl From<PrivateReceiptContent> for EventContent {
	fn from(val: PrivateReceiptContent) -> Self {
		ReceiptType::Private(val).into()
	}
}
