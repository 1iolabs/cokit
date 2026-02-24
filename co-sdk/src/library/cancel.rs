// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use futures::Future;
use tokio_util::sync::CancellationToken;

#[allow(dead_code)]
pub async fn cancel<F, R>(cancel: CancellationToken, fut: F) -> Option<R>
where
	F: Future<Output = Option<R>>,
{
	tokio::select! {
		_ = cancel.cancelled() => None,
		v = fut => v,
	}
}

#[allow(dead_code)]
pub async fn try_cancel<F, R, E>(cancel: CancellationToken, fut: F) -> Result<Option<R>, E>
where
	F: Future<Output = Result<Option<R>, E>>,
{
	tokio::select! {
		_ = cancel.cancelled() => Ok(None),
		v = fut => v,
	}
}
