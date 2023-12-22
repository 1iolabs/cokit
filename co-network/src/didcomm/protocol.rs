use super::{codec, message::Message};
use futures::{future::BoxFuture, AsyncWriteExt, FutureExt};
use libp2p::{core::UpgradeInfo, swarm::NegotiatedSubstream, InboundUpgrade, OutboundUpgrade};
use std::iter;

pub const PROTOCOL_NAME: &'static str = "/didcomm/2";

#[derive(Debug, Clone)]
pub struct MessageProtocol {
	codec: codec::Codec,
	message: Option<Message>,
}

impl MessageProtocol {
	pub fn inbound() -> Self {
		MessageProtocol { codec: codec::Codec::default(), message: None }
	}

	pub fn outbound(message: Message) -> Self {
		MessageProtocol { codec: codec::Codec::default(), message: Some(message) }
	}

	pub fn into_message(self) -> Option<Message> {
		return self.message
	}
}

impl UpgradeInfo for MessageProtocol {
	type Info = &'static str;
	type InfoIter = iter::Once<Self::Info>;

	fn protocol_info(&self) -> Self::InfoIter {
		iter::once(PROTOCOL_NAME).into_iter()
	}
}

impl InboundUpgrade<NegotiatedSubstream> for MessageProtocol {
	type Output = Message;
	type Error = codec::Error;
	type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

	fn upgrade_inbound(self, mut socket: NegotiatedSubstream, _info: Self::Info) -> Self::Future {
		async move {
			// read
			let read = self.codec.receive_message(&mut socket);
			let message = read.await?;

			// close substream
			socket.close().await?;

			// result
			Ok(message)
		}
		.boxed()
	}
}

impl OutboundUpgrade<NegotiatedSubstream> for MessageProtocol {
	type Output = Option<Message>;
	type Error = codec::Error;
	type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

	fn upgrade_outbound(mut self, mut socket: NegotiatedSubstream, _info: Self::Info) -> Self::Future {
		async move {
			let mut result = None;

			// write
			if let Some(message) = self.message.take() {
				result = Some(message.clone());
				let write = self.codec.send_message(&mut socket, message);
				write.await?;
			}

			// close substream
			socket.close().await?;

			// done
			Ok(result)
		}
		.boxed()
	}
}
