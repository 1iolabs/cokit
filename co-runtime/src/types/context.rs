// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_api::to_cbor;
use co_primitives::{GuardOutput, ReducerOutput, Tags};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RuntimeContext {
	/// State CID. Set as output after state execution.
	#[serde(default)]
	pub state: Option<Cid>,

	/// Serialized input (e.g. `ReducerInput` or `GuardInput`).
	#[serde(default)]
	pub input: Vec<u8>,

	/// Execution result. `None` before execution, `Some(Ok(()))` on success, `Some(Err(msg))` on failure.
	#[serde(default)]
	pub result: Option<Result<(), String>>,

	/// Tags returned from the COre.
	#[serde(default)]
	pub tags: Tags,
}
impl RuntimeContext {
	pub fn new<T: Serialize>(input: &T) -> Result<Self, anyhow::Error> {
		Ok(Self { input: to_cbor(input)?, ..Default::default() })
	}

	pub fn apply_reducer_output(&mut self, output: ReducerOutput) {
		self.state = output.state;
		self.result = output.error.map(Err).or(Some(Ok(())));
		self.tags = output.tags;
	}

	pub fn apply_guard_output(&mut self, output: GuardOutput) {
		self.result = output.error.map(Err).or(Some(Ok(())));
		self.tags = output.tags;
	}

	/// Get execute result.
	pub fn ok(&self) -> Result<(), anyhow::Error> {
		match &self.result {
			Some(Err(error)) => Err(anyhow::anyhow!("{error}")),
			_ => Ok(()),
		}
	}
}
