// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{DnsActor, DnsMessage, DnsSource};
use co_actor::{ActorHandle, ActorInstance};
use futures::{stream::BoxStream, StreamExt};

#[derive(Debug, Clone)]
pub struct DnsApi {
	handle: ActorHandle<DnsMessage>,
}

impl From<&ActorInstance<DnsActor>> for DnsApi {
	fn from(value: &ActorInstance<DnsActor>) -> Self {
		Self { handle: value.handle() }
	}
}

impl From<ActorHandle<DnsMessage>> for DnsApi {
	fn from(handle: ActorHandle<DnsMessage>) -> Self {
		Self { handle }
	}
}

impl DnsApi {
	pub async fn source(&self) -> Result<Option<DnsSource>, anyhow::Error> {
		Ok(self.handle.request(DnsMessage::Source).await?)
	}

	pub fn source_stream(&self) -> BoxStream<'static, Option<DnsSource>> {
		self.handle.stream_graceful(DnsMessage::SourceStream).boxed()
	}

	pub fn refresh(&self) -> Result<(), anyhow::Error> {
		self.handle.dispatch(DnsMessage::Refresh)?;
		Ok(())
	}
}
