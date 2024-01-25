use crate::library::path;
use axum::{
	routing::{get, post},
	Extension, Router,
};
use clap::Parser;
use co_network::{Libp2pNetwork, Libp2pNetworkConfig};
use co_sdk::{ActionsType, CoState, CoStorage, State, StoreType};
use co_storage::FsStorage;
use libp2p::PeerId;
use std::{net::SocketAddr, sync::Arc};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, Registry};

mod error;
mod http;
mod library;
mod service;
mod types;

#[tokio::main]
async fn main() {
	// cli
	let cli = library::cli::Cli::parse();

	// paths
	let config_path = path::create_folder(path::config_path(&cli.base_path, &cli.config_path).unwrap())
		.await
		.expect("config_path");
	let storage_path = path::create_folder(path::storage_path(&cli.base_path, &cli.storage_path).unwrap())
		.await
		.expect("storage_path");
	let log_path = path::create_folder(path::log_path(&cli.base_path, &cli.log_path).unwrap())
		.await
		.expect("log_path");
	let data_path = path::create_folder(path::data_path(&cli.base_path, &cli.data_path).unwrap())
		.await
		.expect("data_path");

	// tracing
	let log_file = std::fs::File::create(log_path.join("daemon.log")).unwrap();
	// let formatting_layer = BunyanFormattingLayer::new("co-daemon".into(), std::io::stdout);
	let formatting_layer = BunyanFormattingLayer::new("co-daemon".into(), log_file);
	let subscriber = Registry::default()
		.with(LevelFilter::INFO)
		.with(JsonStorageLayer)
		.with(formatting_layer);
	tracing::subscriber::set_global_default(subscriber).unwrap();
	tracing_log::LogTracer::init().unwrap();

	// driver: storage
	let storage: CoStorage = Arc::new(FsStorage::new(storage_path));

	// driver: network
	let network_key = crate::library::local_key::local_key(Some(config_path.join("peer.pb")), cli.force_new_peer_id)
		.await
		.expect("peer-id");
	let network_peer_id = PeerId::from(network_key.public());
	let network_config = Libp2pNetworkConfig::from_keypair(network_key.clone());
	let network: Libp2pNetwork = Libp2pNetwork::new(network_config).await.expect("network");
	tracing::info!(peer_id = ?network_peer_id, "network");

	// driver: state
	let actions: ActionsType = ActionsType::default();
	let state = State::new(CoState::new(config_path, data_path), network, storage.clone(), actions.clone());
	let store: StoreType = state.store();

	// build routes
	let app = Router::new()
		.route("/", get(http::get))
		.route("/cos", get(http::cos::get).post(http::cos::post))
		.route("/cos/:id", post(http::co::post))
		.route("/state", get(http::state::get))
		.layer(Extension(storage))
		.layer(Extension(store))
		.layer(Extension(actions));

	// run it
	let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
	tracing::info! {addr = format!("http://{}/", addr), "listening"};
	let result: hyper::Result<()> = axum::Server::bind(&addr).serve(app.into_make_service()).await;
	result.unwrap();
}
