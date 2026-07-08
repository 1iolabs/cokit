// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

mod actor;
mod api;
mod message;
mod source;
mod state;

pub use actor::{DnsActor, DnsInitialize};
pub use api::DnsApi;
pub use message::DnsMessage;
pub use source::DnsSource;
pub(crate) use state::DnsState;
