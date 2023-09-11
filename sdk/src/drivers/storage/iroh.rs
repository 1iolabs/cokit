use super::{Storage, StorageError};
use anyhow::Result;
use iroh_api::{Api, Block, Bytes, IpfsPath, OutType};
use iroh_embed::{Iroh, IrohBuilder, Libp2pConfig, P2pService, RocksStoreService};
use libipld::{
	cbor::DagCborCodec,
	cid::Cid,
	codec::Decode,
	ipld::Ipld,
	multihash::MultihashDigest,
	prelude::{Codec, Encode},
	IpldCodec,
};
use std::{collections::HashSet, io::Cursor, path::PathBuf};

#[derive(Debug)]
pub struct IrohStorage {
	iroh: Iroh,
	// base_path: String,
}

#[derive(Clone, Debug)]
pub struct IrohConfig {
	pub base_path: PathBuf,
	pub tcp_port: Option<u16>,
	pub quic_port: Option<u16>,
}

impl IrohStorage {
	pub async fn new(config: IrohConfig) -> Result<IrohStorage> {
		let store = RocksStoreService::new(config.base_path.join("store")).await?;
		let mut p2p_config = Libp2pConfig::default();
		p2p_config.kademlia = false;
		p2p_config.relay_server = false;
		p2p_config.gossipsub = false;
		p2p_config.mdns = true;
		p2p_config.bootstrap_peers = Vec::new();
		p2p_config.listening_multiaddrs = vec![
			format!("/ip4/0.0.0.0/tcp/{}", config.tcp_port.unwrap_or(0)).parse()?, // configured or random port
			format!("/ip4/0.0.0.0/udp/{}/quic-v1", config.quic_port.unwrap_or(0)).parse()?, // configured or random port
		];
		let p2p = P2pService::new(p2p_config, config.base_path.to_owned(), store.addr()).await?;
		let iroh: Iroh = IrohBuilder::new().store(store).p2p(p2p).build().await?;
		Ok(IrohStorage {
			// base_path: config.base_path.into_os_string().into_string().unwrap(),
			iroh,
		})
	}
}

#[async_trait::async_trait]
impl Storage for IrohStorage {
	async fn get_object(&self, cid: &Cid) -> Result<Ipld> {
		let path = IpfsPath::from_cid(cid.to_owned());
		let stream = self.iroh.api().get(&path)?;
		let buffer = read_bytes(stream).await?;
		Decode::decode(DagCborCodec, &mut Cursor::new(buffer.as_slice()))
	}

	async fn put_object(&self, data: &Ipld) -> Result<Cid> {
		let block = to_dag_cbor_block(&data)?;
		let client = get_client(self.iroh.api());
		let (cid, bytes, links) = block.into_parts();
		let result = cid.clone();
		client.try_store()?.put(cid, bytes, links).await?;
		Ok(result)
	}
}

fn get_client(api: &Api) -> iroh_rpc_client::Client {
	struct MyApi {
		pub client: iroh_rpc_client::Client,
		#[allow(dead_code)]
		resolver: iroh_resolver::resolver::Resolver<iroh_unixfs::content_loader::FullLoader>,
	}
	let my_api: MyApi = unsafe { std::mem::transmute(api.clone()) };
	my_api.client
}

fn to_dag_cbor_block(data: &Ipld) -> Result<Block> {
	let mut buffer = Vec::<u8>::new();
	data.encode(DagCborCodec, &mut buffer)?;
	let bytes: Bytes = buffer.into();
	let cid = Cid::new_v1(IpldCodec::DagCbor.into(), libipld::cid::multihash::Code::Sha2_256.digest(&bytes));
	let mut set = HashSet::new();
	DagCborCodec.references::<Ipld, _>(&bytes, &mut set)?;
	Ok(Block::new(cid, bytes, set.into_iter().collect()))
}

async fn read_bytes<T>(mut stream: futures::stream::BoxStream<'static, Result<(T, OutType)>>) -> Result<Vec<u8>> {
	use futures::stream::StreamExt;
	use tokio::io::AsyncReadExt;
	let mut buffer = Vec::<u8>::new();
	while let Some(v) = stream.next().await {
		match v?.1 {
			OutType::Reader(mut r) => {
				r.read_to_end(&mut buffer).await?;
			},
			_ => {
				// bail on other values as we expect binary data
				return Err(StorageError::UnexpectedDataType.into())
			},
		}
	}
	Ok(buffer)
}
