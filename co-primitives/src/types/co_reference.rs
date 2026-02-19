// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{Block, BlockSerializer, BlockSerializerError, KnownMultiCodec};
use serde::{Deserialize, Serialize};

/// Wrapps a reference/link/Cid and applies attributes useful in context of a Co.
/// - A [`CoReference`] should be encoded with [`crate::KnownMultiCodec::CoReference`].
/// - A [`CoReference`] should be used with a narrow scope so that only the links to it are moved around.
/// - A [`CoReference`] should be used for cross Co references to state and heads as this has special semantics for
///   example encryption mapping.
///
/// ## FAQ
/// ### When to use [`CoReference::Weak`] and when [`co_primitives::WeakCid`]?
/// Use [`CoReference::Weak`] when you want to reference a root.
/// When the garbage collection reaches a [`CoReference::Weak`] it will not try to keep it alve with its parent.
/// Example: Keeping a historic root for reference or fast traversing.
///
/// Use [`co_primitives::WeakCid`] when the reference should not been handled as a link and will not be
/// accessed/traversed using this reference.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CoReference<T> {
	/// Handle the reference as a weak reference. Used for reference counting.
	#[serde(rename = "w")]
	Weak(T),
}
impl<T> CoReference<T> {
	pub fn into_value(self) -> T {
		match self {
			CoReference::Weak(t) => t,
		}
	}

	pub fn to_block(&self) -> Result<Block, BlockSerializerError>
	where
		T: Serialize,
	{
		BlockSerializer::new_codec(KnownMultiCodec::CoReference).serialize(self)
	}
}
impl<T> AsRef<T> for CoReference<T> {
	fn as_ref(&self) -> &T {
		match self {
			CoReference::Weak(t) => t,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::CoReference;
	use crate::{from_cbor, to_cbor};
	use serde::de::IgnoredAny;

	#[test]
	fn test_serialize() {
		let item = CoReference::Weak(1);
		let encoded = to_cbor(&item).unwrap();
		let decoded: CoReference<i32> = from_cbor(&encoded).unwrap();
		assert_eq!(decoded, item);
		let decoded: CoReference<IgnoredAny> = from_cbor(&encoded).unwrap();
		assert_eq!(decoded, CoReference::Weak(IgnoredAny));
	}
}
