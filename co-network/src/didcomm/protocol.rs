// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{codec, message::EncodedMessage};
use futures::{future::BoxFuture, AsyncWriteExt};
use libp2p::{core::UpgradeInfo, InboundUpgrade, OutboundUpgrade, Stream};
use std::iter;

pub const PROTOCOL_NAME: &str = "/didcomm/2";

#[derive(Debug, Clone)]
pub struct MessageProtocol {
	codec: codec::Codec,
	message: Option<EncodedMessage>,
}

impl MessageProtocol {
	pub fn inbound() -> Self {
		MessageProtocol { codec: codec::Codec::default(), message: None }
	}

	pub fn outbound(message: EncodedMessage) -> Self {
		MessageProtocol { codec: codec::Codec::default(), message: Some(message) }
	}

	pub fn into_message(self) -> Option<EncodedMessage> {
		self.message
	}
}

impl UpgradeInfo for MessageProtocol {
	type Info = &'static str;
	type InfoIter = iter::Once<Self::Info>;

	fn protocol_info(&self) -> Self::InfoIter {
		iter::once(PROTOCOL_NAME)
	}
}

impl InboundUpgrade<Stream> for MessageProtocol {
	type Output = EncodedMessage;
	type Error = codec::Error;
	type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

	fn upgrade_inbound(self, mut socket: Stream, _info: Self::Info) -> Self::Future {
		Box::pin(async move {
			// read
			tracing::trace!("didcomm-upgrade-inbound-read");
			let read = self.codec.receive_message(&mut socket);
			let result = read.await;
			if let Err(err) = &result {
				tracing::error!(?err, "didcomm-upgrade-inbound-read-failed");
			}

			// close substream
			tracing::trace!("didcomm-upgrade-inbound-close");
			match socket.close().await {
				Ok(_) => {},
				Err(err) => {
					tracing::warn!(?err, "didcomm-upgrade-inbound-close-failed");
				},
			}

			// result
			result
		})
	}
}

impl OutboundUpgrade<Stream> for MessageProtocol {
	type Output = Option<EncodedMessage>;
	type Error = codec::Error;
	type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

	fn upgrade_outbound(mut self, mut socket: Stream, _info: Self::Info) -> Self::Future {
		Box::pin(async move {
			let mut result = None;

			// write
			if let Some(message) = self.message.take() {
				tracing::trace!("didcomm-upgrade-outbound-write");
				result = Some(message.clone());
				let write = self.codec.send_message(&mut socket, message);
				let write_result = write.await;
				if let Err(err) = &write_result {
					tracing::error!(?err, "didcomm-upgrade-outbound-write-failed");
					write_result?;
				}
			}

			// close substream
			tracing::trace!("didcomm-upgrade-outbound-close");
			match socket.close().await {
				Ok(_) => {},
				Err(err) => {
					tracing::warn!(?err, "didcomm-upgrade-outbound-close-failed");
				},
			}

			// done
			Ok(result)
		})
	}
}
