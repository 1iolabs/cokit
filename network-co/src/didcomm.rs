use async_trait::async_trait;
use futures::{prelude::*, AsyncWriteExt};
use libp2p::{
	core::{
		upgrade::{read_length_prefixed, read_varint, write_length_prefixed, write_varint},
		ProtocolName,
	},
	request_response::{ProtocolSupport, RequestResponseCodec, RequestResponseConfig},
};
use std::{io, iter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DidCommStatus {
	Accepted = 202,
	TooManyRequests = 429,
	ServiceUnavilable = 503,
}

#[derive(Debug, Clone)]
pub struct DidCommProtocol();

#[derive(Debug, Clone)]
pub struct DidCommCodec();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DidCommRequest(Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DidCommResponse(DidCommStatus);

pub type DidCommBehavior = libp2p::request_response::RequestResponse<DidCommCodec>;

pub fn create_did_comm_behavior() -> DidCommBehavior {
	let protocols = iter::once((DidCommProtocol(), ProtocolSupport::Full));
	let cfg = RequestResponseConfig::default();
	libp2p::request_response::RequestResponse::new(DidCommCodec(), protocols.clone(), cfg.clone())
}

impl ProtocolName for DidCommProtocol {
	fn protocol_name(&self) -> &[u8] {
		"/didcomm/2".as_bytes()
	}
}

/// Request response codec to send/receive didcomm encoded messages.
/// TODO: Do we need an response?
#[async_trait]
impl RequestResponseCodec for DidCommCodec {
	type Protocol = DidCommProtocol;
	type Request = DidCommRequest;
	type Response = DidCommResponse; // ?

	async fn read_request<T>(&mut self, _: &DidCommProtocol, io: &mut T) -> io::Result<Self::Request>
	where
		T: AsyncRead + Unpin + Send,
	{
		let vec = read_length_prefixed(io, 1 * 1024 * 1024).await?;

		if vec.is_empty() {
			return Err(io::ErrorKind::UnexpectedEof.into())
		}

		Ok(DidCommRequest(vec))
	}

	async fn read_response<T>(&mut self, _: &DidCommProtocol, io: &mut T) -> io::Result<Self::Response>
	where
		T: AsyncRead + Unpin + Send,
	{
		let status = read_varint(io).await?;

		Ok(DidCommResponse(status.try_into()?))
	}

	async fn write_request<T>(
		&mut self,
		_: &DidCommProtocol,
		io: &mut T,
		DidCommRequest(data): Self::Request,
	) -> io::Result<()>
	where
		T: AsyncWrite + Unpin + Send,
	{
		write_length_prefixed(io, data).await?;
		io.close().await?;

		Ok(())
	}

	async fn write_response<T>(
		&mut self,
		_: &DidCommProtocol,
		io: &mut T,
		DidCommResponse(status): Self::Response,
	) -> io::Result<()>
	where
		T: AsyncWrite + Unpin + Send,
	{
		write_varint(io, status as usize).await?;
		io.close().await?;

		Ok(())
	}
}

impl TryFrom<usize> for DidCommStatus {
	type Error = io::ErrorKind;

	fn try_from(value: usize) -> Result<Self, Self::Error> {
		match value {
			202 => Ok(DidCommStatus::Accepted),
			429 => Ok(DidCommStatus::TooManyRequests),
			503 => Ok(DidCommStatus::ServiceUnavilable),
			_ => Err(io::ErrorKind::InvalidData),
		}
	}
}
