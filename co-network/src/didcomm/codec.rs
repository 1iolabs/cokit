use super::message::Message;
use asynchronous_codec::{FramedRead, FramedWrite, LengthCodec};
use futures::{AsyncRead, AsyncWrite, SinkExt, TryStreamExt};

#[derive(Debug, Clone)]
pub struct Codec {
	max_message_size_bytes: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Failed to read/write")]
	Io(#[from] std::io::Error),
	#[error("Received empty message")]
	Empty,
}

impl From<std::io::ErrorKind> for Error {
	fn from(value: std::io::ErrorKind) -> Self {
		let error: std::io::Error = value.into();
		error.into()
	}
}

impl Codec {
	pub async fn receive_message<S: AsyncRead + Unpin>(&self, socket: &mut S) -> Result<Message, Error> {
		let mut framed = FramedRead::new(socket, LengthCodec {});
		let frame = framed.try_next().await?;
		match frame {
			None => Err(Error::Empty),
			Some(data) => Ok(Message::Message(data.into())),
		}
	}

	pub async fn send_message<S: AsyncWrite + Unpin>(
		&self,
		socket: &mut S,
		Message::Message(data): Message,
	) -> Result<(), Error> {
		if data.len() > self.max_message_size_bytes {
			return Err(std::io::ErrorKind::InvalidInput.into())
		}
		let mut framed = FramedWrite::new(socket, LengthCodec {});
		framed.send(data.into()).await?;
		framed.flush().await?;
		Ok(())
	}
}

impl Default for Codec {
	fn default() -> Self {
		Self {
            max_message_size_bytes: 1024 * 1024, // 1MB
        }
	}
}
