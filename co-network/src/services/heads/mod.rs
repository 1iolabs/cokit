// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

mod action;
mod actor;
mod api;
mod epics;

pub use action::{Heads, HeadsAction, PublishAction, ReceiveAction, SubscribeAction};
pub use actor::{HeadsActor, HeadsContext};
pub use api::HeadsApi;
