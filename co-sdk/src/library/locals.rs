use super::fs_read::fs_read_option;
use crate::CoReducerState;
use anyhow::{anyhow, Context as _};
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, Response, ResponseStream, ResponseStreams};
use co_primitives::{from_cbor, tags, to_cbor, Tags};
use futures::{pin_mut, stream, Stream, StreamExt, TryStreamExt};
use libc::flock;
use nix::fcntl::{fcntl, FcntlArg, Flock, Flockable};
use notify::{
	event::{CreateKind, ModifyKind},
	RecursiveMode, Watcher,
};
use pin_project::{pin_project, pinned_drop};
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	future::ready,
	io::ErrorKind,
	ops::DerefMut,
	os::fd::AsRawFd,
	path::PathBuf,
	pin::Pin,
	task::{Context, Poll},
};
use tokio::{
	fs::File,
	io::{AsyncSeekExt, AsyncWriteExt},
	task::JoinHandle,
};
use tokio_util::sync::{CancellationToken, DropGuard};

#[async_trait]
pub trait Locals: Clone + Debug + Send + Sync {
	/// Get current ApplicationLocal instances.
	async fn get(&self) -> Result<Vec<ApplicationLocal>, anyhow::Error>;

	/// Watch ApplicationLocal instances after last get.
	fn watch(&self) -> impl Stream<Item = ApplicationLocal> + Send + Sync + 'static;

	/// Set ApplicationLocal for our instance.
	async fn set(&mut self, local: ApplicationLocal) -> Result<(), anyhow::Error>;
}

#[derive(Debug, Clone)]
pub struct MemoryLocals {
	watcher: tokio::sync::watch::Sender<Option<ApplicationLocal>>,
}
impl MemoryLocals {
	pub fn new(initial: Option<ApplicationLocal>) -> Self {
		Self { watcher: tokio::sync::watch::channel(initial).0 }
	}
}
#[async_trait]
impl Locals for MemoryLocals {
	async fn get(&self) -> Result<Vec<ApplicationLocal>, anyhow::Error> {
		Ok(match self.watcher.borrow().as_ref() {
			Some(local) => vec![local.clone()],
			None => Default::default(),
		})
	}

	async fn set(&mut self, local: ApplicationLocal) -> Result<(), anyhow::Error> {
		self.watcher.send_replace(Some(local));
		Ok(())
	}

	fn watch(&self) -> impl Stream<Item = ApplicationLocal> + Send + Sync + 'static {
		// tokio_stream::wrappers::WatchStream::new(self.watcher.subscribe()).filter_map(|item| ready(item))
		// as we only ever have our local state it can not changed from outside
		stream::empty()
	}
}

#[derive(Debug, Clone)]
pub struct FileLocals {
	handle: ActorHandle<FileLocalsMessage>,
}
impl FileLocals {
	/// Create locals by reading all local configurations.
	///
	/// # Arguments
	/// * `config_path` - The local configuratin path. Normally `{base_path}/etc`.
	pub fn new(config_path: PathBuf, identifier: String, lock: bool) -> Result<Self, anyhow::Error> {
		let instance = Actor::spawn(
			tags!("type": "file-locals", "application": &identifier),
			FileLocalsActor { config_path, identifier, lock: if lock { Lock::Fcntl } else { Lock::None } },
			(),
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
				let local = ApplicationLocal::read(&local_path).await?;
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
	type Initialize = ();

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		_initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(FileLocalsState::default())
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
					let stream = watch(self.config_path.clone())?.take_until(cancel.clone().cancelled_owned());
					state.watch = Some((
						cancel.drop_guard(),
						tokio::spawn({
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
		let path = self.config_path.join(&self.identifier).join("local.cbor");

		// create parent dir
		tokio::fs::create_dir_all(path.parent().ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?).await?;

		// open
		let file = tokio::fs::OpenOptions::new()
			.create(true)
			.truncate(false)
			.write(true)
			.open(&path)
			.await?;

		// result
		Ok(FileLocalsFile::File(path, file))
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug))]
	async fn open_and_lock(&self) -> Result<FileLocalsFile, anyhow::Error> {
		let mut path = self.config_path.join(&self.identifier).join("local.cbor");

		// create and lock
		let mut index = 1;
		loop {
			// create parent dir
			tokio::fs::create_dir_all(path.parent().ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?).await?;

			// open
			let file = tokio::fs::OpenOptions::new()
				.read(true)
				.write(true)
				.create(true)
				.truncate(false)
				.open(&path)
				.await?;

			// lock
			let lock = flock { l_start: 0, l_len: 0, l_pid: 0, l_type: libc::F_WRLCK as libc::c_short, l_whence: 0 };
			match fcntl(file.as_raw_fd(), FcntlArg::F_SETLK(&lock)) {
				Ok(_) => {
					tracing::info!(?path, "locals-lock");
					return Ok(FileLocalsFile::LockedFile(path, file));
				},
				Err(errno) => {
					// close file
					// note: this should not drop any locks as we exepct we only have one local.cbor per process!
					drop(file);

					// log
					tracing::warn!(?path, ?errno, "locals-lock-failed");

					// index
					path = self
						.config_path
						.join(format!("{}-{}", self.identifier, index))
						.join("local.cbor");
					index += 1;
				},
			}
		}
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug))]
	async fn open_and_flock(&self) -> Result<FileLocalsFile, anyhow::Error> {
		let mut path = self.config_path.join(&self.identifier).join("local.cbor");

		// create and lock
		let mut index = 1;
		loop {
			// create parent dir
			tokio::fs::create_dir_all(path.parent().ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?).await?;

			// open
			let file = TokioFile(
				tokio::fs::OpenOptions::new()
					.read(true)
					.write(true)
					.create(true)
					.truncate(false)
					.open(&path)
					.await?,
			);

			// lock
			match Flock::lock(file, nix::fcntl::FlockArg::LockExclusiveNonblock) {
				Ok(lock) => {
					tracing::info!(?path, "locals-lock (flock)");
					return Ok(FileLocalsFile::Flock(path, lock));
				},
				Err((file, errno)) => {
					// close file
					// note: this should not drop any locks as we exepct we only have one local.cbor per process!
					drop(file);

					// log
					tracing::warn!(?path, ?errno, "locals-lock-failed");

					// index
					path = self
						.config_path
						.join(format!("{}-{}", self.identifier, index))
						.join("local.cbor");
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
enum FileLocalsFile {
	#[default]
	None,
	File(PathBuf, tokio::fs::File),
	Flock(PathBuf, Flock<TokioFile>),
	LockedFile(PathBuf, tokio::fs::File),
}
impl FileLocalsFile {
	fn file_mut(&mut self) -> Option<(&PathBuf, &mut tokio::fs::File)> {
		match self {
			Self::None => None,
			Self::File(path, file) => Some((path, file)),
			Self::Flock(path, lock) => Some((path, &mut lock.deref_mut().0)),
			Self::LockedFile(path, file) => Some((path, file)),
		}
	}

	fn is_none(&self) -> bool {
		matches!(self, Self::None)
	}
}

#[derive(Debug, Default)]
struct FileLocalsState {
	/// Loaded locals.
	locals: BTreeMap<PathBuf, ApplicationLocal>,

	/// Our local.cbor, if already written to, locked.
	file: FileLocalsFile,

	/// Active watchers.
	watchers: ResponseStreams<(PathBuf, ApplicationLocal)>,
	watch: Option<(DropGuard, JoinHandle<()>)>,
}
impl FileLocalsState {
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

	/// Write local to the locked file.
	async fn write(&mut self, local: ApplicationLocal) -> Result<(), anyhow::Error> {
		// get file
		let (path, file) = self.file.file_mut().ok_or(anyhow!("No file."))?;

		// apply
		self.locals.insert(path.clone(), local.clone());

		// serialize
		let data = to_cbor(&local)?;

		// log
		tracing::debug!(?path, ?local, "locals-write");

		// write
		file.set_len(0).await?;
		file.seek(std::io::SeekFrom::Start(0)).await?;
		file.write_all(&data).await?;
		file.flush().await?;

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

/// Watch for all local.cbor changes in config_path.
fn watch(config_path: PathBuf) -> Result<impl Stream<Item = PathBuf>, anyhow::Error> {
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
	tokio::spawn({
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
		.filter_map(move |event| {
			ready(match &event.kind {
				notify::EventKind::Create(CreateKind::File) | notify::EventKind::Modify(ModifyKind::Data(_)) => {
					let paths = event
						.paths
						.iter()
						.filter(|path| {
							path.parent().and_then(|f| f.parent()) == Some(config_path.as_ref())
								&& path.file_name().and_then(|f| f.to_str()) == Some("local.cbor")
						})
						.cloned()
						.collect();

					// log
					#[cfg(feature = "logging-verbose")]
					tracing::trace!(?paths, ?event, "locals-watch-send");

					// result
					Some(paths)
				},
				_ => {
					// log
					#[cfg(feature = "logging-verbose")]
					tracing::trace!(?event, "locals-watch-ignore");

					// none
					None
				},
			})
		})
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplicationLocal {
	/// The application local version.
	#[serde(rename = "v")]
	pub version: u8,

	/// The latest heads.
	/// Todo: Do we need this as this is encoded in the state anyway?
	#[serde(rename = "h")]
	pub heads: BTreeSet<Cid>,

	/// The latest state.
	#[serde(rename = "s")]
	pub state: Cid,

	/// The latest encryption mapping.
	#[serde(rename = "m", skip_serializing_if = "Option::is_none", default)]
	pub mapping: Option<Cid>,
}
impl ApplicationLocal {
	pub fn version() -> u8 {
		1
	}

	pub fn new(heads: BTreeSet<Cid>, state: Cid, mapping: Option<Cid>) -> Self {
		Self { heads, state, version: Self::version(), mapping }
	}

	/// Read path as ApplicationLocal expecting DAG-CBOR format.
	/// Returns `None` if file not exists.
	#[tracing::instrument(level = tracing::Level::TRACE, name = "locals-read", err(Debug))]
	pub async fn read(path: &PathBuf) -> anyhow::Result<Option<ApplicationLocal>> {
		Ok(
			match fs_read_option(path)
				.await
				.with_context(|| format!("Reading file: {:?}", path))?
			{
				Some(data) => {
					let result: ApplicationLocal = from_cbor(&data)?;
					if result.version != Self::version() {
						return Err(anyhow!("Invalid file version"));
					}
					Some(result)
				},
				None => None,
			},
		)
	}

	pub fn reducer_state(&self) -> CoReducerState {
		(self.state, self.heads.clone()).into()
	}
}

#[cfg(test)]
mod tests {
	use crate::library::locals::{ApplicationLocal, FileLocals, Locals};
	use co_primitives::BlockSerializer;
	use co_test::TmpDir;

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
		let mut locals = FileLocals::new(tmp.path().into(), "test".to_owned(), true).unwrap();
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
}
