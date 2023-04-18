use crate::{
    library::generate_random_name::generate_random_name,
    types::{action::CoAction, context::CoContext, state::CoState},
    CoExecuteState, CoSettings, ErrorKind, IntoAction,
};
use anyhow::Result;
use co_node_edge::{chain_spec::development_config, service};
use co_state::{ActionObservable, StateObservable};
use libipld::Ipld;
use rxrust::prelude::*;
use sc_cli::ChainSpec;
use sc_service::{
    config::{
        ExecutionStrategy, KeystoreConfig, NetworkConfiguration, NodeKeyConfig,
        OffchainWorkerConfig, WasmExecutionMethod, WasmtimeInstantiationStrategy,
    },
    BasePath, Configuration, DatabaseSource, Role, RpcMethods,
};
use std::{convert::Infallible, path::PathBuf, sync::Arc};

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
                id: id,
                state: CoExecuteState::Stopped,
            };
            of(running)
                .merge(
                    observable::from_future(
                        tokio::spawn(async move {
                            tracing::info!("substrate-start");
                            let result = service::new_full(config); // todo: result
                            match result {
                                Ok(_task_manager) => {}
                                Err(err) => tracing::error!(?err, "substrate-error"),
                            }
                            tracing::info!("substrate-stop");
                        }),
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

struct CoRunner {
    pub id: String,
    pub co_path: PathBuf,
    // pub shared_params: SharedParams,
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
        Ok(NetworkConfiguration::new(
            format!("{}/{}", node_name, self.id),
            client_version,
            node_key,
            Some(self.co_path.join("network")),
        ))
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
            rpc_http: None,
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
            tracing_targets: None,
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
