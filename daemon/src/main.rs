use crate::types::http_error::HttpResult;
use axum::extract::Path;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use co_sdk::Co;
use co_sdk::CoAction;
use co_sdk::CoCreate;
use co_sdk::Libp2pNetwork;
use co_sdk::Libp2pNetworkConfig;
use co_sdk::Request;
use co_sdk::State;
use co_sdk::StorageType;
use co_sdk::{ActionsType, CoExecuteState};
use co_sdk::{IrohConfig, IrohStorage, StoreType};
use libp2p::identity;
use libp2p::PeerId;
use library::read_cos::read_cos;
use rxrust::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::{json, to_value};
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use tokio::join;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

mod error;
mod library;
mod service;
mod types;

#[tokio::main]
async fn main() {
    // tracing
    let log_file = std::fs::File::create("daemon.log").unwrap();
    // let formatting_layer = BunyanFormattingLayer::new("co-daemon".into(), std::io::stdout);
    let formatting_layer = BunyanFormattingLayer::new("co-daemon".into(), log_file);
    let subscriber = Registry::default()
        .with(LevelFilter::INFO)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // driver: storage
    let config = IrohConfig {
        base_path: "/tmp/co/storage".into(),
        tcp_port: None,
        quic_port: None,
    };
    let storage: StorageType = Arc::new(IrohStorage::new(config).await.expect("storage"));

    // driver: network
    let network_key = identity::Keypair::generate_ed25519(); // todo: persist?
    let network_peer_id = PeerId::from(network_key.public());
    let network_config = Libp2pNetworkConfig {
        addr: None,
        bootstap: Vec::new(),
        keypair: network_key.clone(),
    };
    let network = Libp2pNetwork::new(network_config).await.expect("network");
    tracing::info!(peer_id = ?network_peer_id, "network");

    // driver: state
    let actions: ActionsType = ActionsType::default();
    let state = State::new("/tmp/co".into(), storage.clone(), actions.clone());
    let store: StoreType = state.store();

    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/cos", get(get_cos).post(post_cos))
        .route("/cos/:id/start", post(post_cos_start))
        .layer(Extension(storage))
        .layer(Extension(store))
        .layer(Extension(actions));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info! {addr = format!("http://{}/", addr), "listening"};
    let result: hyper::Result<()> = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await;
    result.unwrap();
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum GetCosItem {
    Ok(Co),
    Err {
        #[serde(rename = "$err")]
        err: String,
    },
}

#[axum_macros::debug_handler]
async fn get_cos(
    storage: Extension<StorageType>,
    store: Extension<StoreType>,
) -> HttpResult<(StatusCode, Json<Vec<GetCosItem>>)> {
    let result: Vec<GetCosItem> = read_cos(storage.0, &store.state().await.root)
        .await?
        .into_iter()
        .map::<GetCosItem, _>(|i| match i {
            Ok(c) => GetCosItem::Ok(c),
            Err(e) => GetCosItem::Err {
                err: format!("{}", e),
            },
        })
        .collect();
    Ok((StatusCode::OK, Json(result)))
}

#[axum_macros::debug_handler]
async fn post_cos_start(
    Path(co_id): Path<String>,
    store: Extension<StoreType>,
    actions: Extension<ActionsType>,
) -> HttpResult<(StatusCode, Json<Value>)> {
    let actions = actions.deref().clone();

    // validate
    let state = store.state().await;
    let execute_state = state.execute.get(&co_id);
    match execute_state {
        Some(CoExecuteState::Running) => {
            return Ok((
                StatusCode::CONFLICT,
                json!({"message": "CO already running."}).into(),
            ));
        }
        Some(CoExecuteState::Stopping) => {
            return Ok((
                StatusCode::CONFLICT,
                json!({"message": "CO is currently stopping."}).into(),
            ));
        }
        Some(CoExecuteState::Starting) => {
            return Ok((
                StatusCode::CONFLICT,
                json!({"message": "CO already starting."}).into(),
            ));
        }
        Some(CoExecuteState::Stopped) | None => {}
    }

    // start and wait for running of stopped (failed)
    let action = CoAction::CoStartup { id: co_id.clone() };
    let (response, _) = join!(
        actions
            .filter_map(move |action| match action {
                CoAction::CoExecuteStateChanged {
                    id,
                    state: CoExecuteState::Running,
                } if id == co_id => {
                    Some(CoExecuteState::Running)
                }
                CoAction::CoExecuteStateChanged {
                    id,
                    state: CoExecuteState::Stopped,
                } if id == co_id => {
                    Some(CoExecuteState::Stopped)
                }
                _ => None,
            })
            .take(1)
            .to_future(),
        store.dispatch(action),
    );

    // response
    match response?? {
        CoExecuteState::Running => Ok((StatusCode::OK, json!("{}").into())),
        CoExecuteState::Stopped => Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"message": "CO startup failed."}).into(),
        )),
        _ => unreachable!("Invalid response state"),
    }
}

#[axum_macros::debug_handler]
async fn post_cos(
    store: Extension<StoreType>,
    actions: Extension<ActionsType>,
    Json(payload): Json<Value>,
) -> HttpResult<(StatusCode, Json<Value>)> {
    let actions = actions.deref().clone();

    // parse
    let body: CoCreate = serde_json::from_value(payload)?;

    // create
    let request = Request::new(body);
    let action = CoAction::CoCreate(request.clone());
    let (response, _) = join!(
        actions
            .filter_map(move |action| match action {
                CoAction::CoCreateResponse(response) => {
                    if response.reference == request.reference {
                        Some(response)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .take(1)
            .to_future(),
        store.dispatch(action),
    );

    // response
    match response??.response {
        Ok(i) => Ok((StatusCode::OK, Json(to_value(i)?))),
        Err(e) => Ok((
            e.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(to_value(e)?),
        )),
    }
}

async fn handler() -> axum::response::Json<VersionInfo> {
    axum::response::Json(VersionInfo {
        name: "co",
        version: "0.0.1",
        commit: "",
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionInfo {
    name: &'static str,
    version: &'static str,
    commit: &'static str,
}
