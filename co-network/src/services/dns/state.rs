// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::DnsSource;
use co_actor::{ResponseStream, ResponseStreams};

#[derive(Debug, Default)]
pub struct DnsState {
	streams: ResponseStreams<Option<DnsSource>>,
}

impl DnsState {
	pub fn subscribe(&mut self, mut stream: ResponseStream<Option<DnsSource>>, source: Option<DnsSource>) {
		if stream.send(source).is_ok() {
			self.streams.push(stream);
		}
	}

	pub fn publish(&mut self, source: DnsSource) {
		self.streams.send(Some(source));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use co_actor::ResponseStreamReceiver;
	use futures::{FutureExt, StreamExt};

	#[tokio::test]
	async fn source_stream_gets_initial_value_and_changes() {
		let mut state = DnsState::default();
		let (stream, mut receiver) = ResponseStreamReceiver::new();

		state.subscribe(stream, Some(DnsSource::Fallback));
		assert_eq!(receiver.next().await, Some(Some(DnsSource::Fallback)));

		state.publish(DnsSource::System);
		assert_eq!(receiver.next().await, Some(Some(DnsSource::System)));
	}

	#[tokio::test]
	async fn source_stream_gets_none_before_native_source_exists() {
		let mut state = DnsState::default();
		let (stream, mut receiver) = ResponseStreamReceiver::new();

		state.subscribe(stream, None);

		assert_eq!(receiver.next().await, Some(None));
		assert!(receiver.next().now_or_never().is_none());
	}
}
