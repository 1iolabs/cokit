// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

mod behaviour;
mod codec;
mod handler;
mod inbound;
mod message;
mod protocol;

pub use behaviour::{Behaviour, Config, Event};
pub use message::EncodedMessage;
