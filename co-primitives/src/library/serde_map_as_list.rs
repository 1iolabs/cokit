use serde::{
	de::{SeqAccess, Visitor},
	ser::SerializeSeq,
	Deserialize, Deserializer, Serialize, Serializer,
};
use std::{iter::FromIterator, marker::PhantomData};

pub fn serialize<S, K, V, M>(map: &M, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
	K: Serialize,
	V: Serialize,
	for<'a> &'a M: IntoIterator<Item = (&'a K, &'a V)>,
{
	let iter = map.into_iter();
	let (len, _) = iter.size_hint();
	let mut seq = serializer.serialize_seq(Some(len))?;
	for (k, v) in iter {
		seq.serialize_element(&(&k, v))?;
	}
	seq.end()
}

pub fn deserialize<'de, D, K, V, M>(deserializer: D) -> Result<M, D::Error>
where
	D: Deserializer<'de>,
	K: Deserialize<'de>,
	V: Deserialize<'de>,
	M: FromIterator<(K, V)> + Default + Extend<(K, V)>,
{
	struct MapAsListVisitor<K, V, M> {
		marker: PhantomData<(K, V, M)>,
	}

	impl<'de, K, V, M> Visitor<'de> for MapAsListVisitor<K, V, M>
	where
		K: Deserialize<'de>,
		V: Deserialize<'de>,
		M: Default + Extend<(K, V)>,
	{
		type Value = M;

		fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.write_str("a sequence of [key, value] pairs")
		}

		fn visit_seq<A>(self, mut seq: A) -> Result<M, A::Error>
		where
			A: SeqAccess<'de>,
		{
			let mut map = M::default();

			while let Some((k, v)) = seq.next_element::<(K, V)>()? {
				// extend with a single element; cheap, no big Vec
				map.extend(std::iter::once((k, v)));
			}

			Ok(map)
		}
	}

	deserializer.deserialize_seq(MapAsListVisitor { marker: PhantomData::<(K, V, M)>::default() })
}

#[cfg(test)]
mod tests {
	use crate::{from_cbor, serde_map_as_list, to_cbor};
	use serde::{Deserialize, Serialize};
	use std::collections::BTreeMap;

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(transparent)]
	struct Wrapper(#[serde(with = "serde_map_as_list")] BTreeMap<u32, String>);

	#[test]
	fn test_list_btreemap() {
		let mut map = BTreeMap::new();
		map.insert(1, "one".to_owned());
		map.insert(2, "two".to_owned());

		let wrapper = Wrapper(map);

		let json = serde_json::to_string(&wrapper).unwrap();
		// check JSON as BTreeMap gives deterministic order
		assert_eq!(json, r#"[[1,"one"],[2,"two"]]"#);
	}

	#[test]
	fn test_roundtrip_btreemap_dagcbor() {
		let mut map = BTreeMap::new();
		map.insert(42, "forty-two".to_owned());
		map.insert(7, "seven".to_owned());

		let wrapper = Wrapper(map.clone());

		let bytes = to_cbor(&wrapper).unwrap();
		let decoded: Wrapper = from_cbor(&bytes).unwrap();

		assert_eq!(decoded.0, map);
	}

	#[test]
	fn test_roundtrip_btreemap_empty() {
		let map: BTreeMap<u32, String> = BTreeMap::new();
		let wrapper = Wrapper(map.clone());

		let bytes = to_cbor(&wrapper).unwrap();
		let decoded: Wrapper = from_cbor(&bytes).unwrap();

		assert!(decoded.0.is_empty());
		assert_eq!(decoded.0, map);
	}
}
