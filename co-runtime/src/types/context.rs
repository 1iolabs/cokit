use cid::Cid;
use co_primitives::{BlockStorage, BlockStorageExt, DiagnosticMessage};
use derive_more::From;

#[derive(Debug, Clone)]
pub struct RuntimeContext {
	/// The acutual state.
	pub state: Option<Cid>,

	/// The event to apply to the state.
	pub event: Cid,

	/// Diagnostics returned from the COre.
	pub diagnostics: Vec<RuntimeDiagnosic>,
}
impl RuntimeContext {
	pub fn new(state: Option<Cid>, event: Cid) -> Self {
		Self { state, event, diagnostics: Default::default() }
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

#[derive(Debug, Clone, From)]
pub enum RuntimeDiagnosic {
	Reference(Cid),
	Message(DiagnosticMessage),
}
impl RuntimeDiagnosic {
	pub async fn resolve<S: BlockStorage>(&mut self, storage: &S) {
		if let RuntimeDiagnosic::Reference(diagnostic_cid) = &self {
			if let Ok(message) = storage.get_deserialized::<DiagnosticMessage>(diagnostic_cid).await {
				*self = RuntimeDiagnosic::Message(message);
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
