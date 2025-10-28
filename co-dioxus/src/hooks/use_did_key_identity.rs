use crate::use_co_context;
use co_sdk::{
	state, Application, CoId, CoReducerFactory, CoTryStreamExt, DidKeyIdentity, DidKeyProvider, Identity,
	CO_CORE_NAME_KEYSTORE, CO_ID_LOCAL,
};
use dioxus::{
	hooks::use_resource,
	prelude::RenderError,
	signals::{MappedSignal, Readable},
};
use futures::TryStreamExt;
use std::future::ready;

/// Use `did:key:` by name and creating a new one if it not exists.
pub fn use_did_key_identity(name: impl Into<String>) -> Result<MappedSignal<state::Identity>, RenderError> {
	let context = use_co_context();
	let result = use_resource({
		let name = name.into();
		let context = context.clone();
		move || {
			let name = name.clone();
			let context = context.clone();
			async move {
				context
					.try_with_application(move |application| ensure_identity(name, application))
					.await
					.map_err(|err| RenderError::Aborted(err.into()))
			}
		}
	})
	.suspend()?;

	// check for errors
	if let Err(err) = &*result.peek_unchecked() {
		return Err(err.clone());
	}

	// unrwap result
	Ok(result.map(|result| result.as_ref().expect("ok")))
}

/// Use the first or create an identity.
async fn ensure_identity(name: String, application: Application) -> Result<state::Identity, anyhow::Error> {
	let local_co = application.co().try_co_reducer(&CoId::new(CO_ID_LOCAL)).await?;
	let storage = local_co.storage();
	let identity = state::identities(storage, local_co.co_state().await, None)
		.try_filter(|identity| ready(&identity.name == &name && identity.did.starts_with("did:key:")))
		.try_first()
		.await?;
	Ok(if let Some(identity) = identity {
		identity
	} else {
		// create
		let identity = DidKeyIdentity::generate(None);
		let provider = DidKeyProvider::new(local_co, CO_CORE_NAME_KEYSTORE);
		provider.store(&identity, Some(name.clone())).await?;
		state::Identity { name, did: identity.identity().to_owned(), description: "".to_owned() }
	})
}
