// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::fs_write::fs_write_atomic;
use crate::library::locals::{ApplicationLocal, Locals};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, Response, ResponseStream, ResponseStreams, TaskHandle, TaskSpawner};
use co_primitives::{tags, to_cbor, Tags};
use futures::{pin_mut, stream, Stream, StreamExt, TryStreamExt};
use libc::flock;
use nix::fcntl::{fcntl, FcntlArg, Flock, Flockable};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use pin_project::{pin_project, pinned_drop};
use std::{
	collections::BTreeMap,
	fmt::Debug,
	future::ready,
	io::ErrorKind,
	os::fd::AsRawFd,
	path::{Path, PathBuf},
	pin::Pin,
	task::{Context, Poll},
};
use tokio::fs::File;
use tokio_util::sync::{CancellationToken, DropGuard};

#[derive(Debug, Clone)]
pub struct FileLocals {
	handle: ActorHandle<FileLocalsMessage>,
}
impl FileLocals {
	/// Create locals by reading all local configurations.
	///
	/// # Arguments
	/// * `config_path` - The local configuratin path. Normally `{base_path}/etc`.
	pub fn new(
		tasks: TaskSpawner,
		config_path: PathBuf,
		identifier: String,
		lock: bool,
	) -> Result<Self, anyhow::Error> {
		let instance = Actor::spawn(
			tags!("type": "file-locals", "application": &identifier),
			FileLocalsActor { config_path, identifier, lock: if lock { Lock::Fcntl } else { Lock::None } },
			tasks,
		)?;
		Ok(Self { handle: instance.handle() })
	}

	/// Read the local co state from disk.
	/// All folders below `config_path` are checked.
	fn read(config_path: PathBuf) -> impl Stream<Item = Result<(PathBuf, ApplicationLocal), anyhow::Error>> {
		async_stream::try_stream! {
			// read applications
			let mut dir = match tokio::fs::read_dir(&config_path).await {
				Err(e) if e.kind() == ErrorKind::NotFound => {
					// create
					tokio::fs::create_dir_all(&config_path).await?;

					// retry
					tokio::fs::read_dir(&config_path).await
				},
				i => i,
			}?;
			while let Some(child) = dir.next_entry().await? {
				// skip non directories
				if !child.file_type().await?.is_dir() {
					continue;
				}

				// try to read local.cbor
				let local_path = child.path().join("local.cbor");
				let local = match ApplicationLocal::read(&local_path).await {
					Ok(local) => local,
					Err(err) => {
						// log and ignore
						//  a single unreadable/foreign file must not abort the whole read
						tracing::warn!(?local_path, ?err, "locals-read-failed");
						continue;
					},
				};
				if let Some(local) = local {
					yield (local_path, local);
				}
			}
		}
	}
}
#[async_trait]
impl Locals for FileLocals {
	/// Read all available local.cbor files
	async fn get(&self) -> Result<Vec<ApplicationLocal>, anyhow::Error> {
		Ok(self.handle.request(FileLocalsMessage::ReadAll).await??)
	}

	async fn set(&mut self, local: ApplicationLocal) -> Result<(), anyhow::Error> {
		Ok(self
			.handle
			.request(|response| FileLocalsMessage::Write(local, response))
			.await??)
	}

	fn watch(&self) -> impl Stream<Item = ApplicationLocal> + Send + Sync + 'static {
		// start
		self.handle.dispatch(FileLocalsMessage::WatchStart).ok();

		// watch
		DropStream::new(
			self.handle
				.stream(FileLocalsMessage::Watch)
				.filter_map(|item| ready(item.ok()))
				.map(|item| item.1),
			{
				let handle = self.handle.clone();
				move || {
					handle.dispatch(FileLocalsMessage::WatchEnd).ok();
				}
			},
		)
	}
}

#[pin_project(PinnedDrop)]
struct DropStream<T, D>(#[pin] T, Option<D>)
where
	T: Stream,
	D: FnOnce();
impl<T, D> DropStream<T, D>
where
	T: Stream,
	D: FnOnce(),
{
	pub fn new(stream: T, on_drop: D) -> Self {
		Self(stream, Some(on_drop))
	}
}
impl<T, D> Stream for DropStream<T, D>
where
	T: Stream,
	D: FnOnce(),
{
	type Item = T::Item;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.project().0.poll_next(cx)
	}
}
#[pinned_drop]
impl<T, D> PinnedDrop for DropStream<T, D>
where
	T: Stream,
	D: FnOnce(),
{
	fn drop(self: Pin<&mut Self>) {
		if let Some(on_drop) = self.project().1.take() {
			on_drop();
		}
	}
}

#[derive(Debug)]
enum Lock {
	None,
	Fcntl,
	_Flock,
}

#[derive(Debug)]
struct FileLocalsActor {
	config_path: PathBuf,
	identifier: String,
	lock: Lock,
}
#[async_trait]
impl Actor for FileLocalsActor {
	type Message = FileLocalsMessage;
	type State = FileLocalsState;
	type Initialize = TaskSpawner;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(FileLocalsState::new(initialize))
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			FileLocalsMessage::Write(local, response) => {
				response
					.execute(|| async {
						// open and lock file
						if state.file.is_none() {
							state.file = match self.lock {
								Lock::_Flock => self.open_and_flock().await?,
								Lock::Fcntl => self.open_and_lock().await?,
								Lock::None => self.open().await?,
							};
						}

						// write
						state.write(local).await?;

						// result
						Ok(())
					})
					.await
					.ok();
			},
			FileLocalsMessage::ReadAll(response) => {
				response
					.execute(|| async {
						// read
						state.read(self.config_path.clone()).await?;

						// result
						Ok(state.locals.values().cloned().collect())
					})
					.await
					.ok();
			},
			FileLocalsMessage::Watch(response) => {
				state.watchers.push(response);
			},
			FileLocalsMessage::WatchStart => {
				if state.watch.is_none() {
					// update
					state.read(self.config_path.clone()).await?;

					// watch
					let cancel = CancellationToken::new();
					let stream = watch(state.tasks.clone(), self.config_path.clone())?
						.take_until(cancel.clone().cancelled_owned());
					state.watch = Some((
						cancel.drop_guard(),
						state.tasks.spawn({
							let handle = handle.clone();
							async move {
								pin_mut!(stream);
								while let Some(path) = stream.next().await {
									handle.dispatch(FileLocalsMessage::Update(path, None)).ok();
								}
							}
						}),
					));
				}
			},
			FileLocalsMessage::WatchEnd => {
				state.watch = None;
			},
			FileLocalsMessage::Update(path, next) => {
				let next = match next {
					Some(next) => Some(next),
					None => match ApplicationLocal::read(&path).await {
						Ok(next) => next,
						Err(err) => {
							tracing::warn!(?path, ?err, "locals-read-failed");
							None
						},
					},
				};
				if let Some(next) = next {
					state.update(path, next);
				}
			},
		}
		Ok(())
	}
}
impl FileLocalsActor {
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug))]
	async fn open(&self) -> Result<FileLocalsFile, anyhow::Error> {
		// the data file + parent dir are created lazily by the first atomic write
		let path = self.config_path.join(&self.identifier).join("local.cbor");
		Ok(FileLocalsFile::Unlocked(path))
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug))]
	async fn open_and_lock(&self) -> Result<FileLocalsFile, anyhow::Error> {
		let mut dir = self.config_path.join(&self.identifier);

		// find a slot whose lock sidecar is free
		let mut index = 1;
		loop {
			// create slot dir
			tokio::fs::create_dir_all(&dir).await?;

			// open lock sidecar (never renamed, stable inode keeps the lock valid)
			let lock_path = dir.join("local.cbor.lock");
			let file = tokio::fs::OpenOptions::new()
				.read(true)
				.write(true)
				.create(true)
				.truncate(false)
				.open(&lock_path)
				.await?;

			// lock
			let lock = flock { l_start: 0, l_len: 0, l_pid: 0, l_type: libc::F_WRLCK as libc::c_short, l_whence: 0 };
			// F_SETLK is non-blocking: if another process holds the slot, fail fast and try the next index
			match fcntl(file.as_raw_fd(), FcntlArg::F_SETLK(&lock)) {
				Ok(_) => {
					let path = dir.join("local.cbor");
					tracing::info!(?path, "locals-lock");
					return Ok(FileLocalsFile::Locked(path, file));
				},
				Err(errno) => {
					// close file
					// note: this should not drop any locks as we expect we only have one local.cbor per process!
					drop(file);

					// log
					tracing::warn!(?lock_path, ?errno, "locals-lock-failed");

					// index
					dir = self.config_path.join(format!("{}-{}", self.identifier, index));
					index += 1;
				},
			}
		}
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug))]
	async fn open_and_flock(&self) -> Result<FileLocalsFile, anyhow::Error> {
		let mut dir = self.config_path.join(&self.identifier);

		// find a slot whose lock sidecar is free
		let mut index = 1;
		loop {
			// create slot dir
			tokio::fs::create_dir_all(&dir).await?;

			// open lock sidecar (never renamed, stable inode keeps the lock valid)
			let lock_path = dir.join("local.cbor.lock");
			let file = TokioFile(
				tokio::fs::OpenOptions::new()
					.read(true)
					.write(true)
					.create(true)
					.truncate(false)
					.open(&lock_path)
					.await?,
			);

			// lock
			match Flock::lock(file, nix::fcntl::FlockArg::LockExclusiveNonblock) {
				Ok(lock) => {
					let path = dir.join("local.cbor");
					tracing::info!(?path, "locals-lock (flock)");
					return Ok(FileLocalsFile::Flock(path, lock));
				},
				Err((file, errno)) => {
					// close file
					// note: this should not drop any locks as we expect we only have one local.cbor per process!
					drop(file);

					// log
					tracing::warn!(?lock_path, ?errno, "locals-lock-failed");

					// index
					dir = self.config_path.join(format!("{}-{}", self.identifier, index));
					index += 1;
				},
			}
		}
	}
}

#[derive(Debug)]
enum FileLocalsMessage {
	/// Write local.
	Write(ApplicationLocal, Response<Result<(), anyhow::Error>>),

	/// Read all locals.
	ReadAll(Response<Result<Vec<ApplicationLocal>, anyhow::Error>>),

	/// Update locals.
	Update(PathBuf, Option<ApplicationLocal>),

	/// Watch locals.
	Watch(ResponseStream<(PathBuf, ApplicationLocal)>),

	/// Start watcher.
	WatchStart,

	/// End watcher.
	WatchEnd,
}

#[derive(Debug, Default)]
#[allow(dead_code)] // second field holds the lock alive (fcntl fd / flock guard) and releases it on drop; never read directly
enum FileLocalsFile {
	#[default]
	None,
	/// Data path only. No lock held (`Lock::None`).
	Unlocked(PathBuf),
	/// Data path + `flock` guard held on the `.lock` sidecar.
	Flock(PathBuf, Flock<TokioFile>),
	/// Data path + `fcntl`-locked fd held on the `.lock` sidecar.
	Locked(PathBuf, tokio::fs::File),
}
impl FileLocalsFile {
	/// The `local.cbor` data path for this slot, if a slot has been opened.
	fn path(&self) -> Option<&PathBuf> {
		match self {
			Self::None => None,
			Self::Unlocked(path) | Self::Flock(path, _) | Self::Locked(path, _) => Some(path),
		}
	}

	fn is_none(&self) -> bool {
		matches!(self, Self::None)
	}
}

#[derive(Debug, Default)]
struct FileLocalsState {
	tasks: TaskSpawner,

	/// Loaded locals.
	locals: BTreeMap<PathBuf, ApplicationLocal>,

	/// Our local.cbor, if already written to, locked.
	file: FileLocalsFile,

	/// Active watchers.
	watchers: ResponseStreams<(PathBuf, ApplicationLocal)>,
	watch: Option<(DropGuard, TaskHandle<()>)>,
}
impl FileLocalsState {
	fn new(tasks: TaskSpawner) -> Self {
		Self { tasks, ..Default::default() }
	}

	/// Apply `next` to current state.
	fn update(&mut self, path: PathBuf, next: ApplicationLocal) {
		if match self.locals.get(&path) {
			Some(current) => current.heads != next.heads,
			None => true,
		} {
			// apply
			self.locals.insert(path.clone(), next.clone());

			// notify
			self.watchers.send((path, next));
		}
	}

	/// Write local atomically so concurrent readers never see a torn file.
	async fn write(&mut self, local: ApplicationLocal) -> Result<(), anyhow::Error> {
		// get path
		//  note: the lock guard is held by self.file for the actor's lifetime
		let path = self.file.path().ok_or(anyhow!("No file."))?.to_owned();

		// apply
		self.locals.insert(path.clone(), local.clone());

		// serialize
		let data = to_cbor(&local)?;

		// log
		tracing::debug!(?path, ?local, "locals-write");

		// atomic write
		fs_write_atomic(&path, &data, true).await?;

		// result
		Ok(())
	}

	/// Read locals.
	async fn read(&mut self, config_path: PathBuf) -> Result<(), anyhow::Error> {
		let locals = FileLocals::read(config_path);
		pin_mut!(locals);
		while let Some((path, local)) = locals.try_next().await? {
			self.update(path, local);
		}
		Ok(())
	}
}

/// Extract the `local.cbor` paths to re-read from a watch event.
///
/// Watch backends disagree on how an atomic write (temp file + rename-into-place, see
/// [`fs_write_atomic`]) surfaces, so we cannot key purely on `Create`/`Modify` of the target path:
/// - inotify (Linux) and FSEvents report `Create`/`Modify` directly on the `local.cbor` path.
/// - kqueue (macOS) reports a `Remove` of the replaced `local.cbor` plus a `Modify` of the containing slot directory,
///   and *never* a `Create`/`Modify` of the `local.cbor` path itself.
///
/// To cover every backend we map an event to a `local.cbor` re-read when it touches either the
/// `local.cbor` file directly or the slot directory (`<config_path>/<slot>`) that holds it.
/// A re-read is idempotent — [`FileLocalsState::update`] deduplicates by heads — so re-reading on a
/// spurious or coalesced event is harmless.
///
/// # Arguments
/// - `event`: Any `Create`/`Modify`/`Remove` events.
fn local_event_paths(event: &Event, config_path: &Path) -> Vec<PathBuf> {
	if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)) {
		return Vec::new();
	}
	event
		.paths
		.iter()
		.filter_map(|path| {
			// event on `<config_path>/<slot>/local.cbor` (inotify / FSEvents, and kqueue's remove)
			if path.file_name().and_then(|f| f.to_str()) == Some("local.cbor")
				&& path.parent().and_then(|f| f.parent()) == Some(config_path)
			{
				return Some(path.clone());
			}
			// event on the `<config_path>/<slot>` directory itself (kqueue's rename signal)
			if path.parent() == Some(config_path) {
				return Some(path.join("local.cbor"));
			}
			None
		})
		.collect()
}

/// Watch for all local.cbor changes in config_path.
fn watch(tasks: TaskSpawner, config_path: PathBuf) -> Result<impl Stream<Item = PathBuf>, anyhow::Error> {
	let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<notify::Event, notify::Error>>();

	// watcher
	let mut watcher = notify::recommended_watcher({
		let tx = tx.clone();
		move |event| {
			tx.send(event).ok();
		}
	})?;
	watcher.watch(&config_path, RecursiveMode::Recursive)?;

	// shutdown
	tasks.spawn({
		let config_path = config_path.clone();
		async move {
			// wait reader is dropped
			tx.closed().await;

			// unwatch
			watcher.unwatch(&config_path).ok();
		}
	});

	// stream
	let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
		.filter_map({
			let config_path = config_path.clone();
			move |result| {
				ready(match result {
					Ok(event) => Some(event),
					Err(err) => {
						tracing::warn!(?err, ?config_path, "locals-watch-error");
						None
					},
				})
			}
		})
		.filter_map(move |event| ready(Some(local_event_paths(&event, &config_path))))
		.flat_map(|paths: Vec<PathBuf>| stream::iter(paths));

	// result
	Ok(stream)
}

#[derive(Debug)]
struct TokioFile(pub File);
impl AsRawFd for TokioFile {
	fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
		self.0.as_raw_fd()
	}
}
unsafe impl Flockable for TokioFile {}

#[cfg(test)]
mod tests {
	use crate::library::{
		locals::{ApplicationLocal, Locals},
		locals_file::FileLocals,
	};
	use co_primitives::BlockSerializer;
	use co_test::TmpDir;
	use std::{
		path::PathBuf,
		sync::{
			atomic::{AtomicBool, Ordering},
			Arc,
		},
	};

	#[tokio::test]
	async fn test_file_locals_overwrite() {
		// tracing_subscriber::fmt()
		// 	.with_env_filter(tracing_subscriber::EnvFilter::new(format!(
		// 		"{}=trace",
		// 		module_path!().split(":").next().expect("module path")
		// 	)))
		// 	.try_init()
		// 	.ok();

		let tmp = TmpDir::new("co");

		// read
		let mut locals = FileLocals::new(Default::default(), tmp.path().into(), "test".to_owned(), true).unwrap();
		let items = locals.get().await.unwrap();
		assert_eq!(items.len(), 0);

		// write
		let v1 = BlockSerializer::default().serialize(&1).unwrap();
		locals
			.set(ApplicationLocal::new([*v1.cid()].into(), *v1.cid(), None))
			.await
			.unwrap();

		// read
		let items = locals.get().await.unwrap();
		assert_eq!(items.len(), 1);
		assert_eq!(&items.first().unwrap().state, v1.cid());

		// write
		let v2 = BlockSerializer::default().serialize(&2).unwrap();
		locals
			.set(ApplicationLocal::new([*v2.cid()].into(), *v2.cid(), None))
			.await
			.unwrap();

		// read
		let items = locals.get().await.unwrap();
		assert_eq!(items.len(), 1);
		assert_eq!(&items.first().unwrap().state, v2.cid());
	}

	#[tokio::test]
	async fn test_file_locals_uses_sidecar_lock() {
		let tmp = TmpDir::new("co");
		let mut locals = FileLocals::new(Default::default(), tmp.path().into(), "test".to_owned(), true).unwrap();

		let v = BlockSerializer::default().serialize(&1).unwrap();
		locals
			.set(ApplicationLocal::new([*v.cid()].into(), *v.cid(), None))
			.await
			.unwrap();

		let dir = tmp.path().join("test");
		assert!(dir.join("local.cbor").exists(), "data file should exist");
		assert!(dir.join("local.cbor.lock").exists(), "sidecar lock file should exist");
		assert!(!dir.join("local.cbor.tmp").exists(), "no temp file should remain");
	}

	#[tokio::test]
	async fn test_read_all_skips_unreadable_local() {
		let tmp = TmpDir::new("co");
		let config: PathBuf = tmp.path().into();

		// a valid local under "good"
		let mut good = FileLocals::new(Default::default(), config.clone(), "good".to_owned(), true).unwrap();
		let v = BlockSerializer::default().serialize(&42u64).unwrap();
		good.set(ApplicationLocal::new([*v.cid()].into(), *v.cid(), None))
			.await
			.unwrap();

		// a corrupt local under "bad" (empty file == the mid-truncation state that triggers CBOR Eof)
		let bad_dir = config.join("bad");
		tokio::fs::create_dir_all(&bad_dir).await.unwrap();
		tokio::fs::write(bad_dir.join("local.cbor"), b"").await.unwrap();

		// ReadAll returns the good local and skips the bad one (no error)
		let reader = FileLocals::new(Default::default(), config.clone(), "reader".to_owned(), true).unwrap();
		let items = reader.get().await.unwrap();
		assert_eq!(items.len(), 1);
		assert_eq!(&items.first().unwrap().state, v.cid());
	}

	#[test]
	fn test_local_event_paths_matches_create_modify_and_rename() {
		use notify::{
			event::{AccessKind, CreateKind, DataChange, ModifyKind, RenameMode},
			Event, EventKind,
		};

		let config = PathBuf::from("/cfg");
		let local = PathBuf::from("/cfg/app/local.cbor");
		let tmp = PathBuf::from("/cfg/app/local.cbor.tmp");
		let lock = PathBuf::from("/cfg/app/local.cbor.lock");

		// rename-into-place (atomic write) - the case the old filter missed
		let ev = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::To))).add_path(local.clone());
		assert_eq!(super::local_event_paths(&ev, &config), vec![local.clone()]);

		// in-place data modify still matches
		let ev = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Any))).add_path(local.clone());
		assert_eq!(super::local_event_paths(&ev, &config), vec![local.clone()]);

		// create still matches
		let ev = Event::new(EventKind::Create(CreateKind::File)).add_path(local.clone());
		assert_eq!(super::local_event_paths(&ev, &config), vec![local.clone()]);

		// `.tmp` / `.lock` siblings are ignored (filename gate)
		let ev = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::To)))
			.add_path(tmp)
			.add_path(lock);
		assert!(super::local_event_paths(&ev, &config).is_empty());

		// unrelated event kinds are ignored
		let ev = Event::new(EventKind::Access(AccessKind::Read)).add_path(local);
		assert!(super::local_event_paths(&ev, &config).is_empty());
	}

	/// The exact event sequence the macOS `kqueue` backend delivers for an atomic write
	/// (temp file + rename-into-place): a `Create` of the temp sibling, a `Modify` of the slot
	/// directory, and a `Remove` of the replaced `local.cbor` - never a `Create`/`Modify` of the
	/// `local.cbor` path itself. At least one event must map to a `local.cbor` re-read.
	#[test]
	fn test_local_event_paths_matches_kqueue_atomic_rename() {
		use notify::{
			event::{CreateKind, DataChange, ModifyKind, RemoveKind},
			Event, EventKind,
		};

		let config = PathBuf::from("/cfg");
		let slot = PathBuf::from("/cfg/app");
		let local = PathBuf::from("/cfg/app/local.cbor");
		let tmp = PathBuf::from("/cfg/app/.tmp0wJ1Zw");

		// temp-file create in the slot dir must NOT trigger a re-read
		let ev = Event::new(EventKind::Create(CreateKind::File)).add_path(tmp);
		assert!(super::local_event_paths(&ev, &config).is_empty());

		// modify of the slot directory itself maps to that slot's `local.cbor`
		let ev = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Any))).add_path(slot);
		assert_eq!(super::local_event_paths(&ev, &config), vec![local.clone()]);

		// remove of the replaced `local.cbor` (fired after the rename completes) maps to a re-read
		let ev = Event::new(EventKind::Remove(RemoveKind::Any)).add_path(local.clone());
		assert_eq!(super::local_event_paths(&ev, &config), vec![local]);
	}

	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
	async fn test_file_locals_concurrent_read_during_write() {
		let tmp = TmpDir::new("co");
		let config: PathBuf = tmp.path().into();

		// writer instance; seed an initial value so `local.cbor` exists and is complete
		let mut writer = FileLocals::new(Default::default(), config.clone(), "writer".to_owned(), true).unwrap();
		let v0 = BlockSerializer::default().serialize(&0u64).unwrap();
		writer
			.set(ApplicationLocal::new([*v0.cid()].into(), *v0.cid(), None))
			.await
			.unwrap();

		// reader instance over the same config dir (reads `writer/local.cbor` via ReadAll)
		let reader = FileLocals::new(Default::default(), config.clone(), "reader".to_owned(), true).unwrap();

		let stop = Arc::new(AtomicBool::new(false));

		// writer task: hammer atomic writes
		let writer_task = tokio::spawn({
			let stop = stop.clone();
			async move {
				for i in 1..=5000u64 {
					let v = BlockSerializer::default().serialize(&i).unwrap();
					writer
						.set(ApplicationLocal::new([*v.cid()].into(), *v.cid(), None))
						.await
						.unwrap();
				}
				stop.store(true, Ordering::Release);
			}
		});

		// reader task: read continuously while the writer runs; must never error.
		// the meaningful assertion is the `.unwrap()` on each read (it panics on a torn file);
		// the final `assert!(reads > 0)` is just a sanity check that the loop body ran at all.
		let reader_task = tokio::spawn(async move {
			let mut reads = 0u64;
			while !stop.load(Ordering::Acquire) {
				// before the fix this intermittently panics with: CBOR ... Eof
				reader.get().await.unwrap();
				reads += 1;
			}
			reads
		});

		writer_task.await.unwrap();
		let reads = reader_task.await.unwrap();
		assert!(reads > 0, "reader should have completed at least one read");
	}
}
