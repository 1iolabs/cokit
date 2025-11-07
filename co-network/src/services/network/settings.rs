use crate::try_peer_id;
use multiaddr::Multiaddr;
use std::{collections::BTreeSet, time::Duration};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NetworkSettings {
	pub force_new_peer_id: bool,
	pub listen: Multiaddr,
	pub bootstrap: BTreeSet<Multiaddr>,
	pub relay: bool,
	pub keep_alive: Duration,
}
impl Default for NetworkSettings {
	fn default() -> Self {
		Self {
			force_new_peer_id: Default::default(),
			listen: Self::default_listen(),
			bootstrap: Self::default_bootstrap(),
			relay: false,
			keep_alive: Duration::from_secs(30),
		}
	}
}
impl NetworkSettings {
	pub fn new() -> Self {
		Self::default()
	}

	fn default_listen() -> Multiaddr {
		"/ip4/0.0.0.0/udp/0/quic-v1".parse().expect("to parse")
	}

	fn default_bootstrap() -> BTreeSet<Multiaddr> {
		let bootstrap =
			["/dns/bootstrap.1io.com/udp/5000/quic-v1/p2p/12D3KooWCoAgVrvp9dWqk3bds1paFcrK8HuYB8yY13XWaahwfm7o"];
		bootstrap.into_iter().map(|s| s.parse().expect("to parse")).collect()
	}

	pub fn with_force_new_peer_id(mut self, value: bool) -> Self {
		self.force_new_peer_id = value;
		self
	}

	/// Set listen endpoint.
	pub fn with_listen(mut self, listen: Multiaddr) -> Self {
		self.listen = listen;
		self
	}

	/// Set listen endpoint.
	pub fn with_listen_from_string(mut self, listen: &str) -> Result<Self, anyhow::Error> {
		self.listen = listen.parse()?;
		Ok(self)
	}

	/// Set local listen endpoint.
	pub fn with_localhost(mut self) -> Self {
		self.listen = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
		self
	}

	/// Clear all bootstrap endpoints.
	pub fn without_bootstrap(mut self) -> Self {
		self.bootstrap.clear();
		self
	}

	/// Add bootstrap endpoint.
	pub fn with_bootstrap(mut self, bootstrap: Multiaddr) -> Self {
		self.bootstrap.insert(bootstrap);
		self
	}

	/// Add bootstrap endpoint.
	pub fn with_bootstraps(mut self, bootstrap: impl IntoIterator<Item = Multiaddr>) -> Self {
		self.bootstrap.extend(bootstrap);
		self
	}

	/// Add bootstrap endpoint.
	pub fn with_bootstrap_from_string(mut self, bootstrap: &str) -> Result<Self, anyhow::Error> {
		self.bootstrap.insert(bootstrap.parse()?);
		Ok(self)
	}

	/// Enable relay mode to allow hole-punching over this swarm.
	pub fn with_relay(mut self) -> Self {
		self.relay = true;
		self
	}

	/// Validate if settings are correct.
	pub fn build(self) -> Result<Self, anyhow::Error> {
		for bootstrap in self.bootstrap.iter() {
			try_peer_id(bootstrap)?;
		}
		Ok(self)
	}
}
