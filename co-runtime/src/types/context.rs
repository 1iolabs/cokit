use cid::Cid;
use co_primitives::{BlockStorage, BlockStorageExt, DiagnosticMessage};

#[derive(Debug, Clone)]
pub struct RuntimeContext {
	/// The acutual state.
	pub state: Option<Cid>,

	/// The event to apply to the state.
	pub event: Cid,

	/// Diagnostics returned from the COre.
	pub diagnostics: Vec<Cid>,
}
impl RuntimeContext {
	pub fn new(state: Option<Cid>, event: Cid) -> Self {
		Self { state, event, diagnostics: Default::default() }
	}

	/// Test for failures in diagnostics.
	pub async fn ok<S: BlockStorage>(&self, storage: &S) -> Result<(), anyhow::Error> {
		for diagnostic_cid in self.diagnostics.iter() {
			if let Ok(diagnostic) = storage.get_deserialized::<DiagnosticMessage>(diagnostic_cid).await {
				match diagnostic {
					DiagnosticMessage::Failure(diagnostic) => {
						return Err(anyhow::anyhow!(diagnostic));
					},
				}
			}
		}
		Ok(())
	}
}
