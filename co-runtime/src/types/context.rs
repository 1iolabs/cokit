// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_api::to_cbor;
use co_primitives::{BlockStorage, BlockStorageExt, DiagnosticMessage};
use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeContext {
	/// The acutual state.
	pub state: Option<Cid>,

	/// The event to apply to the state.
	pub event: Cid,

	/// Runtime payload.
	pub payload: Vec<u8>,

	/// Diagnostics returned from the COre.
	pub diagnostics: Vec<RuntimeDiagnosic>,
}
impl RuntimeContext {
	pub fn new(state: Option<Cid>, event: Cid) -> Self {
		Self { state, event, payload: Default::default(), diagnostics: Default::default() }
	}

	pub fn new_payload<T: Serialize>(payload: &T) -> Result<Self, anyhow::Error> {
		Ok(Self {
			state: Default::default(),
			event: Default::default(),
			payload: to_cbor(payload)?,
			diagnostics: Default::default(),
		})
	}

	/// Resolve diagnostics to messages.
	pub async fn resolve_diagnostics<S: BlockStorage>(&mut self, storage: &S) -> Result<(), anyhow::Error> {
		for diagnostic in self.diagnostics.iter_mut() {
			diagnostic.resolve(storage).await;
		}
		Ok(())
	}

	/// Test for failures in diagnostics.
	pub async fn ok<S: BlockStorage>(&self, storage: &S) -> Result<(), anyhow::Error> {
		for diagnostic in self.diagnostics.iter() {
			let mut diagnostic = diagnostic.clone();
			diagnostic.resolve(storage).await;
			if let RuntimeDiagnosic::Message(message) = diagnostic {
				match message {
					DiagnosticMessage::Failure(diagnostic) => {
						return Err(anyhow::anyhow!(diagnostic));
					},
				}
			}
		}
		Ok(())
	}
}

#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub enum RuntimeDiagnosic {
	Reference(Cid),
	Message(DiagnosticMessage),
}
impl RuntimeDiagnosic {
	pub async fn resolve<S: BlockStorage>(&mut self, storage: &S) {
		if let RuntimeDiagnosic::Reference(diagnostic_cid) = &self {
			match storage.get_deserialized::<DiagnosticMessage>(diagnostic_cid).await {
				Ok(message) => {
					*self = RuntimeDiagnosic::Message(message);
				},
				Err(err) => {
					tracing::warn!(?diagnostic_cid, ?err, "resolve-diagnostic-failed");
				},
			}
		}
	}

	pub fn message(&self) -> Option<&DiagnosticMessage> {
		if let RuntimeDiagnosic::Message(message) = self {
			Some(message)
		} else {
			None
		}
	}
}
