use serde::{Deserialize, Serialize};

/// Special CO metadata.
pub trait CoMetadata: Serialize {
	fn metadata() -> Vec<Metadata>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Metadata {
	/// External field namess.
	#[serde(rename = "ext")]
	External(Vec<String>),
}

#[derive(Clone, Serialize)]
pub struct WithCoMetadata<T: CoMetadata + Serialize> {
	#[serde(rename = "$co", skip_serializing_if = "Vec::is_empty")]
	co: Vec<Metadata>,
	#[serde(flatten)]
	value: T,
}
impl<T> WithCoMetadata<T>
where
	T: CoMetadata + Serialize,
{
	pub fn new(value: T) -> Self {
		Self { co: T::metadata(), value }
	}
}
impl<T: CoMetadata + Serialize> From<T> for WithCoMetadata<T> {
	fn from(value: T) -> Self {
		WithCoMetadata::new(value)
	}
}

// pub fn serialize_with_metadata<T: CoMetadata + Serialize, S: serde::Serializer>(
// 	serializer: S,
// 	value: T,
// ) -> Result<S::Ok, S::Error> {
// 	WithCoMetadata::new(value).serialize(serializer)
// }

/// Workaround for https://github.com/rust-lang/rust/issues/50133
// pub struct WithCoMetadataWrapper<T>(T);
// impl<T: CoMetadata> Into<WithCoMetadataWrapper<T>> for WithCoMetadata<T> {
// 	fn into(self) -> WithCoMetadataWrapper<T> {
// 		WithCoMetadataWrapper(self.value)
// 	}
// }
// impl<T: CoMetadata> Into<WithCoMetadata<T>> for T {
// 	fn into(self) -> WithCoMetadata<T> {
// 		WithCoMetadata::new(self)
// 	}
// }

#[cfg(test)]
mod tests {
	use super::WithCoMetadata;
	use crate::{CoMetadata, Metadata};
	use cid::Cid;
	use co_macros::TaggedFields;
	use serde::{Deserialize, Serialize};

	#[test]
	fn metadata() {
		#[derive(Debug, Clone, Serialize, Deserialize)]
		// error[E0275]: overflow evaluating the requirement `&mut Vec<u8>: Sized`
		// #[serde(into = "WithCoMetadata<Test>")]
		struct Test {
			hello: i32,
			world: Cid,
		}
		impl CoMetadata for Test {
			fn metadata() -> Vec<crate::Metadata> {
				vec![Metadata::External(vec!["world".to_owned()])]
			}
		}
		// impl Serialize for Test {
		// 	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		// 	where
		// 		S: serde::Serializer,
		// 	{
		// 		self.serialize_with_metadata(serializer)
		// 	}
		// }

		let json = serde_json::to_string_pretty(&WithCoMetadata::new(Test {
			hello: 1,
			world: Cid::try_from("bafyr4igf663hpuvdpvque42uxmkbacg5ubd4cgageulmwmqo33g2tpod7e").unwrap(),
		}))
		.unwrap();
		println!("{json}");
	}

	#[test]
	fn metadata_derive() {
		#[derive(Debug, Clone, Serialize, Deserialize, TaggedFields)]
		struct Test {
			hello: i32,
			#[tagged(external)]
			world: Cid,
		}
		let json = serde_json::to_string_pretty(&WithCoMetadata::new(Test {
			hello: 1,
			world: Cid::try_from("bafyr4igf663hpuvdpvque42uxmkbacg5ubd4cgageulmwmqo33g2tpod7e").unwrap(),
		}))
		.unwrap();
		println!("{json}");
	}
}

// impl<T> From<WithCoMetadata<T>> for T
// where
// 	T: CoMetadata,
// {
// 	fn from(value: WithCoMetadata<T>) -> Self {
// 		value.value
// 	}
// }

// impl<T: CoMetadata> Into<T> for WithCoMetadata<T> {
// 	fn into(self) -> T {
// 		self.value
// 	}
// }

// #[proc_macro_attribute]
// pub fn co_metadata(attr: TokenStream, input: TokenStream) -> TokenStream {
// }

// macro_rules! metadata_serialize {
// 	($t:ident) => {
// 		impl Serialize for X {
// 			fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
// 				#[derive(Serialize)]
// 				struct XSerialize {
// 					a: u32,
// 					b: u32,
// 					c: u32,
// 					$co: &'static str,
// 				}

// 				XSerialize { a: self.a, b: self.b, c: self.c, d: "only at serialization" }.serialize(serializer)
// 			}
// 		}
// 	};
// }

// impl Serialize for X {
// 	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
// 		#[derive(Serialize)]
// 		struct XSerialize {
// 			a: u32,
// 			b: u32,
// 			c: u32,
// 			d: &'static str,
// 		}

// 		XSerialize { a: self.a, b: self.b, c: self.c, d: "only at serialization" }.serialize(serializer)
// 	}
// }
