// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use hickory_resolver::config::{ResolverConfig, ResolverOpts};

/// Seam for reading the OS DNS configuration, so tests can fake adapter states.
pub(crate) trait SystemConfSource: Send + Sync + 'static {
	fn read(&self) -> std::io::Result<(ResolverConfig, ResolverOpts)>;
}

/// Production source: hickory's `read_system_conf` (SCDynamicStore on Apple
/// platforms since hickory 0.26, `/etc/resolv.conf` on Linux).
pub(crate) struct RealSystemConf;
impl SystemConfSource for RealSystemConf {
	fn read(&self) -> std::io::Result<(ResolverConfig, ResolverOpts)> {
		// The concrete error type of `read_system_conf` differs per platform
		// (`ProtoError` on Apple/Windows/Android, `NetError` on Linux); both
		// implement `Display` via `thiserror`, so normalize through `to_string`.
		hickory_resolver::system_conf::read_system_conf().map_err(|err| std::io::Error::other(err.to_string()))
	}
}

/// Static fallback used when the OS has no usable DNS configuration.
pub(crate) fn fallback_config() -> (ResolverConfig, ResolverOpts) {
	(ResolverConfig::udp_and_tcp(&hickory_resolver::config::CLOUDFLARE), ResolverOpts::default())
}
