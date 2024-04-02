use super::{fs_read::fs_read_option, fs_write::fs_write};
use anyhow::Context;
use futures::{Stream, TryStreamExt};
use libipld::Cid;
use notify::{
	event::{CreateKind, DataChange, ModifyKind},
	RecursiveMode, Watcher,
};
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	io::ErrorKind,
	path::{Path, PathBuf},
};

pub struct Locals {
	config_path: PathBuf,
	locals: BTreeMap<PathBuf, ApplicationLocal>,
}
impl Locals {
	/// Create locals by reading all local configurations.
	///
	/// # Arguments
	/// * `config_path` - The local configuratin path. Normally `{base_path}/etc`.
	pub async fn new(config_path: PathBuf) -> Result<Self, anyhow::Error> {
		// read
		let locals: BTreeMap<PathBuf, ApplicationLocal> = Self::read(config_path.clone())
			// .filter_map(|item| async move {
			// 	match item {
			// 		Ok(r) => Some(r),
			// 		Err(err) => {
			// 			tracing::warn!(?err, "locals-read-configuration-failed");
			// 			None
			// 		},
			// 	}
			// })
			.try_collect()
			.await?;

		// result
		Ok(Self { config_path, locals })
	}

	/// Iterate over locals.
	pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &ApplicationLocal)> {
		self.locals.iter()
	}

	/// Watch if any locals change.
	pub fn watch(self) -> tokio::sync::mpsc::UnboundedReceiver<(PathBuf, ApplicationLocal)> {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<(PathBuf, ApplicationLocal)>();
		let config_path = self.config_path;
		let mut locals = self.locals;

		// spawn
		std::thread::spawn(move || {
			let result: Result<(), anyhow::Error> = (move || {
				// watch
				let (watcher_tx, watcher_rx) = std::sync::mpsc::channel();
				let mut watcher = notify::recommended_watcher(watcher_tx)?;
				watcher.watch(&config_path, RecursiveMode::Recursive)?;

				// process
				loop {
					match watcher_rx.recv()? {
						Ok(event) => match &event.kind {
							notify::EventKind::Create(CreateKind::File) |
							notify::EventKind::Modify(ModifyKind::Data(DataChange::Content)) => {
								for path in &event.paths {
									if path.parent().and_then(|f| f.parent()) == Some(config_path.as_ref()) &&
										path.file_name().and_then(|f| f.to_str()) == Some("local.cbor")
									{
										tracing::trace!(?path, ?event, "locals-watch-test");
										match ApplicationLocal::read_sync(path) {
											Ok(local) => {
												if match locals.get(path) {
													Some(other) => other.heads != local.heads,
													None => true,
												} {
													// store
													locals.insert(path.clone(), local.clone());

													// send update
													if tx.send((path.clone(), local)).is_err() {
														// stop thread when rx has been dropped
														return Ok(());
													}
												}
											},
											Err(err) => {
												tracing::trace!(?event, ?path, ?err, "locals-watch-read-failed");
											},
										}
									}
								}
							},
							_ => {
								tracing::trace!(?event, "locals-watch-ignore");
							},
						},
						Err(err) => {
							tracing::warn!(?err, "locals-watch-error");
						},
					}
				}
			})();
			match result {
				Ok(_) => tracing::trace!("locals-watch-end"),
				Err(err) => tracing::warn!(?err, "locals-watch-failed"),
			}
		});

		// result
		rx
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
	#[serde(rename = "m")]
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
	pub async fn read(path: &PathBuf) -> anyhow::Result<Option<ApplicationLocal>> {
		Ok(
			match fs_read_option(path)
				.await
				.with_context(|| format!("Reading file: {:?}", path))?
			{
				Some(data) => {
					let result: ApplicationLocal = serde_ipld_dagcbor::from_slice(&data)?;
					if result.version != Self::version() {
						return Err(anyhow::anyhow!("Invalid file version"));
					}
					Some(result)
				},
				None => None,
			},
		)
	}

	fn read_sync(path: impl AsRef<Path>) -> anyhow::Result<ApplicationLocal> {
		let data = std::fs::read(&path).with_context(|| format!("Reading file: {:?}", path.as_ref().display()))?;
		let result: ApplicationLocal = serde_ipld_dagcbor::from_slice(&data)?;
		if result.version != Self::version() {
			return Err(anyhow::anyhow!("Invalid file version"));
		}
		Ok(result)
	}

	pub async fn write(&self, path: &PathBuf) -> anyhow::Result<()> {
		// serialize
		let data = serde_ipld_dagcbor::to_vec(self)?;

		// write
		fs_write(path, data, true)
			.await
			.with_context(|| format!("Writing file: {:?}", path))?;

		// result
		Ok(())
	}
}
