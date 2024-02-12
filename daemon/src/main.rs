use axum::{
	routing::{get, post},
	Extension, Router,
};
use clap::Parser;
use co_sdk::{local_keypair_fetch, ApplicationBuilder, Network};
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
	if cli.no_keychain {
		application_builder = application_builder.without_keychain();
	}
	let application = application_builder.build().await.expect("application");

	// local
	let local_co = application.local_co_reducer().await.expect("local-co");
	let local_identity = application.local_identity();

	// peer-id
	let network_key = local_keypair_fetch(&local_co, &local_identity, cli.force_new_peer_id)
		.await
		.expect("peer-id");

	// driver: network
	let _network = Network::new(network_key);

	// build routes
	let app = Router::new()
		.route("/", get(http::get))
		.route("/cos", get(http::cos::get).post(http::cos::post))
		.route("/cos/:id", post(http::co::post))
		.layer(Extension(local_co))
		.layer(Extension(application.storage()));

	// run it
	let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
	tracing::info! {addr = format!("http://{}/", addr), "listening"};
	let result: hyper::Result<()> = axum::Server::bind(&addr).serve(app.into_make_service()).await;
	result.unwrap();
}
