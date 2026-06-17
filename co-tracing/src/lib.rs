// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

//! Opinionated COKIT logging/tracing setup.

mod sink;
pub use sink::{parse_log, resolve_filter, LogConfig, LogSink, SinkSpec};

#[cfg(feature = "subscriber")]
mod builder;
#[cfg(feature = "subscriber")]
pub use builder::{env_filter, level_from_verbosity, TracingBuilder, TracingGuard};

#[cfg(feature = "opentelemetry")]
mod builder_open_telemetry;

#[cfg(all(feature = "clap", feature = "subscriber"))]
mod args;
#[cfg(all(feature = "clap", feature = "subscriber"))]
pub use args::{LogArgs, LogContext};
