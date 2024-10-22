use std::{future::Future, time::Duration};
use tokio::sync::oneshot;

pub struct ResponseList<A, S> {
	pending: Vec<Response<A, S>>,
}
impl<A, S> Default for ResponseList<A, S> {
	fn default() -> Self {
		Self { pending: Default::default() }
	}
}
impl<A, S> ResponseList<A, S> {
	pub fn handle(&mut self, action: &A, state: &S) {
		self.pending.retain_mut(|item| !(item.accept)(action, state));
	}

	fn push(&mut self, response: Response<A, S>) {
		self.pending.push(response);
	}

	pub fn create<O: Send + 'static>(
		&mut self,
		filter: impl Fn(&A, &S) -> Option<O> + Sync + Send + 'static,
	) -> impl Future<Output = Result<O, anyhow::Error>> + Send + 'static {
		let (response, fut) = Response::create(filter);
		self.push(response);
		fut
	}

	pub fn create_with_timeout<O: Send + 'static>(
		&mut self,
		timeout: Duration,
		filter: impl Fn(&A, &S) -> Option<O> + Sync + Send + 'static,
	) -> impl Future<Output = Result<O, anyhow::Error>> + Send + 'static {
		let (response, fut) = Response::create_with_timeout(timeout, filter);
		self.push(response);
		fut
	}
}

struct Response<A, S> {
	accept: Box<dyn FnMut(&A, &S) -> bool + Sync + Send + 'static>,
}
impl<A, S> Response<A, S> {
	pub fn create<O: Send + 'static>(
		filter: impl Fn(&A, &S) -> Option<O> + Sync + Send + 'static,
	) -> (Self, impl Future<Output = Result<O, anyhow::Error>> + Send + 'static) {
		let (tx, rx) = oneshot::channel();
		let mut tx = Some(tx);
		(
			Self {
				accept: Box::new(move |action: &A, state: &S| {
					if let Some(response) = filter(action, state) {
						// respond
						if let Some(tx) = tx.take() {
							tx.send(response).ok();
						}

						// done
						true
					} else {
						// remove when receiver has dropped
						if let Some(tx) = &tx {
							if tx.is_closed() {
								return true;
							}
						}

						// not done
						false
					}
				}),
			},
			async move { Ok(rx.await?) },
		)
	}

	pub fn create_with_timeout<O: Send + 'static>(
		timeout: Duration,
		filter: impl Fn(&A, &S) -> Option<O> + Sync + Send + 'static,
	) -> (Self, impl Future<Output = Result<O, anyhow::Error>> + Send + 'static) {
		let (response, result) = Self::create(filter);
		(response, async move { Ok(tokio::time::timeout(timeout, result).await??) })
	}
}
