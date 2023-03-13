use crate::service::{create_co_from_json, PersistentState};
use anyhow::Result;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Extension;
use axum::{Json, Router};
use co_sdk::drivers::storage::iroh::{IrohConfig, IrohStorage};
use co_sdk::types::co::Co;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use service::read_cos;
use std::net::SocketAddr;
use std::sync::Arc;

mod entities;
mod error;
mod service;

#[tokio::main]
async fn main() {
    // drivers
    let config = IrohConfig {
        base_path: "/tmp/iroh".into(),
        tcp_port: None,
        quic_port: None,
    };
    let storage = Arc::new(IrohStorage::new(config).await.unwrap());
    let state = Arc::new(PersistentState::open("/tmp/co.json").await.unwrap());

    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/cos", get(get_cos).post(post_cos))
        .layer(Extension(storage))
        .layer(Extension(state));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on http://{}/", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn handle_error(result: Result<impl IntoResponse>) -> axum::response::Response {
    match result {
        Ok(r) => r.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("Something went wrong: {}", e) })),
        )
            .into_response(),
    }
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

async fn get_cos(
    storage: Extension<Arc<IrohStorage>>,
    state: Extension<Arc<PersistentState>>,
) -> axum::response::Response {
    handle_error(
        async {
            let result: Vec<GetCosItem> = read_cos(storage.0, &state.state().await.root)
                .await?
                .into_iter()
                .map::<GetCosItem, _>(|i| match i {
                    Ok(c) => GetCosItem::Ok(c),
                    Err(e) => GetCosItem::Err {
                        err: format!("{}", e),
                    },
                })
                .collect();
            Ok(Json(result))
        }
        .await,
    )
}

#[axum_macros::debug_handler]
async fn post_cos(
    storage: Extension<Arc<IrohStorage>>,
    state: Extension<Arc<PersistentState>>,
    Json(payload): Json<Value>,
) -> axum::response::Response {
    handle_error(
        create_co_from_json(storage.0, state.0, payload)
            .await
            .map(|id| Json(id.to_string())),
    )
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
