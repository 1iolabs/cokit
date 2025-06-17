use axum::{routing::get, Extension, Router};
use clap::Parser;
use co_sdk::{ApplicationBuilder, NetworkSettings};
use std::net::SocketAddr;

mod error;
mod http;
mod library;
mod service;
mod types;

#[tokio::main]
async fn main() {
	// cli
	let cli = library::cli::Cli::parse();

	// application
	let mut application_builder = match cli.base_path {
		None => ApplicationBuilder::new(cli.instance_id.to_owned()),
		Some(path) => ApplicationBuilder::new_with_path(cli.instance_id.to_owned(), path),
	};
	if !cli.no_log {
		application_builder = application_builder.with_bunyan_logging(cli.log_path);
	}
	if cli.no_keychain {
		application_builder = application_builder.without_keychain();
	}
	let mut application = application_builder.build().await.expect("application");

	// driver: network
	application
		.create_network(NetworkSettings { force_new_peer_id: cli.force_new_peer_id, ..Default::default() })
		.await
		.expect("network");

	// build routes
	let app = Router::new()
		.route("/", get(http::get))
		.route("/cos", get(http::cos::get).post(http::cos::post))
		.route("/cos/:id", get(http::co::get).post(http::co::post))
		.route("/cos/:id/cores", get(http::co_cores::get))
		.route("/cos/:id/cores/:core", get(http::co_core::get))
		.layer(Extension(application));

	// run it
	let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
	tracing::info! {addr = format!("http://{}/", addr), "listening"};
	let result: hyper::Result<()> = axum::Server::bind(&addr).serve(app.into_make_service()).await;
	result.unwrap();
}
