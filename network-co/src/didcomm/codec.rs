use futures::{AsyncRead, AsyncWrite};
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed};

use super::message::Message;

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
		let data = read_length_prefixed(socket, self.max_message_size_bytes).await?;
		if data.is_empty() {
			return Err(Error::Empty);
		}
		Ok(Message::Message(data))
	}

	pub async fn send_message<S: AsyncWrite + Unpin>(
		&self,
		socket: &mut S,
		Message::Message(data): Message,
	) -> Result<(), Error> {
		if data.len() > self.max_message_size_bytes {
			return Err(std::io::ErrorKind::InvalidInput.into());
		}
		write_length_prefixed(socket, data).await?;
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
