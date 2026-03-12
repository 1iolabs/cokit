// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{services::application::ApplicationMessage, Action};
use co_actor::{time, ActorHandle};
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
	stream.next().await.ok_or(anyhow::anyhow!("No response"))
}

pub async fn wait_response_timeout<F, T>(
	handle: ActorHandle<ApplicationMessage>,
	timeout: Duration,
	filter: F,
) -> anyhow::Result<T>
where
	F: Fn(&Action) -> Option<T>,
{
	time::timeout(timeout, wait_response(handle, filter))
		.await
		.map_err(|_| anyhow::anyhow!("Timeout"))?
}

pub async fn request_response<F, T>(
	handle: ActorHandle<ApplicationMessage>,
	request: Action,
	response: F,
) -> anyhow::Result<T>
where
	F: Fn(&Action) -> Option<T>,
{
	let response_fut = wait_response(handle.clone(), response);
	handle.dispatch(request)?;
	response_fut.await
}

pub async fn request_response_timeout<F, T>(
	handle: ActorHandle<ApplicationMessage>,
	timeout: Duration,
	request: Action,
	response: F,
) -> anyhow::Result<T>
where
	F: Fn(&Action) -> Option<T>,
{
	let response_fut = wait_response_timeout(handle.clone(), timeout, response);
	handle.dispatch(request)?;
	response_fut.await
}
