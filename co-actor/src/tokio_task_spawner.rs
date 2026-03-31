// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::TaskOptions;
use futures::Future;
use std::{panic::Location, sync::Arc};
use tokio::task::JoinHandle;
use tokio_util::task::TaskTracker;
use tracing::Instrument;

pub type TaskHandle<T> = JoinHandle<T>;

#[derive(Debug, Clone)]
pub struct TaskSpawner {
	pub(crate) idenitfier: Arc<String>,
	pub(crate) inner: TaskTracker,
}
impl TaskSpawner {
	pub fn new(idenitfier: String) -> Self {
		Self { idenitfier: Arc::new(idenitfier), inner: TaskTracker::new() }
	}

	/// Spawn task.
	#[inline]
	#[track_caller]
	pub fn spawn<F>(&self, task: F) -> TaskHandle<F::Output>
	where
		F: Future + Send + 'static,
		F::Output: Send + 'static,
	{
		let caller_file = Location::caller().file();
		let caller_line = Location::caller().line();
		let caller_column = Location::caller().column();
		let span = tracing::trace_span!(
			"task",
			application = self.idenitfier.as_str(),
			caller_file,
			caller_line,
			caller_column,
		);
		self.inner.spawn(task.instrument(span))
	}

	/// Spawn task.
	#[inline]
	#[track_caller]
	#[allow(unexpected_cfgs)]
	pub fn spawn_named<F>(&self, name: &str, task: F) -> TaskHandle<F::Output>
	where
		F: Future + Send + 'static,
		F::Output: Send + 'static,
	{
		let caller_file = Location::caller().file();
		let caller_line = Location::caller().line();
		let caller_column = Location::caller().column();
		let span = tracing::trace_span!(
			"task",
			task_name = name,
			application = self.idenitfier.as_str(),
			caller_file,
			caller_line,
			caller_column,
		);
		#[cfg(tokio_unstable)]
		{
			tokio::task::Builder::new()
				.name(name)
				.spawn(self.inner.track_future(task.instrument(span)))
				.expect("tokio runtime")
		}
		#[cfg(not(tokio_unstable))]
		{
			self.inner.spawn(task.instrument(span))
		}
	}

	/// Spawn task.
	#[inline]
	#[track_caller]
	#[allow(unexpected_cfgs)]
	pub fn spawn_options<F>(&self, options: TaskOptions, task: F) -> TaskHandle<F::Output>
	where
		F: Future + Send + 'static,
		F::Output: Send + 'static,
	{
		let caller_file = Location::caller().file();
		let caller_line = Location::caller().line();
		let caller_column = Location::caller().column();
		let span = tracing::trace_span!(
			"task",
			task_name = options.name,
			application = self.idenitfier.as_str(),
			caller_file,
			caller_line,
			caller_column,
		);
		#[cfg(tokio_unstable)]
		{
			let mut builder = tokio::task::Builder::new();
			if let Some(name) = options.name {
				builder = builder.name(name);
			}
			builder
				.spawn(if options.untracked {
					futures::future::Either::Left(task.instrument(span))
				} else {
					futures::future::Either::Right(self.inner.track_future(task.instrument(span)))
				})
				.expect("tokio runtime")
		}
		#[cfg(not(tokio_unstable))]
		if options.untracked {
			tokio::spawn(task.instrument(span))
		} else {
			self.inner.spawn(task.instrument(span))
		}
	}

	/// Spawn blocking task.
	#[inline]
	#[track_caller]
	#[allow(unexpected_cfgs)]
	pub fn spawn_blocking<F, R>(&self, options: TaskOptions, task: F) -> TaskHandle<R>
	where
		F: FnOnce() -> R + Send + 'static,
		R: Send + 'static,
	{
		let caller_file = Location::caller().file();
		let caller_line = Location::caller().line();
		let caller_column = Location::caller().column();
		let span = tracing::trace_span!(
			"task-blocking",
			task_name = options.name,
			application = self.idenitfier.as_str(),
			caller_file,
			caller_line,
			caller_column,
		);
		let task = move || {
			let _span_guard = span.enter();
			task()
		};
		#[cfg(tokio_unstable)]
		{
			let mut builder = tokio::task::Builder::new();
			if let Some(name) = options.name {
				builder = builder.name(name);
			}
			builder
				.spawn_blocking(if options.untracked {
					futures::future::Either::Left(task)
				} else {
					futures::future::Either::Right(self.inner.track_future(task))
				})
				.expect("tokio runtime")
		}
		#[cfg(not(tokio_unstable))]
		if options.untracked {
			tokio::task::spawn_blocking(task)
		} else {
			self.inner.spawn_blocking(task)
		}
	}

	pub fn tracker(&self) -> TaskTracker {
		self.inner.clone()
	}
}
impl Default for TaskSpawner {
	fn default() -> Self {
		Self { idenitfier: Arc::new("default".to_string()), inner: Default::default() }
	}
}
