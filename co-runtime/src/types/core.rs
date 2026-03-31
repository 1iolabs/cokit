// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_api::{Reducer, ReducerRef};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

/// A executable core reference.
#[derive(Clone)]
pub enum Core {
	Wasm(Cid),
	Binary(Vec<u8>),
	Native(ReducerRef),
}
impl Core {
	pub fn native<R, A>() -> Core
	where
		R: Reducer<A> + Default + 'static,
		A: Clone + DeserializeOwned + 'static,
	{
		Core::Native(ReducerRef::new::<R, A>())
	}
}
impl Debug for Core {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Wasm(arg0) => f.debug_tuple("Wasm").field(arg0).finish(),
			Self::Binary(arg0) => f.debug_tuple("Binary").field(&arg0.len()).finish(),
			Self::Native(_) => f.debug_tuple("Native").field(&"[native]").finish(),
		}
	}
}
impl From<Cid> for Core {
	fn from(value: Cid) -> Self {
		Core::Wasm(value)
	}
}
