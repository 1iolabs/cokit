
use std::net::SocketAddr;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{Router, Json};
use axum::http::{StatusCode};
use axum::routing::{get};
use axum::Extension;
use co_sdk::drivers::storage::iroh::{IrohStorage, IrohConfig};
use co_sdk::entities::co::Co;
use serde::{Serialize, Deserialize};
use service::read_cos;
use serde_json::{Value, json};
use anyhow::Result;

use crate::service::{create_co_from_json, PersistentState};


mod service;
mod error;
mod entities;


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
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": format!("Something went wrong: {}", e)}))
            )
                .into_response()
        },
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

async fn get_cos(storage: Extension<Arc<IrohStorage>>, state: Extension<Arc<PersistentState>>) -> axum::response::Response {
    handle_error(async {
        let result: Vec<GetCosItem> = read_cos(storage.0, &state.state().await.root)
            .await?
            .into_iter()
            .map::<GetCosItem, _>(|i| match i {
                Ok(c) => GetCosItem::Ok(c),
                Err(e) => GetCosItem::Err{err: format!("{}", e)},
            })
            .collect();
        Ok(Json(result))
    }.await)
}

#[axum_macros::debug_handler]
async fn post_cos(storage: Extension<Arc<IrohStorage>>, state: Extension<Arc<PersistentState>>, Json(payload): Json<Value>) -> axum::response::Response {
    handle_error(
        create_co_from_json(storage.0, state.0, payload).await
            .map(|id| { Json(id.to_string()) })
    )
}

async fn handler() -> axum::response::Json<VersionInfo> {
    axum::response::Json(
        VersionInfo {
            name: "co",
            version: "0.0.1",
            commit: "",
        }
    )
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionInfo {
    name: &'static str,
    version: &'static str,
    commit: &'static str,
}
