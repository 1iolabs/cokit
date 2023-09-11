pub(crate) mod libp2p;

pub trait Network {
	fn shutdown(self);
}
