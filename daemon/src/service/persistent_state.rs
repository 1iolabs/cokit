use std::io::ErrorKind;

use futures::Future;
use tokio::{sync::{Mutex}, fs};
use crate::entities::State;
use serde_json;

pub struct PersistentState {
    path: Mutex<String>,
    pub state: Mutex<State>,
}

impl PersistentState {
    pub async fn open(path: &str) -> anyhow::Result<Self> {
        let data = match fs::read_to_string(&path).await {
            Ok(data) => data,
            Err(e) => {
                return match e.kind() {
                    ErrorKind::NotFound => {
                        Ok(Self {
                            path: Mutex::new(path.to_string()),
                            state: Mutex::new(State { root: None }),
                        })
                    }
                    _ => Err(anyhow::Error::from(e).context(format!("Open file: {}", &path))),
                }
            },
        };
        let state: State = serde_json::from_str(&data)?;
        Ok(Self {
            path: Mutex::new(path.to_string()),
            state: Mutex::new(state),
        })
    }

    pub async fn state(&self) -> State {
        let state = self.state.lock().await;
        let clone = state.clone();
        return clone;
    }

    pub async fn mutate<F, Fut>(&self, mut f: F) -> anyhow::Result<()>
    where
        F: FnMut(&mut State) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut state = self.state.lock().await;
        f(&mut state).await;
        self.save().await?;
        Ok(())
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let path = &self.path.lock().await;
        let json = serde_json::to_string(&self.state().await)?;
        Ok(fs::write(path.as_str(), json).await?)
    }
}
