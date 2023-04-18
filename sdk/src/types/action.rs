use super::reference::{Request, Response};
use crate::{Co, CoCreate, CoExecuteState, ErrorContext, IntoAction};
use libipld::{Cid, Ipld};

/// Co Application Actions.
///
/// Note: Adding new items should not be considered a breaking change.
#[derive(Debug, Clone, PartialEq)]
pub enum CoAction {
    Error(String, ErrorContext),
    Initialize,
    Initialized,
    SettingChanged(String, Ipld, Cause),
    RootChanged(Cid, Cause),
    CoCreate(Request<CoCreate>),
    CoCreateResponse(Response<Co>),
    CoStartup { id: String },
    CoShutdown { id: String },
    CoExecuteStateChanged { id: String, state: CoExecuteState },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cause {
    Initialize,
    Change,
}

impl IntoAction<CoAction> for anyhow::Error {
    fn into_action<C: Into<ErrorContext>>(self, context: C) -> CoAction {
        CoAction::Error(self.to_string(), context.into())
    }
}

impl IntoAction<CoAction> for anyhow::Result<CoAction> {
    fn into_action<C: Into<ErrorContext>>(self, context: C) -> CoAction {
        match self {
            Ok(a) => a,
            Err(e) => e.into_action(context),
        }
    }
}

impl IntoAction<Vec<CoAction>> for anyhow::Result<Vec<CoAction>> {
    fn into_action<C: Into<ErrorContext>>(self, context: C) -> Vec<CoAction> {
        match self {
            Ok(a) => a,
            Err(e) => vec![e.into_action(context)],
        }
    }
}
