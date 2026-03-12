// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::library::locals::{ApplicationLocal, Locals};
use async_trait::async_trait;
use futures::{stream, Stream};
use std::fmt::Debug;
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;

#[derive(Debug, Clone)]
pub struct MemoryLocals {
	watcher: tokio::sync::watch::Sender<Option<ApplicationLocal>>,
}
impl MemoryLocals {
	pub fn new(initial: Option<ApplicationLocal>) -> Self {
		Self { watcher: tokio::sync::watch::channel(initial).0 }
	}
}
#[async_trait]
impl Locals for MemoryLocals {
	async fn get(&self) -> Result<Vec<ApplicationLocal>, anyhow::Error> {
		Ok(match self.watcher.borrow().as_ref() {
			Some(local) => vec![local.clone()],
			None => Default::default(),
		})
	}

	async fn set(&mut self, local: ApplicationLocal) -> Result<(), anyhow::Error> {
		self.watcher.send_replace(Some(local));
		Ok(())
	}

	fn watch(&self) -> impl Stream<Item = ApplicationLocal> + Send + Sync + 'static {
		// tokio_stream::wrappers::WatchStream::new(self.watcher.subscribe()).filter_map(|item| ready(item))
		// as we only ever have our local state it can not changed from outside
		stream::empty()
	}
}
