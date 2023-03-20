use super::reference::{Request, Response};
use crate::{Co, CoCreate, ErrorContext, IntoAction};
use libipld::{Cid, Ipld};
use std::path::PathBuf;

/// Co Application Actions.
///
/// Note: Adding new items should not be considered a breaking change.
#[derive(Debug, Clone)]
pub enum CoAction {
    Error(String, ErrorContext),
    Initialize(PathBuf),
    Initialized,
    SettingChanged(String, Ipld, Cause),
    RootChanged(Cid, Cause),
    CoCreate(Request<CoCreate>),
    CoCreateResponse(Response<Co>),
}

#[derive(Debug, Clone)]
pub enum Cause {
    Initialize,
    Change,
}

impl IntoAction<CoAction> for anyhow::Result<CoAction> {
    fn into_action<C: Into<ErrorContext>>(self, context: C) -> CoAction {
        match self {
            Ok(a) => a,
            Err(e) => CoAction::Error(e.to_string(), context.into()),
        }
    }
}

impl IntoAction<Vec<CoAction>> for anyhow::Result<Vec<CoAction>> {
    fn into_action<C: Into<ErrorContext>>(self, context: C) -> Vec<CoAction> {
        match self {
            Ok(a) => a,
            Err(e) => vec![CoAction::Error(e.to_string(), context.into())],
        }
    }
}
