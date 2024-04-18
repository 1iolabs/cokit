use crate::CoContext;
use co_sdk::{Application, DidKeyIdentity, DidKeyProvider, Identity, CO_CORE_NAME_KEYSTORE};

pub async fn create_identity(co_context: CoContext, seed: Vec<u8>, name: String) -> Result<String, anyhow::Error> {
	let (tx, rx) = tokio::sync::oneshot::channel();
	co_context.execute(|application| {
		let application = application.clone();
		tokio::spawn(async move {
			match create(application, seed, name).await {
				Ok(identity) => {
					tracing::info!(identity, "create-identity");
					tx.send(Ok(identity)).ok();
				},
				Err(err) => {
					tracing::error!(?err, "create-identity-failed");
					tx.send(Err(err)).ok();
				},
			}
		});
	});
	Ok(rx.await??)
}

async fn create(application: Application, seed: Vec<u8>, name: String) -> Result<String, anyhow::Error> {
	let identity = DidKeyIdentity::generate(Some(&seed));
	let co = application.local_co_reducer().await?;
	let provider = DidKeyProvider::new(co, CO_CORE_NAME_KEYSTORE);
	provider.store(&identity, Some(name)).await?;
	Ok(identity.identity().to_owned())
}
