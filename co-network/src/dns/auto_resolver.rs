// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{
	refresh::Refresher,
	source::{fallback_config, SystemConfSource},
	Resolver,
};
use crate::services::{
	dns::{DnsMessage, DnsSource},
	network::NetworkDns,
};
use co_actor::ActorHandle;
use hickory_resolver::{
	config::{ResolverConfig, ResolverOpts},
	lookup::Lookup,
	lookup_ip::LookupIp,
	net::NetError,
};
use std::{path::PathBuf, sync::Arc};

/// A cloneable, live-refreshing resolver for the DNS transport.
///
/// - `NetworkDns::System`: follows the OS configuration (SCDynamicStore/resolv.conf), falls back to Cloudflare when the
///   OS has none, and returns to the OS config on `refresh()` (recover path) or when the staleness file changes
///   (per-dial check).
/// - `NetworkDns::Cloudflare` / `NetworkDns::None`: fixed configuration, `refresh()` no-op.
#[derive(Clone)]
pub(crate) struct AutoResolver {
	refresher: Arc<Refresher>,
	reporter: ActorHandle<DnsMessage>,
}

impl AutoResolver {
	pub(crate) fn new(dns: &NetworkDns, source: Box<dyn SystemConfSource>, reporter: ActorHandle<DnsMessage>) -> Self {
		let refresher = match dns {
			NetworkDns::System => Refresher::system(source, staleness_path()),
			NetworkDns::Cloudflare => {
				let (config, opts) = fallback_config();
				Refresher::fixed(config, opts)
			},
			NetworkDns::None => Refresher::fixed(ResolverConfig::default(), ResolverOpts::default()),
		};
		Self { refresher, reporter }
	}

	/// Re-read the OS DNS configuration now (recover()-path trigger).
	pub(crate) fn refresh(&self) {
		if let Some(source) = self.refresher.refresh() {
			self.publish_source(source);
		}
	}

	pub(crate) fn source(&self) -> DnsSource {
		self.refresher.source()
	}

	fn current(&self) -> Arc<hickory_resolver::TokioResolver> {
		let current = self.refresher.current();
		if let Some(source) = current.changed {
			self.publish_source(source);
		}
		current.resolver
	}

	fn publish_source(&self, source: DnsSource) {
		if let Err(err) = self.reporter.dispatch(DnsMessage::SourceChanged(source)) {
			tracing::debug!(?err, ?source, "network-dns-source-report-failed");
		}
	}
}

/// File whose mtime signals an OS DNS change. Linux and macOS keep
/// `/etc/resolv.conf` in sync with the system configuration; iOS has no such file
/// (the `recover()` trigger covers it), Windows likewise. Android is unix-and-not-ios,
/// so it gets `Some("/etc/resolv.conf")` for a file that never exists there; the
/// absent file makes the staleness check inert (recover-only), which is intended.
fn staleness_path() -> Option<PathBuf> {
	#[cfg(all(unix, not(target_os = "ios")))]
	{
		Some(PathBuf::from("/etc/resolv.conf"))
	}
	#[cfg(any(not(unix), target_os = "ios"))]
	{
		None
	}
}

impl Resolver for AutoResolver {
	async fn lookup_ip(&self, name: String) -> Result<LookupIp, NetError> {
		self.current().lookup_ip(name).await
	}

	async fn ipv4_lookup(&self, name: String) -> Result<Lookup, NetError> {
		self.current().ipv4_lookup(name).await
	}

	async fn ipv6_lookup(&self, name: String) -> Result<Lookup, NetError> {
		self.current().ipv6_lookup(name).await
	}

	async fn txt_lookup(&self, name: String) -> Result<Lookup, NetError> {
		self.current().txt_lookup(name).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn cloudflare_mode_is_static() {
		let resolver = AutoResolver::new(
			&NetworkDns::Cloudflare,
			Box::new(super::super::source::RealSystemConf),
			ActorHandle::new_closed(),
		);
		assert_eq!(resolver.source(), DnsSource::Static);
	}

	#[tokio::test]
	async fn none_mode_is_static() {
		let resolver = AutoResolver::new(
			&NetworkDns::None,
			Box::new(super::super::source::RealSystemConf),
			ActorHandle::new_closed(),
		);
		assert_eq!(resolver.source(), DnsSource::Static);
	}
}
