use crate::{Block, BlockSerializer, BlockSerializerError, KnownMultiCodec, StoreParams};
use serde::{Deserialize, Serialize};

/// Wrapps a reference/link/Cid and applies attributes useful in context of a Co.
/// - A [`CoReference`] should be encoded with [`crate::KnownMultiCodec::CoReference`].
/// - A [`CoReference`] should be used with a narrow scope so that only the links to it are moved around.
/// - A [`CoReference`] should be used for cross Co references to state and heads as this has special semantics for
///   example encryption mapping.
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

	pub fn to_block<P>(&self) -> Result<Block<P>, BlockSerializerError>
	where
		P: StoreParams,
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
