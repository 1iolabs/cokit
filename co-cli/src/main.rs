// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use clap::Parser;

mod cli;
mod commands;
mod library;

fn main() {
	let result = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async { app_main().await });
	std::process::exit(result.unwrap());
}

async fn app_main() -> anyhow::Result<exitcode::ExitCode> {
	let cli = cli::Cli::parse();

	// tracing
	let _guard = cli.log.init(&co_tracing::LogContext {
		identifier: &cli.instance_id,
		base_path: cli.base_path.as_deref(),
		default_stderr: Some(tracing::Level::INFO),
	})?;

	// execute
	cli::command(&cli).await
}
