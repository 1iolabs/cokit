use crate::CoContext;
use co_sdk::CreateCo;

pub async fn create_co(co_context: CoContext, create: CreateCo) -> Result<(), anyhow::Error> {
	let (tx, rx) = tokio::sync::oneshot::channel();
	co_context.execute(|application| {
		let application = application.clone();
		tokio::spawn(async move {
			match application.create_co(create.clone()).await {
				Ok(_co) => {
					tracing::info!(?create, "create-co");
					tx.send(Ok(())).ok();
				},
				Err(err) => {
					tracing::error!(?err, ?create, "create-co-failed");
					tx.send(Err(err)).ok();
				},
			}
		});
	});
	Ok(rx.await??)
}
