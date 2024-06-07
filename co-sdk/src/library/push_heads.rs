use super::to_plain::to_plain;
use crate::{
	drivers::network::{
		tasks::co_heads::{CoHeadsNetworkTask, CoHeadsRequest},
		CoNetworkTaskSpawner,
	},
	CoCoreResolver, CoStorage, Reducer, ReducerChangedContext, ReducerChangedHandler,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_identity::PrivateIdentity;
use co_network::PeerProvider;
use co_primitives::CoId;
use co_storage::BlockStorageContentMapping;
use futures::{pin_mut, StreamExt};
use libipld::Cid;
use libp2p::PeerId;
use std::collections::BTreeSet;
use tokio::sync::watch;

///	Use PeerProvider to discover peers and send heads to them whenever a peer comes online or new heads are produced.
pub struct PushHeads<M> {
	heads: watch::Sender<BTreeSet<Cid>>,
	mapping: Option<M>,
	/// Force the mapping to be applied by returning an error when no mapping is found.
	force_mapping: bool,
	initialized: bool,
}
impl<M> PushHeads<M> {
	pub fn new<I, P>(
		spawner: CoNetworkTaskSpawner,
		co: CoId,
		identity: I,
		peer_provider: P,
		mapping: Option<M>,
		force_mapping: bool,
	) -> Self
	where
		I: PrivateIdentity + Clone + Send + Sync + 'static,
		P: PeerProvider + Send + Sync + 'static,
	{
		let (tx, rx) = watch::channel(Default::default());
		tokio::spawn(worker(spawner, co, rx, identity, peer_provider));
		Self { heads: tx, mapping, force_mapping, initialized: false }
	}
}
#[async_trait]
impl<M> ReducerChangedHandler<CoStorage, CoCoreResolver> for PushHeads<M>
where
	M: BlockStorageContentMapping + Send + Sync + 'static,
{
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<CoStorage, CoCoreResolver>,
		context: ReducerChangedContext,
	) -> Result<(), anyhow::Error> {
		// send local changes
		if context.is_local_change() || self.initialized {
			self.initialized = false;

			// map plain heads to encrypted heads
			let mut heads = reducer.heads().clone();
			if self.mapping.is_some() {
				heads = to_plain(&self.mapping, self.force_mapping, heads)
					.await
					.map_err(|err| anyhow!("Failed to map head: {}", err))?;
			}

			// send
			self.heads.send_replace(heads);
		}

		// done
		Ok(())
	}
}

async fn worker<I, P>(
	spawner: CoNetworkTaskSpawner,
	co: CoId,
	mut heads_watcher: watch::Receiver<BTreeSet<Cid>>,
	identity: I,
	peer_provider: P,
) where
	I: PrivateIdentity + Clone + Send + Sync + 'static,
	P: PeerProvider + Send + Sync + 'static,
{
	let identity = PrivateIdentity::boxed(identity);
	let mut peers: BTreeSet<PeerId> = Default::default();
	let peers_stream = peer_provider.peers().fuse();
	pin_mut!(peers_stream);
	loop {
		// next
		let heads = heads_watcher.borrow_and_update().clone();
		let notify: Option<(BTreeSet<Cid>, BTreeSet<PeerId>)> = tokio::select! {
			// wait for heads changed
			Ok(_) = heads_watcher.changed() => {
				// notify known peers about new heads
				let changed_heads = heads_watcher.borrow_and_update().clone();
				if !changed_heads.is_empty() && !peers.is_empty() && changed_heads != heads {
					Some((changed_heads.clone(), peers.clone()))
				} else {
					None
				}
			},

			// wait for new peers
			Some(next_peers) = peers_stream.next(), if !peers_stream.is_done() => {
				// get added peers
				let added: BTreeSet<PeerId> = next_peers.difference(&peers).cloned().collect();

				// update
				peers = next_peers;

				// notify the new peer
				if !added.is_empty() && !heads.is_empty() {
					Some((heads.clone(), added))
				} else {
					None
				}
			},

			// shutdown
			else => break,
		};

		// send
		if let Some(send) = notify {
			if spawner
				.spawn(CoHeadsNetworkTask::new(CoHeadsRequest::Heads {
					co: co.clone(),
					heads: send.0,
					peers: send.1,
					identity: identity.clone(),
				}))
				.is_err()
			{
				// exit loop when we can not spawn new tasks
				break;
			}
		}
	}
}
