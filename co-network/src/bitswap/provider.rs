use libipld::store::StoreParams;
use libp2p_bitswap::Bitswap;

pub trait BitswapBehaviourProvider {
	type StoreParams: StoreParams;

	fn bitswap(&self) -> &Bitswap<Self::StoreParams>;
	fn bitswap_mut(&mut self) -> &mut Bitswap<Self::StoreParams>;
}
