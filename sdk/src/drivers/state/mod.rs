use crate::{
    epics::sdk_epics,
    types::{action::CoAction, context::CoContext, state::CoState},
    StorageType,
};
use co_state::{
    EpicMiddleware, EpicSubscription, FnReducer, LogMiddleware, SubjectMiddleware, SyncStore,
    SyncStoreApi,
};
use rxrust::{subject::SubjectThreads, subscription::Subscription};
use std::{convert::Infallible, path::PathBuf, rc::Rc, sync::Arc, thread::JoinHandle};

mod reducer;

pub type ReducerType = FnReducer<CoState, CoAction>;
pub type StoreType = Arc<dyn SyncStoreApi<ReducerType> + Send + Sync + 'static>;
pub type ActionsType = SubjectThreads<CoAction, Infallible>;

pub struct State {
    store: SyncStore<ReducerType>,
    epic_subscription: EpicSubscription<ReducerType>,
    epic_handle: JoinHandle<()>,
}

impl State {
    pub fn new(path: PathBuf, storage: StorageType, actions: ActionsType) -> Self {
        // middleware
        let (epic_middleware, epic_runner, epic_subscription) = EpicMiddleware::create();
        let subject_middlware = SubjectMiddleware::new(actions);

        // store
        let store = SyncStore::new(CoState::default(), FnReducer::new(reducer::reducer))
            .with_middleware(Box::new(epic_middleware))
            .with_middleware(Box::new(subject_middlware))
            .with_middleware(Box::new(LogMiddleware::new()));

        // epic
        let epic_store = store.clone();
        let epic_handle = std::thread::spawn(move || {
            #[cfg(feature = "futures-runtime")]
            {
                let mut pool = LocalPool::new();
                let local = pool.spawner();
                let context = Arc::new(CoContext::new(storage, local.clone()));
                let dispatch_store = epic_store.clone();
                local
                    .spawn_local(epic_runner.run(
                        Box::new(epic_store.clone()),
                        sdk_epics(),
                        context,
                    ))
                    .unwrap();
                local
                    .spawn_local(async move {
                        dispatch_store.dispatch(CoAction::Initialize(path)).await;
                    })
                    .unwrap();
                pool.run();
            }

            // tokio
            #[cfg(feature = "tokio-runtime")]
            {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                let local = Rc::new(tokio::task::LocalSet::new());
                let context = Arc::new(CoContext::new(storage, local.clone()));
                local.block_on(&runtime, async {
                    // setup and run futures
                    let dispatch_store = epic_store.clone();
                    let epic_handle = local.spawn_local(epic_runner.run(
                        Box::new(epic_store),
                        sdk_epics(),
                        context,
                    ));
                    let dispatch_handle = local.spawn_local(async move {
                        dispatch_store.dispatch(CoAction::Initialize(path)).await;
                    });

                    // run
                    dispatch_handle.await.unwrap();
                    epic_handle.await.unwrap();
                });
            }

            // log
            tracing::debug!("state-thread-shutdown");
        });

        // result
        Self {
            store,
            epic_subscription,
            epic_handle,
        }
    }

    pub fn store(&self) -> Arc<dyn SyncStoreApi<ReducerType> + Send + Sync + 'static> {
        Arc::new(self.store.clone())
    }

    pub fn shutdown(self) {
        self.epic_subscription.unsubscribe();
        self.epic_handle.join().unwrap();
    }
}
