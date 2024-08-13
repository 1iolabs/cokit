use crate::{reactive::context::ActionObservable, Action};
use futures::{future::ready, pin_mut, StreamExt};
use std::time::Duration;

pub async fn wait_response<F, T>(actions: ActionObservable, timeout: Duration, filter: F) -> anyhow::Result<T>
where
	F: Fn(&Action) -> Option<T>,
{
	let stream = actions.filter_map(move |action| ready(filter(&action)));
	let stream = tokio_stream::StreamExt::timeout(stream, timeout);
	pin_mut!(stream);
	Ok(stream.next().await.ok_or(anyhow::anyhow!("No response"))??)
}
