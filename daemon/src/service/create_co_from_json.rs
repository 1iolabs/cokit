
use std::collections::BTreeMap;
use std::sync::Arc;
use co_sdk::drivers::storage::Storage;
use co_sdk::entities::co::Co;
use libipld::Ipld;
use libipld::serde::{to_ipld, from_ipld};
use libipld::cid::Cid;
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use anyhow::Result;
use serde;

use super::PersistentState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Body {
    id: Option<String>,
    name: String,
    data: Option<BTreeMap<String, Ipld>>,
}
impl Into<Co> for Body {
    fn into(self) -> Co {
        Co {
            id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: self.name,
            data: self.data.unwrap_or_else(|| BTreeMap::new()),
        }
    }
}

/// Store JSON representation of an `Co` and return `Cid` for it.
pub async fn create_co_from_json(storage: Arc<dyn Storage + Send + Sync>, state: Arc<PersistentState>, data: Value) -> Result<Cid> {
    // create co
    let body: Body = serde_json::from_value(data)?;
    let create: Co = body.into();
    let ipld: libipld::Ipld = to_ipld(create)?;
    let cid = storage.put_object(&ipld).await?;

    // update state
    {
        let mut state_locked = state.state.lock().await;

        // read current cids
        let mut cids: Vec<Cid> = match state_locked.root {
            Some(root_current) => from_ipld(storage.get_object(&root_current).await?)?,
            None => Vec::new(),
        };
        cids.push(cid.clone());

        // store
        let cids_ipld = to_ipld(cids)?;
        let next_root = storage.put_object(&cids_ipld).await?;

        // update
        state_locked.root = Some(next_root);
        
    };
    
    // save
    state.save().await?;

    // result
    Ok(cid)
}
