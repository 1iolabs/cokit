// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

//! DNS transport with a live-refreshing resolver.
//!
//! Why this exists (instead of `libp2p::dns` / SwarmBuilder's `with_dns`/`with_websocket`):
//! - libp2p's `with_websocket` hardcodes a *system-config* resolver internally; with no active adapter it fails the
//!   whole swarm build (`no nameservers found in config`).
//! - stock `libp2p-dns` cannot take a custom resolver (private fields, fixed `TokioResolver`), so the nameserver list
//!   is frozen at build time.
//!
//! The transport and [`Resolver`] trait come from the forked `co-libp2p-dns` crate, which adds a public
//! `Transport::with_resolver` constructor. [`AutoResolver`] implements [`Resolver`] over a live-swappable config.

mod auto_resolver;
mod refresh;
mod source;

pub(crate) use auto_resolver::AutoResolver;
pub(crate) use libp2p_dns::{Resolver, Transport};
pub(crate) use source::RealSystemConf;
