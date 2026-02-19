// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{use_co_context, use_co_error, CoContext, CoErrorSignal};
use co_sdk::{
	state::Identity, Application, CoId, CreateCo, DidKeyIdentity, DidKeyProvider, PrivateIdentityBox,
	CO_CORE_NAME_KEYSTORE,
};
use serde::Serialize;
use std::fmt::Debug;

/// CO API.
pub fn use_co_api(co: impl Into<CoId>, identity: impl Into<Option<Identity>>) -> CoApi {
	let co: CoId = co.into();
	let context = use_co_context();
	let error = use_co_error();
	CoApi { co, context, error, identity: identity.into() }
}

/// CO API.
#[derive(Debug, Clone)]
pub struct CoApi {
	co: CoId,
	context: CoContext,
	error: CoErrorSignal,
	identity: Option<Identity>,
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
		let identity = self.identity.clone();
		self.context
			.execute_future_with_error(self.error, move |application| async move {
				create_co(application, identity, co).await
			});
	}

	pub fn dispatch<T>(&self, core: impl Into<String> + Debug, action: T)
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
	{
		let co = self.co.clone();
		let core = core.into();
		let identity = self.identity.clone();
		self.context
			.execute_future_with_error(self.error, move |application| async move {
				dispatch(application, identity, &co, &core, &action).await
			});
	}
}

async fn create_co(application: Application, identitiy: Option<Identity>, co: CreateCo) -> Result<(), anyhow::Error> {
	let private_identity: PrivateIdentityBox = match identitiy {
		None => PrivateIdentityBox::new(application.local_identity()),
		Some(value) => application.private_identity(&value.did).await?,
	};
	application.create_co(private_identity, co).await?;
	Ok(())
}

async fn create_identity(application: Application, seed: Vec<u8>, name: String) -> Result<(), anyhow::Error> {
	// create
	let identity = DidKeyIdentity::generate(Some(&seed));
	let co = application.local_co_reducer().await?;
	let provider = DidKeyProvider::new(co, CO_CORE_NAME_KEYSTORE);
	provider.store(&identity, Some(name)).await?;

	// result
	Ok(())
}

async fn dispatch<T>(
	application: Application,
	identitiy: Option<Identity>,
	co: &CoId,
	core: &str,
	item: &T,
) -> Result<(), anyhow::Error>
where
	T: Serialize + Debug + Send + Sync + Clone + 'static,
{
	let private_identity: PrivateIdentityBox = match identitiy {
		None => PrivateIdentityBox::new(application.local_identity()),
		Some(value) => application.private_identity(&value.did).await?,
	};
	let reducer = application
		.co_reducer(co)
		.await?
		.ok_or_else(|| anyhow::anyhow!("Co not found: {}", co))?;
	reducer.push(&private_identity, core, item).await?;
	Ok(())
}
