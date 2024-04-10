use crate::{use_co_context, CoContext};
use co_sdk::{Application, CoId};
use serde::Serialize;
use std::fmt::Debug;

/// Create CO action dispatcher.
///
/// Todo: Identity
/// Todo: Error Handling
/// Todo: Enforce dispatch calls are actually processed in sequence not in parallel.
pub fn use_dispatch(co: &str) -> Dispatch {
	let co: CoId = co.into();
	let context = use_co_context();
	Dispatch { co, context }
}

#[derive(Debug, Clone)]
pub struct Dispatch {
	co: CoId,
	context: CoContext,
}
impl Dispatch {
	/// Dispatch action into CO COre.
	pub fn dispatch<T>(&self, core: &str, action: T)
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
	{
		let co = self.co.clone();
		let core = core.to_owned();
		self.context.execute(move |application| {
			let application = application.clone();
			tokio::spawn(async move {
				match dispatch(application.clone(), &co, &core, &action).await {
					Ok(_) => tracing::info!(?action, core, ?co, "dispatch"),
					Err(err) => tracing::error!(?err, ?action, core, ?co, "dispatch-failed"),
				}
			});
		});
	}
}

async fn dispatch<T>(application: Application, co: &CoId, core: &str, item: &T) -> Result<(), anyhow::Error>
where
	T: Serialize + Debug + Send + Sync + Clone + 'static,
{
	let identity = application.local_identity();
	let reducer = application
		.co_reducer(co)
		.await?
		.ok_or_else(|| anyhow::anyhow!("Co not found: {}", co))?;
	reducer.push(&identity, &core, item).await?;
	Ok(())
}
