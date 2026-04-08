// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_primitives::TotalFloat64;
use derive_more::{From, TryInto};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tag Value
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, From, TryInto, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DynamicValue {
	/// Represents the absence of a value or the value undefined.
	Null,
	/// Represents a boolean value.
	#[from]
	Bool(bool),
	/// Represents an integer.
	#[from(types(i8, i16, i32))]
	Integer(i64),
	/// Represents a floating point value.
	Float(TotalFloat64),
	/// Represents an UTF-8 string.
	String(String),
	/// Represents a sequence of bytes.
	#[from]
	Bytes(Vec<u8>),
	/// Represents a list.
	#[from]
	List(Vec<DynamicValue>),
	/// Represents a map of strings.
	#[from]
	Map(BTreeMap<String, DynamicValue>),
	/// Represents an IPLD Link structure, implemented with Cid's (Content Identifiers)
	/// For more information see: https://ipld.io/docs/data-model/kinds/#link-kind
	#[from]
	Link(Cid),
}

impl DynamicValue {
	/// Test if the default value is assigned.
	pub fn is_empty(&self) -> bool {
		match self {
			DynamicValue::Null => true,
			DynamicValue::Bool(v) => v == &bool::default(),
			DynamicValue::Integer(v) => v == &i64::default(),
			DynamicValue::Float(v) => *v == TotalFloat64::from(0f64),
			DynamicValue::String(v) => v.is_empty(),
			DynamicValue::Bytes(v) => v.is_empty(),
			DynamicValue::List(v) => v.is_empty(),
			DynamicValue::Map(v) => v.is_empty(),
			DynamicValue::Link(_) => false,
		}
	}

	/// Access the string value.
	pub fn string(&self) -> Option<&str> {
		match self {
			DynamicValue::String(s) => Some(s),
			_ => None,
		}
	}
}
impl From<String> for DynamicValue {
	fn from(value: String) -> Self {
		Self::String(value)
	}
}
impl From<&str> for DynamicValue {
	fn from(value: &str) -> Self {
		Self::String(value.to_owned())
	}
}
