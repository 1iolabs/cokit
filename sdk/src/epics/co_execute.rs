use crate::{
    library::generate_random_name::generate_random_name,
    types::{action::CoAction, context::CoContext, state::CoState},
    ActionsType, CoExecuteState, CoSettings, ErrorKind, IntoAction,
};
use anyhow::Result;
use co_node_edge::{chain_spec::development_config, service};
use co_state::{ActionObservable, StateObservable};
use futures::FutureExt;
use libipld::Ipld;
use libp2p::multiaddr;
use rxrust::prelude::*;
use sc_cli::ChainSpec;
use sc_service::{
    config::{
        ExecutionStrategy, KeystoreConfig, NetworkConfiguration, NodeKeyConfig,
        OffchainWorkerConfig, WasmExecutionMethod, WasmtimeInstantiationStrategy,
    },
    BasePath, Configuration, DatabaseSource, Role, RpcMethods,
};
use std::{convert::Infallible, iter, net::Ipv4Addr, path::PathBuf, sync::Arc};
use tokio::select;

/// Startup CO blockchain.
///
/// In: CoAction::CoStartup
/// Out: CoAction::CoExecuteStateChanged
pub fn co_execute<O: Observer<CoAction, Infallible> + 'static>(
    actions: ActionObservable<CoAction>,
    states: StateObservable<CoState>,
    context: Arc<CoContext>,
) -> impl Observable<CoAction, Infallible, O> {
    actions
        .filter_map(|action| match action {
            CoAction::CoStartup { id } => Some(id),
            _ => None,
        })
        .with_latest_from(states)
        .flat_map(move |(id, state)| {
            let runner = CoRunner::new(&state.base_path, id.clone());
            let config = match runner.configuration(&state.settings) {
                Ok(value) => value,
                Err(err) => return of(err.into_action(ErrorKind::Warning)).box_it(),
            };
            let running = CoAction::CoExecuteStateChanged {
                id: id.clone(),
                state: CoExecuteState::Running,
            };
            let stopped = CoAction::CoExecuteStateChanged {
                id: id.clone(),
                state: CoExecuteState::Stopped,
            };
            let runner_actions = context.actions();
            of(running)
                .merge(
                    observable::from_future(
                        tokio::spawn(async move { runner.run(config, runner_actions).await }),
                        context.scheduler(),
                    )
                    .map_to(stopped),
                )
                .box_it()
        })
}

fn to_co_path(base_path: &PathBuf, co_id: &str) -> PathBuf {
    base_path.join(co_id)
}

// fn create_shutdown_signal(
//     actions: ActionObservable<CoAction>,
//     id: String,
// ) -> tokio::sync::oneshot::Receiver<bool> {
//     let (tx, rx) = tokio::sync::oneshot::channel();
//     let id2 = id.clone();
//     actions
//         .clone()
//         .filter_map(move |action| match action {
//             CoAction::CoStartup { id: action_id } if action_id == id => Some(false),
//             CoAction::Shutdown { force } => Some(force),
//             _ => None,
//         })
//         // force shutdown if action observable completes
//         .default_if_empty(true)
//         .take(1)
//         .take_until(actions.filter_map(move |action| match action {
//             CoAction::CoShutdown { id } if id == id2 => Some(()),
//             _ => None,
//         }))
//         .subscribe(|force| {
//             tx.send(force);
//         });
//     rx
// }

struct CoRunner {
    pub id: String,
    pub co_path: PathBuf,
    // pub shared_params: SharedParams,
}

impl CoRunner {
    #[tracing::instrument(name = "substrate", fields(co = &self.id), skip_all)]
    async fn run(&self, config: Configuration, actions: ActionsType) -> anyhow::Result<()> {
        // tokio::task::spawn_blocking(|| {
        //    let runtime = build_runtime().expect("Runtime to be build.");
        let shutdown_id = self.id.clone();
        let shutdown = actions
            .filter_map(move |action| match action {
                CoAction::CoShutdown { id } if shutdown_id == id => Some(false),
                CoAction::Shutdown { force } => Some(force),
                _ => None,
            })
            // force shutdown if action observable completes
            .default_if_empty(true)
            .take(1)
            .to_future()
            .fuse();

        // create
        let mut task_manager = service::new_full(config)?;
        let task_handle = task_manager.future().fuse();
        select! {
            res = task_handle => res?,
            _ = shutdown => {},
        }

        // shutdown
        //  drop task_manager to trigger shutdown
        let _task_registry = task_manager.into_task_registry();
        // todo: shutdown runtime futures?
        //       https://github.com/paritytech/substrate/blob/master/client/cli/src/runner.rs#L181-L183

        // result
        Ok(())
    }
}

impl CoRunner {
    fn new(base_path: &PathBuf, id: String) -> Self {
        let co_path = to_co_path(base_path, &id);
        // let shared_params = SharedParams {
        //     chain: "dev",
        //     dev: true,
        //     base_path: Some(co_path.clone()),
        //     log: (),
        //     detailed_log_output: (),
        //     disable_log_color: (),
        //     enable_log_reloading: (),
        //     tracing_targets: (),
        //     tracing_receiver: (),
        // };
        CoRunner {
            id,
            co_path,
            // shared_params,
        }
    }
}

// impl CliConfiguration for CoRunner {
//     fn shared_params(&self) -> &sc_cli::SharedParams {
//         &self.shared_params
//     }
// }

impl CoRunner {
    fn impl_name() -> &'static str {
        "co-runtime-edge"
    }

    fn impl_version() -> &'static str {
        "0.1.0"
    }

    fn node_key(&self) -> Result<NodeKeyConfig> {
        Ok(NodeKeyConfig::Ed25519(sc_network::config::Secret::File(
            self.co_path.join("secret_ed25519"), // TODO: keychain?
        )))
    }

    fn network_config(
        &self,
        settings: &CoSettings,
        node_key: NodeKeyConfig,
    ) -> Result<NetworkConfiguration> {
        let node_name = match settings.get("node_name") {
            Some(Ipld::String(value)) => value.clone(),
            _ => generate_random_name(64),
        };
        let client_version = format!("{}/{}", Self::impl_name(), Self::impl_version());
        let mut network_config = NetworkConfiguration::new(
            format!("{}/{}", node_name, self.id),
            client_version,
            node_key,
            Some(self.co_path.join("network")),
        );
        network_config.listen_addresses =
            vec![
                iter::once(multiaddr::Protocol::Ip4(Ipv4Addr::new(127, 0, 0, 1)))
                    .chain(iter::once(multiaddr::Protocol::Tcp(0)))
                    .collect(),
            ];
        Ok(network_config)
    }

    fn keystore_config(&self) -> Result<KeystoreConfig> {
        Ok(KeystoreConfig::Path {
            path: self.co_path.join("keystore"),
            password: None, // TODO: replace with OS keychain
        })
    }

    fn database_config(&self, settings: &CoSettings) -> Result<DatabaseSource> {
        Ok(DatabaseSource::RocksDb {
            path: self.co_path.join("db"),
            cache_size: get_int_setting(settings, "substrate.database_cache_size").unwrap_or(1024),
        })
    }

    fn chain_spec(&self) -> Result<Box<dyn ChainSpec>> {
        Ok(Box::new(development_config().unwrap())) // TODO: create CO chain spec
    }

    fn configuration(&self, settings: &CoSettings) -> Result<Configuration> {
        let node_key = self.node_key()?;
        let chain_spec = self.chain_spec()?;
        Ok(Configuration {
            impl_name: Self::impl_name().into(),
            impl_version: Self::impl_version().into(),
            role: Role::Full,
            tokio_handle: tokio::runtime::Handle::current(),
            transaction_pool: Default::default(),
            network: self.network_config(settings, node_key)?,
            keystore: self.keystore_config()?,
            keystore_remote: None,
            database: self.database_config(settings)?,
            trie_cache_maximum_size: get_int_setting::<usize>(
                settings,
                "substrate.trie_cache_maximum_size",
            ),
            state_pruning: None,
            blocks_pruning: sc_service::BlocksPruning::KeepAll,
            chain_spec,
            wasm_method: WasmExecutionMethod::Compiled {
                instantiation_strategy: WasmtimeInstantiationStrategy::PoolingCopyOnWrite,
            },
            wasm_runtime_overrides: None,
            execution_strategies: sc_service::config::ExecutionStrategies {
                syncing: ExecutionStrategy::AlwaysWasm,
                importing: ExecutionStrategy::AlwaysWasm,
                block_construction: ExecutionStrategy::AlwaysWasm,
                offchain_worker: ExecutionStrategy::AlwaysWasm,
                other: ExecutionStrategy::AlwaysWasm,
            },
            rpc_http: Some(std::net::SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0)),
            rpc_ws: None,
            rpc_ipc: None,
            rpc_ws_max_connections: None,
            rpc_cors: None,
            rpc_methods: RpcMethods::Safe,
            rpc_max_payload: None,
            rpc_max_request_size: None,
            rpc_max_response_size: None,
            rpc_id_provider: None,
            rpc_max_subs_per_conn: None,
            ws_max_out_buffer_capacity: None,
            prometheus_config: None,
            telemetry_endpoints: None,
            default_heap_pages: None,
            offchain_worker: OffchainWorkerConfig {
                enabled: true,
                indexing_enabled: false,
            },
            force_authoring: true,
            disable_grandpa: false,
            dev_key_seed: None,
            tracing_targets: Some(format!("node-edge/{}", self.id).to_owned()),
            tracing_receiver: sc_service::TracingReceiver::Log,
            max_runtime_instances: get_int_setting(settings, "substrate.max_runtime_instances")
                .unwrap_or(8),
            announce_block: true,
            base_path: Some(BasePath::new(self.co_path.clone())),
            informant_output_format: Default::default(),
            runtime_cache_size: 2,
        })
    }
}

fn get_int_setting<T: TryFrom<i128>>(settings: &CoSettings, key: &str) -> Option<T> {
    match settings.get(key) {
        Some(Ipld::Integer(value)) => match (*value).try_into() {
            Ok(i) => Some(i),
            _ => None,
        },
        _ => None,
    }
}
