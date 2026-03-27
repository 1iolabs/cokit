// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use anyhow::anyhow;
use cid::Cid;
use co_api::{
	co, tags, AbsolutePath, AbsolutePathOwned, BlockStorageExt, CoMap, CoSet, CoreBlockStorage, Date, Did, Link,
	OptionLink, PathExt, PathOwned, Reducer, ReducerAction, Tags,
};
use futures::{FutureExt, TryStreamExt};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[co(state)]
pub struct File {
	pub nodes: CoMap<AbsolutePathOwned, CoSet<Node>>,
}

#[co]
pub enum Node {
	Folder(FolderNode),
	File(FileNode),
	Link(LinkNode),
}

#[co]
pub struct FileNode {
	pub name: String,
	pub create_time: Date,
	pub modify_time: Date,
	pub size: u64,
	pub mode: u32,
	pub tags: Tags,
	// #[external]
	pub contents: Cid,
	pub owner: Did,
}

#[co]
pub struct FolderNode {
	pub name: String,
	pub create_time: Date,
	pub modify_time: Date,
	pub tags: Tags,
	pub owner: Did,
	pub mode: u32,
}

#[co]
pub struct LinkNode {
	pub name: String,
	pub tags: Tags,
	pub contents: PathOwned,
}

#[co]
pub enum FileAction {
	/// Create a node.
	/// Ignored if a node with the same name already exists at path.
	Create {
		/// The parent to create the node in.
		path: AbsolutePathOwned,
		/// The node to create.
		node: Node,
		/// Whether to create parents recursively.
		recursive: bool,
	},

	/// Remove a node.
	/// If a node has children and recusive is set to false nothing will happen.
	Remove { path: AbsolutePathOwned, recursive: bool },

	/// Modify a node.
	Modify { path: AbsolutePathOwned, modifications: Vec<FileModification> },
}

#[co]
pub enum FileModification {
	/// Rename node to.
	Rename(String),

	/// Move node into path (as children).
	Move(AbsolutePathOwned),

	/// Set create time.
	SetCreateTime(Date),

	/// Set modify time.
	SetModifyTime(Date),

	/// Set mode.
	SetMode(u32),

	/// Set owner.
	SetOwner(Did),

	/// Insert tags.
	TagsInsert(Tags),

	/// Remove tags.
	TagsRemove(Tags),

	/// Set file contents.
	/// Only applicable to [`Node::File`].
	SetContents(Cid, u64),

	/// Set link target.
	/// Only applicable to [`Node::Link`].
	SetLink(PathOwned),
}

impl Reducer<FileAction> for File {
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<FileAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let action = storage.get_value(&event).await?;
		let mut result = storage.get_value_or_default(&state).await?;
		match &action.payload {
			FileAction::Create { path, node, recursive } => {
				reduce_create(storage, &mut result, path, node, &action.from, action.time, *recursive)
					.boxed()
					.await?;
			},
			FileAction::Remove { path, recursive } => {
				reduce_remove(storage, &mut result, path, *recursive).boxed().await?;
			},
			FileAction::Modify { path, modifications } => {
				reduce_modify(storage, &mut result, path, modifications).boxed().await?;
			},
		}
		Ok(storage.set_value(&result).await?)
	}
}

impl Node {
	pub fn name(&self) -> &str {
		match self {
			Node::Folder(node) => &node.name,
			Node::File(node) => &node.name,
			Node::Link(node) => &node.name,
		}
	}

	pub fn is_dir(&self) -> bool {
		matches!(self, Node::Folder(_))
	}

	pub fn is_file(&self) -> bool {
		matches!(self, Node::File(_))
	}

	pub fn is_link(&self) -> bool {
		matches!(self, Node::Link(_))
	}

	pub fn modify(
		&mut self,
		context: &mut FileModificationContext,
		modification: &FileModification,
	) -> anyhow::Result<()> {
		match self {
			Node::Folder(folder_node) => folder_node.modify(context, modification),
			Node::File(file_node) => file_node.modify(context, modification),
			Node::Link(link_node) => link_node.modify(context, modification),
		}
	}
}

impl FileNode {
	pub fn modify(
		&mut self,
		_context: &mut FileModificationContext,
		modification: &FileModification,
	) -> anyhow::Result<()> {
		match modification {
			FileModification::Rename(name) => {
				self.name = name.to_owned();
			},
			FileModification::Move(_) => {
				// nothing todo (files can not have children)
			},
			FileModification::SetCreateTime(time) => {
				self.create_time = *time;
			},
			FileModification::SetModifyTime(time) => {
				self.modify_time = *time;
			},
			FileModification::SetMode(mode) => {
				self.mode = *mode;
			},
			FileModification::SetOwner(owner) => {
				self.owner = owner.to_owned();
			},
			FileModification::TagsInsert(tags) => {
				self.tags.append(&mut tags.clone());
			},
			FileModification::TagsRemove(tags) => {
				self.tags.clear(Some(tags));
			},
			FileModification::SetContents(cid, size) => {
				self.contents = *cid;
				self.size = *size;
			},
			modification => return Err(anyhow!("Unsupported modification: {:?}", modification)),
		}
		Ok(())
	}
}

impl FolderNode {
	pub fn modify(
		&mut self,
		context: &mut FileModificationContext,
		modification: &FileModification,
	) -> anyhow::Result<()> {
		match modification {
			FileModification::Rename(name) => {
				if &self.name != name {
					context.reparent(
						context.path(),
						context
							.path()
							.parent()
							.ok_or(anyhow!("No parent: {}", context.path()))?
							.join_path(name)?,
					)?;
				}
				self.name = name.to_owned();
			},
			FileModification::Move(_to) => {
				// nothing todo (handles in `reduce_modify`)
			},
			FileModification::SetCreateTime(time) => {
				self.create_time = *time;
			},
			FileModification::SetModifyTime(time) => {
				self.modify_time = *time;
			},
			FileModification::SetMode(mode) => {
				self.mode = *mode;
			},
			FileModification::SetOwner(owner) => {
				self.owner = owner.to_owned();
			},
			FileModification::TagsInsert(tags) => {
				self.tags.append(&mut tags.clone());
			},
			FileModification::TagsRemove(tags) => {
				self.tags.clear(Some(tags));
			},
			modification => return Err(anyhow!("Unsupported modification: {:?}", modification)),
		}
		Ok(())
	}
}

impl LinkNode {
	pub fn modify(
		&mut self,
		_context: &mut FileModificationContext,
		modification: &FileModification,
	) -> anyhow::Result<()> {
		match modification {
			FileModification::Rename(name) => {
				self.name = name.to_owned();
			},
			FileModification::Move(_to) => {
				// TODO: change the symlink target?
				// nothing todo as links can not have children
			},
			// TODO: should symlink have own metadata? on posix they have:
			// A symlink has its own metadata, including:
			// Mode (permissions, typically lrwxrwxrwx)
			// (Possibly) a creation time, if supported by the filesystem
			// A modification time, reflecting changes to the symlink itself
			// The symlink's metadata is separate from that of the target file it points to.
			// FileModification::SetCreateTime(time) => {
			// 	self.create_time = *time;
			// },
			// FileModification::SetModifyTime(time) => {
			// 	self.modify_time = *time;
			// },
			// FileModification::SetMode(mode) => {
			// 	self.mode = *mode;
			// },
			// FileModification::SetOwner(owner) => {
			// 	self.owner = owner.to_owned();
			// },
			FileModification::TagsInsert(tags) => {
				self.tags.append(&mut tags.clone());
			},
			FileModification::TagsRemove(tags) => {
				self.tags.clear(Some(tags));
			},
			FileModification::SetLink(path) => {
				self.contents = path.to_owned();
			},
			modification => return Err(anyhow!("Unsupported modification: {:?}", modification)),
		}
		Ok(())
	}
}

async fn reduce_create(
	storage: &CoreBlockStorage,
	state: &mut File,
	path: &AbsolutePath,
	node: &Node,
	from: &Did,
	time: Date,
	recursive: bool,
) -> Result<(), anyhow::Error> {
	let path = path.normalize()?;

	// test if node exists
	let node_path = path.join_path(node.name())?;
	if get_node(storage, &state.nodes, &node_path, true).await?.is_some() {
		return Ok(());
	}

	// implicitly create empty root on first create
	let root: AbsolutePathOwned = AbsolutePath::new_unchecked("/").to_owned();
	if !state.nodes.contains(storage, &root).await? {
		state.nodes.insert(storage, root, Default::default()).await?;
	}

	// recursive?
	if recursive {
		for parent in path.paths() {
			let parent_owned = parent.to_owned();
			if !state.nodes.contains(storage, &parent_owned).await? {
				create_folder(storage, &mut state.nodes, parent, from, time).await?;
			}
		}
	}

	// insert if name not exists already
	create_node(storage, &mut state.nodes, &path, node.clone()).await
}

async fn reduce_remove(
	storage: &CoreBlockStorage,
	state: &mut File,
	path: &AbsolutePath,
	recursive: bool,
) -> Result<(), anyhow::Error> {
	let path = path.normalize()?;
	let (parent_path, name) = path.parent_and_file_name_result()?;

	// children
	let mut stack = VecDeque::new();
	stack.push_back(path.clone());
	while let Some(current) = stack.pop_front() {
		let children = state.nodes.get(storage, &current).await?;
		if let Some(children) = children {
			// do nothing if we still have children and not delete them
			if !recursive {
				return Ok(());
			}

			// queue children
			let child_nodes: Vec<Node> = children.stream(storage).try_collect().await?;
			for child in &child_nodes {
				stack.push_back(current.join_path(child.name())?);
			}

			// remove
			state.nodes.remove(storage, current).await?;
		}
	}

	// remove node from parent
	remove_node_by_name(storage, &mut state.nodes, parent_path, name).await?;

	Ok(())
}

async fn reduce_modify(
	storage: &CoreBlockStorage,
	state: &mut File,
	path: &AbsolutePath,
	modifications: &[FileModification],
) -> Result<(), anyhow::Error> {
	let path = path.normalize()?;
	let (parent_path, name) = path.parent_and_file_name_result()?;
	let parent_path = parent_path.to_owned();
	let mut file_modification_context = FileModificationContext::new(path.clone());

	// move node
	for to_parent in modifications.iter().filter_map(|item| match item {
		FileModification::Move(path) => Some(path),
		_ => None,
	}) {
		// validate: check `to_parent` exists
		let validated_to_parent = if to_parent == "/" {
			to_parent.to_owned()
		} else if let Some((to_parent, node)) = get_node(storage, &state.nodes, to_parent, true).await? {
			if !node.is_dir() {
				return Err(anyhow!("Can only move into folders: {}", to_parent));
			}
			to_parent
		} else {
			return Err(anyhow!("Not found: {}", to_parent));
		};

		// validate: check node `name` doesnt exist in `to_parent`
		let to_path = validated_to_parent.join_path(name)?;
		if get_node(storage, &state.nodes, &to_path, true).await?.is_some() {
			return Err(anyhow!("Node exists: {}", to_path));
		}

		// remove
		let removed = remove_node_by_name(storage, &mut state.nodes, &parent_path, name).await?;

		// insert
		for node in removed {
			create_node(storage, &mut state.nodes, &validated_to_parent, node).await?;
		}

		// reparent
		file_modification_context.reparent(path.clone(), to_path)?;
	}

	// update node
	let modifications: Vec<&FileModification> = modifications
		.iter()
		.filter_map(|item| match item {
			FileModification::Move(_) => None,
			modification => Some(modification),
		})
		.collect();
	if !modifications.is_empty() {
		// get the node set for the parent
		if let Some(mut node_set) = state.nodes.get(storage, &parent_path).await? {
			// check for rename conflicts
			for modification in modifications.iter() {
				if let FileModification::Rename(new_name) = modification {
					let has_conflict = node_set
						.stream(storage)
						.try_any(|node| std::future::ready(node.name() == new_name))
						.await?;
					if has_conflict {
						return Err(anyhow!("File exists: {}", parent_path.join_path(new_name)?));
					}
				}
			}

			// find and update the node
			let nodes: Vec<Node> = node_set.stream(storage).try_collect().await?;
			let mut updated_nodes = Vec::with_capacity(nodes.len());
			for mut node in nodes {
				if node.name() == name {
					for modification in modifications.iter() {
						node.modify(&mut file_modification_context, modification)?;
					}
				}
				updated_nodes.push(node);
			}
			node_set = CoSet::from_iter(storage, updated_nodes).await?;
			state.nodes.insert(storage, parent_path.clone(), node_set).await?;
		}
	}

	// reparent children nodes
	for (from, to) in file_modification_context.reparent.iter() {
		reparent(storage, &mut state.nodes, from, to).await?;
	}

	Ok(())
}

async fn reparent(
	storage: &CoreBlockStorage,
	nodes: &mut CoMap<AbsolutePathOwned, CoSet<Node>>,
	from: &AbsolutePath,
	to: &AbsolutePath,
) -> Result<(), anyhow::Error> {
	let from_owned = from.to_owned();
	if let Some(items) = nodes.remove(storage, from_owned).await? {
		// children
		let child_nodes: Vec<Node> = items.stream(storage).try_collect().await?;
		for child in &child_nodes {
			if child.is_dir() {
				Box::pin(reparent(storage, nodes, &from.join_path(child.name())?, &to.join_path(child.name())?))
					.await?;
			}
		}

		// self
		let to_owned = to.to_owned();
		if nodes.contains(storage, &to_owned).await? {
			return Err(anyhow!("Path exists: {}", to));
		}
		nodes.insert(storage, to_owned, items).await?;
	}
	Ok(())
}

#[derive(Debug)]
pub struct FileModificationContext {
	/// Current node path.
	path: AbsolutePathOwned,

	/// Reparent from -> to.
	reparent: BTreeMap<AbsolutePathOwned, AbsolutePathOwned>,
}
impl FileModificationContext {
	pub fn new(path: AbsolutePathOwned) -> Self {
		Self { path, reparent: Default::default() }
	}

	pub fn path(&self) -> AbsolutePathOwned {
		self.path.clone()
	}

	pub fn reparent(&mut self, from: AbsolutePathOwned, to: AbsolutePathOwned) -> Result<(), anyhow::Error> {
		let from = from.normalize()?;
		let to = to.normalize()?;
		if from != to {
			self.reparent.insert(from, to);
		}
		Ok(())
	}
}

/// Returns the node and its absolute path (without links if resolve_link is true).
async fn get_node(
	storage: &CoreBlockStorage,
	nodes: &CoMap<AbsolutePathOwned, CoSet<Node>>,
	path: &AbsolutePath,
	resolve_link: bool,
) -> Result<Option<(AbsolutePathOwned, Node)>, anyhow::Error> {
	let (parent_path, name) = path.parent_and_file_name_result()?;
	let parent_owned = parent_path.to_owned();
	let Some(node_set) = nodes.get(storage, &parent_owned).await? else {
		return Ok(None);
	};

	let all_nodes: Vec<Node> = node_set.stream(storage).try_collect().await?;
	let node = all_nodes.into_iter().find(|node| node.name() == name);

	// resolve_link
	if let Some(node) = &node {
		if resolve_link {
			if let Node::Link(link) = node {
				let target = parent_path.join(&link.contents)?;
				return Box::pin(get_node(storage, nodes, &target, resolve_link)).await;
			}
		}
	}

	Ok(node.map(|node| (path.to_owned(), node)))
}

async fn create_node(
	storage: &CoreBlockStorage,
	nodes: &mut CoMap<AbsolutePathOwned, CoSet<Node>>,
	parent_path: &AbsolutePath,
	node: Node,
) -> Result<(), anyhow::Error> {
	// validate parent exists
	let validated_parent_path = match parent_path.as_str() {
		// root always exists
		"/" => parent_path.to_owned(),
		// check if node exists
		_ => {
			get_node(storage, nodes, parent_path, true)
				.await?
				.ok_or(anyhow!("No such directory: {}", parent_path))?
				.0
		},
	};

	// get or create node set
	let mut node_set = nodes.get(storage, &validated_parent_path).await?.unwrap_or_default();

	// insert node if name not exists yet
	let all_nodes: Vec<Node> = node_set.stream(storage).try_collect().await?;
	let name_exists = all_nodes.iter().any(|existing| existing.name() == node.name());
	if !name_exists {
		node_set.insert(storage, node).await?;
		nodes.insert(storage, validated_parent_path, node_set).await?;
	}

	Ok(())
}

/// Remove node from set.
async fn remove_node_by_name(
	storage: &CoreBlockStorage,
	nodes: &mut CoMap<AbsolutePathOwned, CoSet<Node>>,
	parent_path: &AbsolutePath,
	name: &str,
) -> Result<BTreeSet<Node>, anyhow::Error> {
	let parent_owned = parent_path.to_owned();
	let node_set = nodes.get(storage, &parent_owned).await?.unwrap_or_default();

	let all_nodes: Vec<Node> = node_set.stream(storage).try_collect().await?;
	let mut kept = Vec::new();
	let mut removed = BTreeSet::new();
	for node in all_nodes {
		if node.name() == name {
			removed.insert(node);
		} else {
			kept.push(node);
		}
	}

	// store
	if kept.is_empty() && parent_path != "/" {
		nodes.remove(storage, parent_owned).await?;
	} else {
		let new_set = CoSet::from_iter(storage, kept).await?;
		nodes.insert(storage, parent_owned, new_set).await?;
	}

	Ok(removed)
}

async fn create_folder(
	storage: &CoreBlockStorage,
	nodes: &mut CoMap<AbsolutePathOwned, CoSet<Node>>,
	path: &AbsolutePath,
	from: &Did,
	time: Date,
) -> Result<(), anyhow::Error> {
	let (parent_path, name) = path.parent_and_file_name_result()?;
	let node = Node::Folder(FolderNode {
		name: name.to_owned(),
		create_time: time,
		modify_time: time,
		tags: tags!(),
		owner: from.to_owned(),
		mode: 0o665,
	});
	create_node(storage, nodes, parent_path, node).await
}

#[cfg(test)]
mod tests {
	use crate::{File, FileAction, FileModification, FileNode, Node};
	use co_api::{
		AbsolutePath, AbsolutePathOwned, BlockSerializer, BlockStorage, BlockStorageExt, CoreBlockStorage, Link,
		OptionLink, Reducer, ReducerAction,
	};
	use co_storage::MemoryBlockStorage;
	use futures::TryStreamExt;

	fn new_storage() -> MemoryBlockStorage {
		MemoryBlockStorage::default()
	}

	fn core_storage(storage: &MemoryBlockStorage) -> CoreBlockStorage {
		CoreBlockStorage::new(storage.clone(), false)
	}

	async fn create_test_file_state() -> (MemoryBlockStorage, Link<File>) {
		let storage = new_storage();

		// create
		let block = BlockSerializer::default().serialize(&"hello world").unwrap();
		let contents = *block.cid();
		storage.set(block).await.unwrap();
		let node = Node::File(FileNode {
			contents,
			create_time: 123,
			modify_time: 123,
			mode: 0o655,
			name: "test.txt".to_owned(),
			owner: "did:local:test".to_owned(),
			size: 11,
			tags: Default::default(),
		});
		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 123,
			payload: FileAction::Create { path: "/hello/world".try_into().unwrap(), node, recursive: true },
		};
		let action_link: Link<ReducerAction<FileAction>> = storage.set_value(&action).await.unwrap();
		let state_link: OptionLink<File> = OptionLink::none();
		let cs = core_storage(&storage);
		let state_link = File::reduce(state_link, action_link, &cs).await.unwrap();
		let state: File = storage.get_value(&state_link).await.unwrap();

		// verify
		let paths = collect_paths(&storage, &state).await;
		assert_eq!(paths.len(), 3); // "/", "/hello", "/hello/world"
		assert_eq!(nodes_at(&storage, &state, "/").await.len(), 1); // "hello"
		assert_eq!(nodes_at(&storage, &state, "/hello").await.len(), 1); // "world"
		assert_eq!(nodes_at(&storage, &state, "/hello/world").await.len(), 1); // "test.txt"

		(storage, state_link)
	}

	async fn collect_paths(storage: &MemoryBlockStorage, state: &File) -> Vec<AbsolutePathOwned> {
		state
			.nodes
			.stream(storage)
			.map_ok(|(key, _): (AbsolutePathOwned, _)| key)
			.try_collect::<Vec<AbsolutePathOwned>>()
			.await
			.unwrap()
	}

	async fn nodes_at(storage: &MemoryBlockStorage, state: &File, path: &str) -> Vec<Node> {
		let path_owned = AbsolutePath::new_unchecked(path).to_owned();
		match state.nodes.get(storage, &path_owned).await.unwrap() {
			Some(set) => set.stream(storage).try_collect().await.unwrap(),
			None => vec![],
		}
	}

	async fn names(storage: &MemoryBlockStorage, state: &File, path: &str) -> Vec<String> {
		nodes_at(storage, state, path)
			.await
			.iter()
			.map(|node| node.name().to_owned())
			.collect()
	}

	async fn reduce_action(
		storage: &MemoryBlockStorage,
		state_link: Link<File>,
		action: ReducerAction<FileAction>,
	) -> (File, Link<File>) {
		let action_link: Link<ReducerAction<FileAction>> = storage.set_value(&action).await.unwrap();
		let cs = core_storage(storage);
		let next_link = File::reduce(state_link.into(), action_link, &cs).await.unwrap();
		let state: File = storage.get_value(&next_link).await.unwrap();
		(state, next_link)
	}

	#[tokio::test]
	async fn test_create() {
		let (_storage, _state_link) = create_test_file_state().await;
	}

	#[tokio::test]
	async fn test_delete_recursive() {
		let (storage, state_link) = create_test_file_state().await;

		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Remove { path: "/hello".try_into().unwrap(), recursive: true },
		};
		let (state, _) = reduce_action(&storage, state_link, action).await;
		let paths = collect_paths(&storage, &state).await;
		assert_eq!(paths.len(), 1); // "/"
		assert_eq!(nodes_at(&storage, &state, "/").await.len(), 0);
	}

	#[tokio::test]
	async fn test_modify_rename() {
		let (storage, state_link) = create_test_file_state().await;

		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Modify {
				path: "/hello/world/test.txt".try_into().unwrap(),
				modifications: vec![FileModification::Rename("welcome.txt".to_owned())],
			},
		};
		let (state, _) = reduce_action(&storage, state_link, action).await;
		let files = nodes_at(&storage, &state, "/hello/world").await;
		assert_eq!(files.len(), 1);
		assert_eq!(files.first().unwrap().name(), "welcome.txt");
	}

	#[tokio::test]
	async fn test_modify_rename_with_children() {
		let (storage, state_link) = create_test_file_state().await;

		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Modify {
				path: "/hello".try_into().unwrap(),
				modifications: vec![FileModification::Rename("test".to_owned())],
			},
		};
		let (state, _) = reduce_action(&storage, state_link, action).await;
		let mut paths: Vec<String> = collect_paths(&storage, &state).await.iter().map(|p| p.to_string()).collect();
		paths.sort();
		assert_eq!(paths, vec!["/", "/test", "/test/world"]);
		assert_eq!(names(&storage, &state, "/").await, vec!["test"]);
		assert_eq!(names(&storage, &state, "/test").await, vec!["world"]);
		assert_eq!(names(&storage, &state, "/test/world").await, vec!["test.txt"]);
	}

	#[tokio::test]
	async fn test_modify_move() {
		let (storage, state_link) = create_test_file_state().await;

		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Modify {
				path: "/hello/world".try_into().unwrap(),
				modifications: vec![FileModification::Move("/".try_into().unwrap())],
			},
		};
		let (state, _) = reduce_action(&storage, state_link, action).await;
		let mut paths: Vec<String> = collect_paths(&storage, &state).await.iter().map(|p| p.to_string()).collect();
		paths.sort();
		assert_eq!(paths, vec!["/", "/world"]); // "/hello" is empty now
		let mut root_names = names(&storage, &state, "/").await;
		root_names.sort();
		assert_eq!(root_names, vec!["hello", "world"]);
		assert!(names(&storage, &state, "/hello").await.is_empty());
		assert_eq!(names(&storage, &state, "/world").await, vec!["test.txt"]);
	}

	#[tokio::test]
	async fn test_modify_move_file() {
		let (storage, state_link) = create_test_file_state().await;

		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Modify {
				path: "/hello/world/test.txt".try_into().unwrap(),
				modifications: vec![FileModification::Move("/hello".try_into().unwrap())],
			},
		};
		let (state, _) = reduce_action(&storage, state_link, action).await;
		let mut paths: Vec<String> = collect_paths(&storage, &state).await.iter().map(|p| p.to_string()).collect();
		paths.sort();
		assert_eq!(paths, vec!["/", "/hello"]); // "/world" is empty now
		assert_eq!(names(&storage, &state, "/").await, vec!["hello"]);
		let mut hello_names = names(&storage, &state, "/hello").await;
		hello_names.sort();
		assert_eq!(hello_names, vec!["test.txt", "world"]);
		assert!(names(&storage, &state, "/hello/world").await.is_empty());
	}
}
