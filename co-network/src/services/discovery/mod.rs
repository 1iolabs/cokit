// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

pub mod action;
pub mod actor;
mod api;
mod epics;
mod event;
pub mod message;
pub mod state;
mod types;

pub use actor::{DiscoveryActor, DiscoveryContext};
pub use api::DiscoveryApi;
pub use event::Event;
pub use message::DiscoveryMessage;
pub use types::{DidDiscovery, DidDiscoveryMessageType, DiscoverMessage, Discovery};
