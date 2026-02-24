// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use serde::{
	ser::{
		Error, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
		SerializeTupleStruct, SerializeTupleVariant, Serializer,
	},
	Serialize,
};

/// A `serde::Serializer` that simulates DAG-CBOR encoding and counts how many bytes it would write.
pub struct DagCborSizeSerializer {
	pub size: usize,
}
impl Default for DagCborSizeSerializer {
	fn default() -> Self {
		Self::new()
	}
}
impl DagCborSizeSerializer {
	pub fn new() -> Self {
		Self { size: 0 }
	}

	pub fn count<T: Serialize>(
		value: &T,
	) -> Result<usize, serde_ipld_dagcbor::error::EncodeError<DagCborSizeSerializerError>> {
		let mut counter = Self::new();
		value.serialize(&mut counter)?;
		Ok(counter.size)
	}

	fn write_len(&mut self, _major: u8, len: u64) {
		self.size += match len {
			0..=23 => 1,
			24..=0xFF => 2,
			0x100..=0xFFFF => 3,
			0x10000..=0xFFFFFFFF => 5,
			_ => 9,
		};
	}

	fn write_u64(&mut self, value: u64) {
		self.size += match value {
			0..=23 => 1,
			24..=0xFF => 2,
			0x100..=0xFFFF => 3,
			0x10000..=0xFFFFFFFF => 5,
			_ => 9,
		};
	}
}

#[derive(Debug, thiserror::Error)]
pub enum DagCborSizeSerializerError {}

impl Serializer for &mut DagCborSizeSerializer {
	type Ok = ();
	type Error = serde_ipld_dagcbor::error::EncodeError<DagCborSizeSerializerError>;

	type SerializeSeq = Self;
	type SerializeTuple = Self;
	type SerializeTupleStruct = Self;
	type SerializeTupleVariant = Self;
	type SerializeMap = Self;
	type SerializeStruct = Self;
	type SerializeStructVariant = Self;

	fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
		self.size += 1; // true=f5, false=f4
		Ok(())
	}

	fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
		self.serialize_i64(v as i64)
	}

	fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
		self.serialize_i64(v as i64)
	}

	fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
		self.serialize_i64(v as i64)
	}

	fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
		if v >= 0 {
			self.serialize_u64(v as u64)
		} else {
			let n = !(v as u64);
			self.size += match n {
				0..=23 => 1,
				24..=0xFF => 2,
				0x100..=0xFFFF => 3,
				0x10000..=0xFFFFFFFF => 5,
				_ => 9,
			};
			Ok(())
		}
	}

	fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
		self.serialize_u64(v as u64)
	}

	fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
		self.serialize_u64(v as u64)
	}

	fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
		self.serialize_u64(v as u64)
	}

	fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
		self.write_u64(v);
		Ok(())
	}

	fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
		self.write_len(3, v.len() as u64); // major type 3 (text)
		self.size += v.len();
		Ok(())
	}

	fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
		self.write_len(2, v.len() as u64); // major type 2 (bytes)
		self.size += v.len();
		Ok(())
	}

	fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
		Err(Self::Error::custom("Option::None is not allowed in DAG-CBOR"))
	}

	fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
		value.serialize(self)
	}

	fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
		self.size += 1; // null = f6
		Ok(())
	}

	fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
		self.serialize_unit()
	}

	fn serialize_unit_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
	) -> Result<Self::Ok, Self::Error> {
		variant.serialize(self)
	}

	fn serialize_newtype_struct<T: ?Sized + Serialize>(
		self,
		_name: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error> {
		value.serialize(self)
	}

	fn serialize_newtype_variant<T: ?Sized + Serialize>(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error> {
		self.size += 1; // map with 1 entry
		variant.serialize(&mut *self)?;
		value.serialize(&mut *self)
	}

	fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
		self.write_len(4, len.unwrap_or(0) as u64);
		Ok(self)
	}

	fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
		self.write_len(4, len as u64);
		Ok(self)
	}

	fn serialize_tuple_struct(
		self,
		_name: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleStruct, Self::Error> {
		self.write_len(4, len as u64);
		Ok(self)
	}

	fn serialize_tuple_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleVariant, Self::Error> {
		self.size += 1; // map of 1
		variant.serialize(&mut *self)?;
		self.write_len(4, len as u64);
		Ok(self)
	}

	fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		self.write_len(5, len.unwrap_or(0) as u64);
		Ok(self)
	}

	fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
		self.write_len(5, len as u64);
		Ok(self)
	}

	fn serialize_struct_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
		len: usize,
	) -> Result<Self::SerializeStructVariant, Self::Error> {
		self.size += 1; // map of 1
		variant.serialize(&mut *self)?;
		self.write_len(5, len as u64);
		Ok(self)
	}

	fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
		self.serialize_str(&v.to_string())
	}

	fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
		Err(Self::Error::custom("floats are not allowed in DAG-CBOR"))
	}

	fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
		Err(Self::Error::custom("floats are not allowed in DAG-CBOR"))
	}
}

// All compound containers use same struct
impl SerializeSeq for &mut DagCborSizeSerializer {
	type Ok = ();
	type Error = serde_ipld_dagcbor::error::EncodeError<DagCborSizeSerializerError>;

	fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> {
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}

impl SerializeTuple for &mut DagCborSizeSerializer {
	type Ok = ();
	type Error = serde_ipld_dagcbor::error::EncodeError<DagCborSizeSerializerError>;

	fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> {
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}
impl SerializeTupleStruct for &mut DagCborSizeSerializer {
	type Ok = ();
	type Error = serde_ipld_dagcbor::error::EncodeError<DagCborSizeSerializerError>;

	fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> {
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}
impl SerializeTupleVariant for &mut DagCborSizeSerializer {
	type Ok = ();
	type Error = serde_ipld_dagcbor::error::EncodeError<DagCborSizeSerializerError>;

	fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> {
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}
impl SerializeMap for &mut DagCborSizeSerializer {
	type Ok = ();
	type Error = serde_ipld_dagcbor::error::EncodeError<DagCborSizeSerializerError>;

	fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<Self::Ok, Self::Error> {
		key.serialize(&mut **self)
	}

	fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> {
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}
impl SerializeStruct for &mut DagCborSizeSerializer {
	type Ok = ();
	type Error = serde_ipld_dagcbor::error::EncodeError<DagCborSizeSerializerError>;

	fn serialize_field<T: ?Sized + Serialize>(
		&mut self,
		_key: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error> {
		_key.serialize(&mut **self)?;
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}
impl SerializeStructVariant for &mut DagCborSizeSerializer {
	type Ok = ();
	type Error = serde_ipld_dagcbor::error::EncodeError<DagCborSizeSerializerError>;

	fn serialize_field<T: ?Sized + Serialize>(
		&mut self,
		_key: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error> {
		_key.serialize(&mut **self)?;
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}
