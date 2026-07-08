// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::source::{fallback_config, SystemConfSource};
use crate::services::dns::DnsSource;
use hickory_resolver::{
	config::{ResolverConfig, ResolverOpts},
	net::runtime::TokioRuntimeProvider,
	TokioResolver,
};
use std::{
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
	time::SystemTime,
};

/// How the refresher operates.
enum Mode {
	/// `NetworkDns::System`: follow the OS configuration, fall back when absent.
	Dynamic { source: Box<dyn SystemConfSource>, staleness: Option<PathBuf> },
	/// `NetworkDns::Cloudflare` / `NetworkDns::None`: fixed configuration.
	Static,
}

struct Tracking {
	source: DnsSource,
	/// mtime of the staleness file at the last refresh (`None` = file absent).
	mtime: Option<SystemTime>,
	/// The active resolver. Lookups clone the `Arc` out and resolve without the lock.
	resolver: Arc<TokioResolver>,
}

pub(crate) struct ResolverSnapshot {
	pub(crate) resolver: Arc<TokioResolver>,
	pub(crate) changed: Option<DnsSource>,
}

/// Owns the swappable resolver and the refresh state machine.
///
/// Never fails: every failure path degrades and logs - a non-empty OS config wins,
/// an empty or unreadable one falls back to the static resolver, and a resolver
/// build failure keeps the last-known-good resolver.
pub(crate) struct Refresher {
	mode: Mode,
	/// Refresh bookkeeping + the active resolver, under one lock so the staleness
	/// check, refresh and resolver load are atomic (concurrent dials on a changed
	/// mtime trigger exactly one refresh). Never held across an `await`;
	/// deliberately held across system-conf IO in refresh (rare, ms-scale);
	tracking: Mutex<Tracking>,
}

impl Refresher {
	/// Dynamic (System) refresher. Performs the initial refresh.
	pub(crate) fn system(source: Box<dyn SystemConfSource>, staleness: Option<PathBuf>) -> Arc<Self> {
		let refresher = Arc::new(Self {
			mode: Mode::Dynamic { source, staleness },
			tracking: Mutex::new(Tracking {
				source: DnsSource::Unavailable,
				mtime: None,
				resolver: Arc::new(empty_resolver()),
			}),
		});
		let _ = refresher.refresh();
		refresher
	}

	/// Static refresher for explicit `Cloudflare`/`None` modes.
	pub(crate) fn fixed(config: ResolverConfig, opts: ResolverOpts) -> Arc<Self> {
		let (resolver, source) = match build_resolver(config, opts) {
			Ok(resolver) => (resolver, DnsSource::Static),
			Err(err) => {
				tracing::warn!(?err, "network-dns-static-build-failed");
				(empty_resolver(), DnsSource::Unavailable)
			},
		};
		let refresher = Arc::new(Self {
			mode: Mode::Static,
			tracking: Mutex::new(Tracking { source, mtime: None, resolver: Arc::new(resolver) }),
		});
		refresher
	}

	/// Re-read the OS configuration and swap the resolver accordingly.
	/// No-op in static mode.
	pub(crate) fn refresh(&self) -> Option<DnsSource> {
		if matches!(self.mode, Mode::Static) {
			return None;
		}
		let mut tracking = self.tracking.lock().expect("dns tracking lock");
		self.refresh_locked(&mut tracking)
	}

	/// The refresh body; the caller holds the `tracking` lock.
	fn refresh_locked(&self, tracking: &mut Tracking) -> Option<DnsSource> {
		let Mode::Dynamic { source, staleness } = &self.mode else { return None };
		tracking.mtime = staleness.as_deref().and_then(mtime_of);

		let next_source = match source.read() {
			Ok((config, opts)) if !config.name_servers().is_empty() => match build_resolver(config, opts) {
				Ok(resolver) => {
					tracking.resolver = Arc::new(resolver);
					DnsSource::System
				},
				Err(err) => {
					// keep last-known-good resolver
					tracing::warn!(?err, "network-dns-refresh-build-failed");
					tracking.source
				},
			},
			result => {
				match result {
					Ok(_) => tracing::debug!("network-dns-system-config-empty"),
					Err(err) => tracing::debug!(?err, "network-dns-system-read-failed"),
				}
				let (config, opts) = fallback_config();
				match build_resolver(config, opts) {
					Ok(resolver) => {
						tracking.resolver = Arc::new(resolver);
						DnsSource::Fallback
					},
					Err(err) => {
						tracing::warn!(?err, "network-dns-fallback-build-failed");
						if tracking.source == DnsSource::Unavailable {
							tracking.resolver = Arc::new(empty_resolver());
						}
						tracking.source
					},
				}
			},
		};

		if next_source != tracking.source {
			tracing::info!(from = ?tracking.source, to = ?next_source, "network-dns-source-changed");
			tracking.source = next_source;
			return Some(next_source);
		}
		None
	}

	/// Current resolver snapshot; refreshes first if the OS config went stale.
	/// Check and refresh happen under one lock, so concurrent dials on a changed
	/// mtime trigger exactly one refresh.
	pub(crate) fn current(&self) -> ResolverSnapshot {
		let mut tracking = self.tracking.lock().expect("dns tracking lock");
		let changed = if self.is_stale(&tracking) { self.refresh_locked(&mut tracking) } else { None };
		ResolverSnapshot { resolver: tracking.resolver.clone(), changed }
	}

	pub(crate) fn source(&self) -> DnsSource {
		self.tracking.lock().expect("dns tracking lock").source
	}

	fn is_stale(&self, tracking: &Tracking) -> bool {
		let Mode::Dynamic { staleness: Some(path), .. } = &self.mode else { return false };
		mtime_of(path) != tracking.mtime
	}
}

fn mtime_of(path: &Path) -> Option<SystemTime> {
	std::fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

fn build_resolver(
	config: ResolverConfig,
	opts: ResolverOpts,
) -> Result<TokioResolver, hickory_resolver::net::NetError> {
	TokioResolver::builder_with_config(config, TokioRuntimeProvider::default())
		.with_options(opts)
		.build()
}

/// Resolver with zero nameservers: every lookup fails per-dial, nothing panics.
/// The empty config is a compile-time constant; hickory building it is treated as
/// infallible (upstream libp2p `.expect`s the same for every static config).
fn empty_resolver() -> TokioResolver {
	build_resolver(ResolverConfig::default(), ResolverOpts::default()).expect("static empty resolver config must build")
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

	/// Fake OS: 0 = valid config, 1 = empty config, 2 = read error.
	struct FakeConf {
		mode: Arc<AtomicU8>,
		reads: Arc<AtomicU64>,
	}

	impl SystemConfSource for FakeConf {
		fn read(&self) -> std::io::Result<(ResolverConfig, ResolverOpts)> {
			self.reads.fetch_add(1, Ordering::SeqCst);
			match self.mode.load(Ordering::SeqCst) {
				0 => Ok((ResolverConfig::udp_and_tcp(&hickory_resolver::config::GOOGLE), ResolverOpts::default())),
				1 => Ok((ResolverConfig::default(), ResolverOpts::default())),
				_ => Err(std::io::Error::other("no adapter")),
			}
		}
	}

	fn fake(initial: u8) -> (Arc<AtomicU8>, Arc<AtomicU64>, Box<FakeConf>) {
		let mode = Arc::new(AtomicU8::new(initial));
		let reads = Arc::new(AtomicU64::new(0));
		(mode.clone(), reads.clone(), Box::new(FakeConf { mode, reads }))
	}

	#[tokio::test]
	async fn read_error_falls_back() {
		let (_, reads, source) = fake(2);
		let refresher = Refresher::system(source, None);
		assert_eq!(refresher.source(), DnsSource::Fallback);
		assert_eq!(reads.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn empty_nameservers_fall_back() {
		let (_, _, source) = fake(1);
		let refresher = Refresher::system(source, None);
		assert_eq!(refresher.source(), DnsSource::Fallback);
	}

	#[tokio::test]
	async fn valid_config_is_system() {
		let (_, _, source) = fake(0);
		let refresher = Refresher::system(source, None);
		assert_eq!(refresher.source(), DnsSource::System);
	}

	#[tokio::test]
	async fn fallback_recovers_to_system() {
		let (mode, reads, source) = fake(2);
		let refresher = Refresher::system(source, None);
		assert_eq!(refresher.source(), DnsSource::Fallback);
		// adapter appears
		mode.store(0, Ordering::SeqCst);
		assert_eq!(refresher.refresh(), Some(DnsSource::System));
		assert_eq!(refresher.source(), DnsSource::System);
		assert_eq!(reads.load(Ordering::SeqCst), 2);
	}

	#[tokio::test]
	async fn staleness_triggers_refresh_on_current() {
		let path = std::env::temp_dir().join(format!("co-dns-test-{}", uuid::Uuid::new_v4()));
		std::fs::write(&path, "nameserver 127.0.0.1\n").unwrap();
		let (mode, reads, source) = fake(2);
		let refresher = Refresher::system(source, Some(path.clone()));
		assert_eq!(reads.load(Ordering::SeqCst), 1);

		// unchanged mtime -> current() must NOT refresh
		assert_eq!(refresher.current().changed, None);
		assert_eq!(reads.load(Ordering::SeqCst), 1);

		// touch the file with a distinct mtime -> current() must refresh
		mode.store(0, Ordering::SeqCst);
		let later = std::time::SystemTime::now() + std::time::Duration::from_secs(2);
		let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
		file.set_modified(later).unwrap();
		assert_eq!(refresher.current().changed, Some(DnsSource::System));
		assert_eq!(reads.load(Ordering::SeqCst), 2);
		assert_eq!(refresher.source(), DnsSource::System);

		std::fs::remove_file(&path).ok();
	}

	#[tokio::test]
	async fn static_mode_never_refreshes() {
		let refresher = Refresher::fixed(
			ResolverConfig::udp_and_tcp(&hickory_resolver::config::CLOUDFLARE),
			ResolverOpts::default(),
		);
		assert_eq!(refresher.source(), DnsSource::Static);
		assert_eq!(refresher.refresh(), None, "refresh must be a no-op in static mode");
	}
}
