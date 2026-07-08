// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::DnsSource;
use co_actor::{Response, ResponseStream};

#[derive(Debug)]
pub enum DnsMessage {
	/// Get current DNS resolver source diagnostics.
	Source(Response<Option<DnsSource>>),

	/// Subscribe to DNS resolver source diagnostics.
	SourceStream(ResponseStream<Option<DnsSource>>),

	/// Re-read OS DNS configuration now. No-op in static modes and on wasm.
	Refresh,

	/// Internal report from the live resolver/refresher.
	#[allow(dead_code)]
	SourceChanged(DnsSource),
}
