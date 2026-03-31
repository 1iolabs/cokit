// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

pub mod action;
mod actor;
mod epics;
mod library;
mod message;
mod resolve;
mod state;

pub use actor::{Connections, ConnectionsContext};
pub use message::ConnectionMessage;
pub use resolve::{DynamicNetworkResolver, NetworkResolver};
pub use state::{CoConnection, ConnectionState, DidConnection, NetworkConnection, PeerConnection};
