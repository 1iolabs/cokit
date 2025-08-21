use crate::{services::application::ApplicationMessage, Action};
use co_actor::ActorHandle;
use futures::{future::ready, pin_mut, StreamExt};
use std::time::Duration;

pub async fn wait_response<F, T>(handle: ActorHandle<ApplicationMessage>, filter: F) -> anyhow::Result<T>
where
	F: Fn(&Action) -> Option<T>,
{
	let actions = handle.stream(ApplicationMessage::Subscribe);
	let stream = actions
		.filter_map(|action| ready(action.ok()))
		.filter_map(move |action| ready(filter(&action)))
		.take(1);
	pin_mut!(stream);
	Ok(stream.next().await.ok_or(anyhow::anyhow!("No response"))?)
}

pub async fn wait_response_timeout<F, T>(
	handle: ActorHandle<ApplicationMessage>,
	timeout: Duration,
	filter: F,
) -> anyhow::Result<T>
where
	F: Fn(&Action) -> Option<T>,
{
	Ok(tokio::time::timeout(timeout, wait_response(handle, filter)).await??)
}
