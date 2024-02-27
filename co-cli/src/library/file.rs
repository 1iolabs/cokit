use co_core_co::CoAction;
use co_core_file::{FolderNode, Node};
use co_primitives::{tags, AbsolutePath, AbsolutePathOwned, PathError, PathExt};
use co_sdk::{
	CoReducer, CoReducerError, CoStorage, Cores, NodeStream, PrivateIdentity, StorageError, CO_CORE_FILE,
	CO_CORE_NAME_CO,
};
use futures::{pin_mut, Stream, StreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
};

#[derive(Debug, thiserror::Error)]
pub enum FileError {
	#[error("No such file or directory: {0}")]
	NoEntry(String, #[source] anyhow::Error),

	#[error("Not a file: {0}")]
	NoFile(String, #[source] anyhow::Error),

	#[error("Storage error")]
	Storage(#[from] StorageError),

	#[error("Reducer error")]
	Reducer(#[from] CoReducerError),

	#[error("Path error")]
	Path(#[from] PathError),

	#[error("Other")]
	Other(#[from] anyhow::Error),
}

/// Get file core state.
/// If the core not exists yet create it.
pub async fn file_core<I>(co_reducer: CoReducer, identity: &I, core: &str) -> Result<co_core_file::File, FileError>
where
	I: PrivateIdentity + Debug + Send + Sync,
{
	match co_reducer.state(core).await {
		Err(CoReducerError::CoreNotFound(_)) => {
			// create core
			let create = CoAction::CoreCreate {
				core: core.to_owned(),
				binary: Cores::default().binary(CO_CORE_FILE).expect(CO_CORE_FILE),
				tags: tags!( "core": CO_CORE_FILE ),
			};
			co_reducer.push(identity, CO_CORE_NAME_CO, &create).await?;

			// assume default state
			Ok(Default::default())
		},
		result => Ok(result?),
	}
}

/// List all nodes in path.
pub fn list_nodes(
	storage: CoStorage,
	file_state: co_core_file::File,
	path: AbsolutePathOwned,
) -> impl Stream<Item = Result<Node, StorageError>> {
	async_stream::try_stream! {
		let stream = NodeStream::from_node_container(storage.clone(), &file_state.nodes);
		for await directory in stream {
			let (directory_path, children) = directory?;
			if directory_path == path {
				let children_stream = NodeStream::from_node_container(storage.clone(), &children);
				for await node in children_stream {
					yield node?
				}
				break;
			}
		}
	}
}

/// Get nodes for absolute paths.
pub async fn get_nodes(
	storage: CoStorage,
	file_state: co_core_file::File,
	paths: BTreeSet<AbsolutePathOwned>,
) -> Result<BTreeMap<AbsolutePathOwned, Node>, StorageError> {
	let mut result = BTreeMap::new();

	// root node
	let root = AbsolutePath::from_str_unchecked("/").to_owned();
	if paths.contains(&root) {
		result.insert(
			root,
			Node::Folder(FolderNode {
				name: "".to_owned(),
				create_time: 0,
				modify_time: 0,
				tags: tags!(),
				owner: "".to_owned(),
				mode: 0o665,
			}),
		);
	}

	// other nodes
	if result.len() != paths.len() {
		let parent_paths = paths
			.iter()
			.map(|path| -> Result<AbsolutePathOwned, StorageError> {
				path.parent_result()
					.map(|e| e.to_owned())
					.map_err(|e| StorageError::InvalidArgument(e.into()))
			})
			.collect::<Result<BTreeSet<AbsolutePathOwned>, StorageError>>()?;
		let nodes = nodes(storage, file_state, Some(parent_paths));
		pin_mut!(nodes);
		while let Some(item) = nodes.next().await {
			let (node_path, node) = item?;
			if paths.contains(&node_path) {
				result.insert(node_path, node);
				if result.len() == paths.len() {
					break;
				}
			}
		}
	}
	Ok(result)
}

/// List all nodes.
/// Optionally list all nodes in `paths`.
/// Returns the full node path and the node.
pub fn nodes(
	storage: CoStorage,
	file_state: co_core_file::File,
	paths: Option<BTreeSet<AbsolutePathOwned>>,
) -> impl Stream<Item = Result<(AbsolutePathOwned, Node), StorageError>> {
	let mut seen_paths = if let Some(paths) = &paths { paths.len() } else { 0 };
	async_stream::try_stream! {
		let stream = NodeStream::from_node_container(storage.clone(), &file_state.nodes);
		for await directory in stream {
			let (directory_path, children) = directory?;

			// filter?
			if let Some(paths) = &paths {
				if paths.contains(&directory_path) {
					seen_paths = seen_paths - 1;
				} else {
					continue;
				}
			}

			// nodes
			let children_stream = NodeStream::from_node_container(storage.clone(), &children);
			for await node in children_stream {
				let node = node?;
				yield (directory_path.join_path(node.name()).map_err(|e| StorageError::Internal(e.into()))?, node)
			}

			// done?
			if paths.is_some() &&  seen_paths == 0 {
				break;
			}
		}
	}
}
