use crate::{use_co_context, use_co_error, CoContext, CoErrorSignal};
use co_sdk::{Application, CoId, CreateCo, DidKeyIdentity, DidKeyProvider, CO_CORE_NAME_KEYSTORE};
use serde::Serialize;
use std::fmt::Debug;

/// CO API.
pub fn use_co_api(co: impl Into<CoId>) -> CoApi {
	let co: CoId = co.into();
	let context = use_co_context();
	let error = use_co_error();
	CoApi { co, context, error }
}

pub struct CoApi {
	co: CoId,
	context: CoContext,
	error: CoErrorSignal,
}
impl CoApi {
	pub fn with_error(self, error: CoErrorSignal) -> Self {
		Self { error, ..self }
	}

	pub fn create_identity(&self, seed: Vec<u8>, name: String) {
		self.context
			.execute_future_with_error(self.error, move |application| async move {
				create_identity(application, seed, name).await
			});
	}

	pub fn create_co(&self, co: CreateCo) {
		self.context
			.execute_future_with_error(self.error, move |application| async move { create_co(application, co).await });
	}

	pub fn dispatch<T>(&self, core: &str, action: T)
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
	{
		let co = self.co.clone();
		let core = core.to_owned();
		self.context
			.execute_future_with_error(self.error, move |application| async move {
				dispatch(application, &co, &core, &action).await
			});
	}
}

async fn create_co(application: Application, co: CreateCo) -> Result<(), anyhow::Error> {
	application.create_co(co).await?;
	Ok(())
}

async fn create_identity(application: Application, seed: Vec<u8>, name: String) -> Result<(), anyhow::Error> {
	let identity = DidKeyIdentity::generate(Some(&seed));
	let co = application.local_co_reducer().await?;
	let provider = DidKeyProvider::new(co, CO_CORE_NAME_KEYSTORE);
	provider.store(&identity, Some(name)).await?;
	Ok(())
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
