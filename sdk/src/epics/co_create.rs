use crate::{
    types::{
        action::{Cause, CoAction},
        context::CoContext,
        state::CoState,
    },
    Co, CoCreate, Request, StorageType,
};
use anyhow::Result;
use co_state::{ActionObservable, StateObservable};
use libipld::{
    serde::{from_ipld, to_ipld},
    Cid,
};
use rxrust::prelude::*;
use std::{convert::Infallible, sync::Arc};

pub fn co_create<O: Observer<CoAction, Infallible> + 'static>(
    actions: ActionObservable<CoAction>,
    states: StateObservable<CoState>,
    context: Arc<CoContext>,
) -> impl Observable<CoAction, Infallible, O> {
    actions
        .filter_map(|action| match action {
            CoAction::CoCreate(request) => Some(request),
            _ => None,
        })
        .with_latest_from(states)
        .flat_map(move |(request, state)| {
            observable::from_future(
                create(context.storage(), state, request),
                context.scheduler(),
            )
            .flat_map(|i| from_iter(i))
        })
}

/// Store an `Co` and return `Cid` for it.
async fn create(storage: StorageType, state: CoState, create: Request<CoCreate>) -> Vec<CoAction> {
    // store co
    let co: Co = create.clone().request.into();
    let next_root: Result<Cid> = async {
        let ipld: libipld::Ipld = to_ipld(co.clone())?;
        let cid = storage.put_object(&ipld).await?;

        // update root
        modify_root(storage, state.root, |mut cids| {
            cids.push(cid);
            cids
        })
        .await
    }
    .await;

    // result
    match next_root {
        Ok(root) => vec![
            CoAction::RootChanged(root, Cause::Change),
            CoAction::CoCreateResponse(create.response(Ok(co))),
        ],
        Err(e) => vec![CoAction::CoCreateResponse(create.response(Err(e.into())))],
    }
}

async fn modify_root<F: FnOnce(Vec<Cid>) -> Vec<Cid>>(
    storage: StorageType,
    root: Option<Cid>,
    f: F,
) -> Result<Cid> {
    // read current cids
    let cids: Vec<Cid> = match root {
        Some(root_current) => from_ipld(storage.get_object(&root_current).await?)?,
        None => Vec::new(),
    };
    let next_cids = f(cids);

    // store
    let next_cids_ipld = to_ipld(next_cids)?;
    Ok(storage.put_object(&next_cids_ipld).await?)
}
