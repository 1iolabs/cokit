use anyhow::anyhow;
use axum::{
	routing::{get, post},
	Extension, Router,
};
use clap::Parser;
use co_core_keystore::Key;
use co_sdk::{
	keystore_fetch, local_keypair_fetch, Application, ApplicationBuilder, CoReducer, CoState, Network, State, Storage,
};
use libp2p::{identity::Keypair, PeerId};
use std::net::SocketAddr;

mod error;
mod http;
mod library;
mod service;
mod types;

const APP_IDENTIFIER: &str = "co-http";

#[tokio::main]
async fn main() {
	// cli
	let cli = library::cli::Cli::parse();

	// application
	let mut application_builder = match cli.base_path {
		None => ApplicationBuilder::new(APP_IDENTIFIER.to_owned()),
		Some(path) => ApplicationBuilder::new_with_path(APP_IDENTIFIER.to_owned(), path),
	};
	if cli.no_log == false {
		application_builder = application_builder.with_bunyan_logging(cli.log_path);
	}
	let application = application_builder.build().await.expect("application");

	// local
	let local_co = application.create_local_co().await.expect("local-co");

	// peer-id
	let network_key = local_keypair_fetch(&local_co).await.expect("peer-id");

	// driver: network
	let network = Network::new(network_key);

	// driver: state
	let state = State::new(CoState::new("".into(), "".into()), network.into_network(), application.storage());

	// build routes
	let app = Router::new()
		.route("/", get(http::get))
		.route("/cos", get(http::cos::get).post(http::cos::post))
		.route("/cos/:id", post(http::co::post))
		.route("/state", get(http::state::get))
		.layer(Extension(application.storage()))
		.layer(Extension(state.store()))
		.layer(Extension(state.actions()));

	// run it
	let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
	tracing::info! {addr = format!("http://{}/", addr), "listening"};
	let result: hyper::Result<()> = axum::Server::bind(&addr).serve(app.into_make_service()).await;
	result.unwrap();
}
