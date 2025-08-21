use crate::{
	library::find_co_secret::find_co_secret_by_membership, services::reducers::ReducerStorage, CoContext, CoToken,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle};
use co_network::bitswap::{BitswapMessage, Token};
use co_primitives::{Block, BlockLinks, CoId, DefaultParams, KnownMultiCodec, MultiCodec, Tags};
use co_storage::{BlockStorage, StorageError};
use libp2p::PeerId;

/// Bitswap service that responds to bitswap protocol requests.
pub struct Bitswap {
	context: CoContext,
}
impl Bitswap {
	pub fn new(context: CoContext) -> Self {
		Self { context }
	}
}
#[async_trait]
impl Actor for Bitswap {
	type Message = BitswapMessage<DefaultParams>;
	type State = ();
	type Initialize = ();

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		_initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(())
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		_state: &mut Self::State,
	) -> Result<(), ActorError> {
		// handle
		match message {
			BitswapMessage::Contains(cid, remote_peer, tokens, response) => {
				self.context.tasks().spawn({
					let context = self.context.clone();
					async move {
						response.send(contains(context, cid, remote_peer, tokens).await).ok();
					}
				});
			},
			BitswapMessage::Get(cid, remote_peer, tokens, response) => {
				self.context.tasks().spawn({
					let context = self.context.clone();
					async move {
						response.send(get(context, cid, remote_peer, tokens).await).ok();
					}
				});
			},
			BitswapMessage::Insert(block, remote_peer, tokens, response) => {
				self.context.tasks().spawn({
					let context = self.context.clone();
					async move {
						response.send(insert(context, block, remote_peer, tokens).await).ok();
					}
				});
			},
			BitswapMessage::MissingBlocks(cid, tokens, response) => {
				self.context.tasks().spawn({
					let context = self.context.clone();
					async move {
						response.send(missing_blocks(context, cid, tokens).await).ok();
					}
				});
			},
		}

		// result
		Ok(())
	}
}

async fn first_valid_token(
	context: &CoContext,
	remote_peer: &PeerId,
	tokens: &[Token],
) -> Result<Option<CoId>, StorageError> {
	let local_co = context.local_co_reducer().await?;
	for token in tokens.iter() {
		match CoToken::from_bitswap_token(&token) {
			Ok(co_token) => {
				let secret = find_co_secret_by_membership(&local_co, &co_token.body.1).await?;
				if let Some(secret) = secret {
					if co_token.verify(&secret, &remote_peer, None) {
						return Ok(Some(co_token.body.1));
					}
				}
			},
			Err(err) => return Err(StorageError::InvalidArgument(err)),
		}
	}
	return Ok(None);
}

/// Get first CO token (local use only).
/// Warning: The token will not be validated as this is only intendetd for local operations.
async fn enforce_first_token_local(context: &CoContext, tokens: &[Token]) -> Result<CoId, StorageError> {
	// get the co which initialited the request
	let mut co = None;
	for token in tokens.iter() {
		match CoToken::from_bitswap_token(&token) {
			Ok(co_token) => {
				co = Some(co_token.body.1);
				break;
			},
			Err(err) => return Err(StorageError::InvalidArgument(err.into())),
		}
	}
	let co = co.ok_or(StorageError::InvalidArgument(anyhow!("Insert is only allowed in context of a CO")))?;
	if !context.is_shared(&co).await {
		return Err(StorageError::InvalidArgument(anyhow!("Insert is only allowed for shared COs")));
	}
	Ok(co)
}

async fn contains(context: CoContext, cid: Cid, remote_peer: PeerId, tokens: Vec<Token>) -> Result<bool, StorageError> {
	match get(context, cid, remote_peer, tokens).await {
		Ok(Some(_)) => Ok(true),
		Ok(None) => Ok(false),
		Err(StorageError::NotFound(_, _)) => Ok(false),
		Err(err) => Err(err),
	}
}

#[tracing::instrument(level = tracing::Level::TRACE, name = "bitswap-service-get", err(Debug), skip(context, tokens))]
async fn get(
	context: CoContext,
	cid: Cid,
	remote_peer: PeerId,
	tokens: Vec<Token>,
) -> Result<Option<Vec<u8>>, StorageError> {
	// validate token
	let co = first_valid_token(&context, &remote_peer, &tokens).await?;

	// log
	tracing::trace!(?cid, ?co, tokens = ?tokens.iter().map(CoToken::from_bitswap_token).collect::<Vec<_>>(), "bitswap-service-get");

	// storage
	let storage = match co {
		Some(co) => {
			if !context.is_shared(&co).await {
				return Err(StorageError::InvalidArgument(anyhow!("Bitswap is only allowed for Shared COs")));
			}
			context
				.inner
				.reducers_control()
				.storage(co)
				.await
				.map_err(|err| StorageError::NotFound(cid, err.into()))?
		},
		None => ReducerStorage::Default(context.inner.storage()),
	};

	// encrypted block:
	// - only allow if the token provides access
	if MultiCodec::is(&cid, KnownMultiCodec::CoEncryptedBlock) {
		let storage = storage
			.encrypted_storage()
			.ok_or(StorageError::NotFound(cid, anyhow!("Not allowed")))?;

		// make sure the block belongs to this CO.
		//  currently we just try to decrypt it.
		//  this also ensures we don't allow to get blocks from local co as it is always encrypted.
		storage.get_unencrypted(&cid).await?;

		// get
		let block = storage.get(&cid).await?;
		return Ok(Some(block.into_inner().1));
	}

	// unencrypted block:
	// - only allow if available in the CO or plain
	let block = storage.storage().get(&cid).await?;
	return Ok(Some(block.into_inner().1));
}

/// Insert block into storage.
/// Note: This request always has an local origin so the tokens was created by us and dont need to be validated.
async fn insert(
	context: CoContext,
	block: Block<DefaultParams>,
	_remote_peer: PeerId,
	tokens: Vec<Token>,
) -> Result<(), StorageError> {
	// get the co which initialited the request
	let co = enforce_first_token_local(&context, &tokens).await?;

	// get storage
	let storage = context
		.inner
		.reducers_control()
		.storage(co)
		.await
		.map_err(|err| StorageError::InvalidArgument(err.into()))?;

	// encrypted block:
	// - store (also validate encrypted by specified CO)
	if MultiCodec::is(block.cid(), KnownMultiCodec::CoEncryptedBlock) {
		let storage = storage
			.encrypted_storage()
			.ok_or(StorageError::InvalidArgument(anyhow!("Encrypted block for public CO")))?;
		storage.set_encrypted(block).await?;
		return Ok(());
	}

	// unencrypted block:
	storage.storage().set(block).await?;

	// result
	Ok(())
}

/// Read missing blocks.
/// Note: This request always has an local origin so the tokens was created by us and dont need to be validated.
async fn missing_blocks(context: CoContext, cid: Cid, tokens: Vec<Token>) -> Result<Vec<Cid>, StorageError> {
	// get the co which initialited the request
	let co = enforce_first_token_local(&context, &tokens).await?;

	// get storage
	let storage = context
		.inner
		.reducers_control()
		.storage(co)
		.await
		.map_err(|err| StorageError::InvalidArgument(err.into()))?
		.storage()
		.clone();

	// build
	let mut stack = vec![cid];
	let mut missing = vec![];
	while let Some(cid) = stack.pop() {
		match storage.get(&cid).await {
			Ok(block) => {
				stack.extend(BlockLinks::default().links(&block)?);
			},
			Err(StorageError::NotFound(_, _)) => {
				missing.push(cid);
			},
			Err(e) => return Err(e.into()),
		}
	}
	Ok(missing)
}
