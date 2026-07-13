// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

/// Where the active DNS resolver configuration comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DnsSource {
	/// OS-provided configuration (`NetworkDns::System`, config currently readable).
	System,
	/// Static fallback because the OS provided no usable configuration.
	Fallback,
	/// Explicitly static mode (`NetworkDns::Cloudflare` / `NetworkDns::None`).
	Static,
	/// No resolver could be built at all; DNS-named dials fail per-dial.
	Unavailable,
}
