use anyhow::anyhow;
use cid::Cid;
use co_api::{
	tags, AbsolutePath, AbsolutePathOwned, Context, DagCollection, DagMap, DagSet, Date, Did, PathExt, PathOwned,
	Reducer, ReducerAction, Tags,
};
use serde::{Deserialize, Serialize};
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct File {
	pub nodes: DagMap<AbsolutePathOwned, DagSet<Node>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub enum Node {
	Folder(FolderNode),
	File(FileNode),
	Link(LinkNode),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct FolderNode {
	pub name: String,
	pub create_time: Date,
	pub modify_time: Date,
	pub tags: Tags,
	pub owner: Did,
	pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct LinkNode {
	pub name: String,
	pub tags: Tags,
	pub contents: PathOwned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl Reducer for File {
	type Action = FileAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
		match &event.payload {
			FileAction::Create { path, node, recursive } => {
				reduce_create(context, self, path, node, &event.from, event.time, *recursive).unwrap()
			},
			FileAction::Remove { path, recursive } => reduce_remove(context, self, path, *recursive).unwrap(),
			FileAction::Modify { path, modifications } => reduce_modify(context, self, path, modifications).unwrap(),
		}
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
		match self {
			Node::Folder(_) => true,
			_ => false,
		}
	}

	pub fn is_file(&self) -> bool {
		match self {
			Node::File(_) => true,
			_ => false,
		}
	}

	pub fn is_link(&self) -> bool {
		match self {
			Node::Link(_) => true,
			_ => false,
		}
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
			m => return Err(anyhow!("Unsupported modification: {:?}", m)),
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
			m => return Err(anyhow!("Unsupported modification: {:?}", m)),
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
			m => return Err(anyhow!("Unsupported modification: {:?}", m)),
		}
		Ok(())
	}
}

fn reduce_create(
	context: &mut dyn Context,
	mut state: File,
	path: &AbsolutePath,
	node: &Node,
	from: &Did,
	time: Date,
	recursive: bool,
) -> Result<File, anyhow::Error> {
	let path = path.normalize()?;

	// nodes
	state.nodes.try_update(context, |context, paths| {
		// test if node exists
		let node_path = path.join_path(node.name())?;
		if get_node(context, paths, &node_path, true)?.is_some() {
			// tracing::info(path = ?node_path, "path-exists");
			return Ok(());
		}

		// implicitly create empty root on first create
		if !paths.contains_key(AbsolutePath::new_unchecked("/")) {
			paths.insert(AbsolutePath::new_unchecked("/").to_owned(), Default::default());
		}

		// recursive?
		if recursive {
			for parent in path.paths() {
				if !paths.contains_key(parent) {
					create_folder(context, paths, parent, from, time)?;
				}
			}
		}

		// insert if name not exists already
		create_node(context, paths, &path, node.clone())
	})?;

	// result
	Ok(state)
}

fn reduce_remove(
	context: &mut dyn Context,
	mut state: File,
	path: &AbsolutePath,
	recursive: bool,
) -> Result<File, anyhow::Error> {
	let path = path.normalize()?;

	// nodes
	let mut nodes = state.nodes.collection(context.storage());

	// apply
	let (parent_path, name) = path.parent_and_file_name_result()?;

	// children
	let mut stack = VecDeque::new();
	stack.push_back(path.clone());
	while let Some(path) = stack.pop_front() {
		let children = nodes.get(&path);
		if let Some(children) = children {
			// do nothing if we still have children and not delete them
			if !recursive {
				return Ok(state);
			}

			// queue
			for child in children.iter(context.storage()) {
				stack.push_back(path.join_path(child.name())?);
			}

			// remove
			nodes.remove(&path);
		}
	}

	// remove
	remove_node_by_name(&mut nodes, context, parent_path, name);

	// apply to state
	state.nodes.set_collection(context.storage_mut(), nodes);

	// result
	Ok(state)
}

/// Remove node from set.
fn remove_node_by_name(
	paths: &mut BTreeMap<AbsolutePathOwned, DagSet<Node>>,
	context: &mut dyn Context,
	parent_path: &AbsolutePath,
	name: &str,
) -> BTreeSet<Node> {
	// remove
	let (nodes, removed_nodes): (BTreeSet<Node>, BTreeSet<Node>) = paths
		.get(parent_path)
		.cloned()
		.unwrap_or_default()
		.iter(context.storage())
		.partition(|node| node.name() != name);

	// store
	if nodes.is_empty() && parent_path != "/" {
		paths.remove(parent_path);
	} else {
		paths.insert(parent_path.to_owned(), DagSet::create(context.storage_mut(), nodes));
	}

	// result
	removed_nodes
}

fn reduce_modify(
	context: &mut dyn Context,
	mut state: File,
	path: &AbsolutePath,
	modifications: &Vec<FileModification>,
) -> Result<File, anyhow::Error> {
	let path = path.normalize()?;
	let (parent_path, name) = path.parent_and_file_name_result()?;
	let parent_path = parent_path.to_owned();
	let mut file_modification_context = FileModificationContext::new(path.clone());

	// move node
	for to_parent in modifications.iter().filter_map(|item| match item {
		FileModification::Move(p) => Some(p),
		_ => None,
	}) {
		state.nodes.try_update(context, |context, paths| {
			// validate: check `to_parent` exists
			let validated_to_parent = if to_parent == "/" {
				to_parent.to_owned()
			} else if let Some((to_parent, node)) = get_node(context, paths, &to_parent, true)? {
				if !node.is_dir() {
					return Err(anyhow!("Can only move into folders: {}", to_parent));
				}
				to_parent
			} else {
				return Err(anyhow!("Not found: {}", to_parent));
			};

			// validate: check node `name` dont exists in `to_parent`
			let to_path = validated_to_parent.join_path(name)?;
			if get_node(context, paths, &to_path, true)?.is_some() {
				return Err(anyhow!("Node exists: {}", to_path));
			}

			// remove
			let removed = remove_node_by_name(paths, context, &parent_path, name);

			// insert
			for node in removed {
				create_node(context, paths, &validated_to_parent, node)?;
			}

			// reparent
			file_modification_context.reparent(path.clone(), to_path)?;

			// result
			Ok(())
		})?;
	}

	// update node
	let modifications: Vec<&FileModification> = modifications
		.iter()
		.filter_map(|item| match item {
			FileModification::Move(_) => None,
			m => Some(m),
		})
		.collect();
	if !modifications.is_empty() {
		state.nodes.try_update_key(context, &parent_path, |context, _, item| {
			// validate
			for modification in modifications.iter() {
				match modification {
					FileModification::Rename(name) => {
						// check `name` dont exists as sibling
						if item.iter(context.storage()).find(|node| node.name() == name).is_some() {
							return Err(anyhow!("File exists: {}", parent_path.join_path(name)?));
						}
					},
					_ => {},
				}
			}

			// update item
			item.try_update_one(
				context,
				|_, node| node.name() == name,
				|_, node| {
					for modification in modifications.iter() {
						node.modify(&mut file_modification_context, modification)?;
					}
					Ok(())
				},
			)?;
			Ok(())
		})?;
	}

	// reparent children nodes
	if !file_modification_context.reparent.is_empty() {
		state.nodes.try_update(context, |context, nodes| {
			for (from, to) in file_modification_context.reparent.iter() {
				reparent(context, nodes, from, to)?;
			}
			Ok(())
		})?;
	}

	// result
	Ok(state)
}

fn reparent(
	context: &dyn Context,
	nodes: &mut BTreeMap<AbsolutePathOwned, DagSet<Node>>,
	from: &AbsolutePath,
	to: &AbsolutePath,
) -> Result<(), anyhow::Error> {
	if let Some(items) = nodes.remove(from) {
		// children
		for child in items.iter(context.storage()) {
			if child.is_dir() {
				reparent(context, nodes, &from.join_path(child.name())?, &to.join_path(child.name())?)?;
			}
		}

		// self
		if nodes.insert(to.to_owned(), items).is_some() {
			return Err(anyhow!("Path exists: {}", to));
		}
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

/// Returns the node and its absoulte path (without links if resolve_link is true).
/// TODO: Fix links in path
fn get_node(
	context: &mut dyn Context,
	paths: &BTreeMap<AbsolutePathOwned, DagSet<Node>>,
	path: &AbsolutePath,
	resolve_link: bool,
) -> Result<Option<(AbsolutePathOwned, Node)>, anyhow::Error> {
	let (parent_path, name) = path.parent_and_file_name_result()?;
	let nodes = match paths.get(parent_path) {
		Some(nodes) => nodes,
		None => return Ok(None),
	};
	let node = nodes.collection(context.storage()).into_iter().find(|node| node.name() == name);

	// resolve_link
	if let Some(node) = &node {
		if resolve_link {
			match node {
				Node::Link(link) => {
					let target = parent_path.join(&link.contents)?;
					return get_node(context, paths, &target, resolve_link);
				},
				_ => {},
			}
		}
	}

	// result
	Ok(node.map(|node| (path.to_owned(), node)))
}

fn create_node(
	context: &mut dyn Context,
	paths: &mut BTreeMap<AbsolutePathOwned, DagSet<Node>>,
	parent_path: &AbsolutePath,
	node: Node,
) -> Result<(), anyhow::Error> {
	// validate parent exists
	let validated_parent_path = match parent_path.as_str() {
		// root always exists
		"/" => parent_path.to_owned(),
		// check if node exists
		_ => {
			get_node(context, paths, parent_path, true)?
				.ok_or(anyhow!("No such directory: {}", parent_path))?
				.0
		},
	};

	// node
	let nodes = match paths.entry(validated_parent_path) {
		Entry::Occupied(o) => o.into_mut(),
		Entry::Vacant(v) => v.insert(Default::default()),
	};
	nodes.try_update(context, |_, nodes| {
		// insert node if name not exists yet
		if !nodes.iter().any(|item| item.name() == node.name()) {
			nodes.insert(node);
		}
		Ok(())
	})?;

	Ok(())
}

fn create_folder(
	context: &mut dyn Context,
	paths: &mut BTreeMap<AbsolutePathOwned, DagSet<Node>>,
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
	create_node(context, paths, parent_path, node)
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[no_mangle]
pub extern "C" fn state() {
	co_api::reduce::<File>()
}

#[cfg(test)]
mod tests {
	use crate::{File, FileAction, FileModification, FileNode, Node};
	use cid::Cid;
	use co_api::{
		AbsolutePath, Block, BlockSerializer, Context, DagCollection, DefaultParams, PathExt, Reducer, ReducerAction,
	};
	use co_storage::{MemoryStorage, Storage, StorageError};
	use std::{cell::RefCell, rc::Rc};

	#[derive(Debug, Default)]
	struct TestContext {
		storage: Rc<RefCell<MemoryStorage>>,
	}
	impl Context for TestContext {
		fn storage(&self) -> &dyn co_api::Storage {
			self
		}

		fn storage_mut(&mut self) -> &mut dyn co_api::Storage {
			self
		}

		fn event(&self) -> Cid {
			unimplemented!()
		}

		fn state(&self) -> Option<Cid> {
			unimplemented!()
		}

		fn store_state(&mut self, _cid: Cid) {
			unimplemented!()
		}
	}
	impl co_api::Storage for TestContext {
		fn get(&self, cid: &Cid) -> Block<DefaultParams> {
			self.storage.borrow().get(cid).unwrap()
		}

		fn set(&mut self, block: Block<DefaultParams>) -> Cid {
			self.storage.borrow_mut().set(block).unwrap()
		}
	}
	impl Storage for TestContext {
		type StoreParams = DefaultParams;

		fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
			self.storage.borrow().get(cid)
		}

		fn set(&mut self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
			self.storage.borrow_mut().set(block)
		}

		fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
			self.storage.borrow_mut().remove(cid)
		}
	}
	/// Create file state with:
	///  - `/hello` - folder
	///  - `/hello/world` - folder
	///  - `/hello/world/text.txt`: file with contents: "hello world"
	fn create_test_file_state() -> (TestContext, File) {
		let mut context = TestContext::default();
		let state = File::default();

		// create
		let block = BlockSerializer::default().serialize(&"hello world").unwrap();
		let contents = *block.cid();
		context.set(block).unwrap();
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
		let state = state.reduce(&action, &mut context);
		let paths = state.nodes.collection(&context);
		assert_eq!(paths.len(), 3); // "/", "/hello", "/hello/world"
		assert_eq!(paths.get(AbsolutePath::new_unchecked("/")).unwrap().collection(&context).len(), 1); // "hello"
		assert_eq!(
			paths
				.get(AbsolutePath::new_unchecked("/hello"))
				.unwrap()
				.collection(&context)
				.len(),
			1
		); // "world"
		assert_eq!(
			paths
				.get(AbsolutePath::new_unchecked("/hello/world"))
				.unwrap()
				.collection(&context)
				.len(),
			1
		); // "test.txt"

		// result
		(context, state)
	}
	fn names(context: &TestContext, state: &File, path: &str) -> Vec<String> {
		state
			.nodes
			.collection(context)
			.get(AbsolutePath::new_unchecked(path))
			.cloned()
			.unwrap_or_default()
			.collection(context)
			.iter()
			.map(Node::name)
			.map(ToOwned::to_owned)
			.collect::<Vec<_>>()
	}

	#[test]
	fn test_delete_recursive() {
		let (mut context, state) = create_test_file_state();

		// delete
		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Remove { path: "/hello".try_into().unwrap(), recursive: true },
		};
		let state = state.reduce(&action, &mut context);
		let paths = state.nodes.collection(&context);
		assert_eq!(paths.len(), 1); // "/"
		assert_eq!(paths.get(AbsolutePath::new_unchecked("/")).unwrap().collection(&context).len(), 0);
	}

	#[test]
	fn test_modify_rename() {
		let (mut context, state) = create_test_file_state();

		// rename
		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Modify {
				path: "/hello/world/test.txt".try_into().unwrap(),
				modifications: vec![FileModification::Rename("welcome.txt".to_owned())],
			},
		};
		let state = state.reduce(&action, &mut context);
		let paths = state.nodes.collection(&context);
		let files = paths
			.get(AbsolutePath::new_unchecked("/hello/world"))
			.unwrap()
			.collection(&context);
		assert_eq!(files.len(), 1);
		assert_eq!(files.first().unwrap().name(), "welcome.txt");
	}

	#[test]
	fn test_modify_rename_with_children() {
		let (mut context, state) = create_test_file_state();

		// rename with children
		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Modify {
				path: "/hello".try_into().unwrap(),
				modifications: vec![FileModification::Rename("test".to_owned())],
			},
		};
		let state = state.reduce(&action, &mut context);
		let paths = state.nodes.collection(&context);
		assert_eq!(paths.iter().map(|(k, _)| k.as_str()).collect::<Vec<&str>>(), vec!["/", "/test", "/test/world"]);
		assert_eq!(names(&context, &state, "/"), vec!["test"]);
		assert_eq!(names(&context, &state, "/test"), vec!["world"]);
		assert_eq!(names(&context, &state, "/test/world"), vec!["test.txt"]);
	}

	#[test]
	fn test_modify_move() {
		let (mut context, state) = create_test_file_state();

		// move
		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Modify {
				path: "/hello/world".try_into().unwrap(),
				modifications: vec![FileModification::Move("/".try_into().unwrap())],
			},
		};
		let state = state.reduce(&action, &mut context);
		let paths = state.nodes.collection(&context);
		assert_eq!(paths.iter().map(|(k, _)| k.as_str()).collect::<Vec<&str>>(), vec!["/", "/world"]); // "/hello" is empty now
		assert_eq!(names(&context, &state, "/"), Vec::<&str>::from(["hello", "world"]));
		assert_eq!(names(&context, &state, "/hello"), Vec::<&str>::from([]));
		assert_eq!(names(&context, &state, "/world"), Vec::<&str>::from(["test.txt"]));
	}

	#[test]
	fn test_modify_move_file() {
		let (mut context, state) = create_test_file_state();

		// move
		let action = ReducerAction {
			from: "did:local:test".to_owned(),
			core: "file".to_owned(),
			time: 456,
			payload: FileAction::Modify {
				path: "/hello/world/test.txt".try_into().unwrap(),
				modifications: vec![FileModification::Move("/hello".try_into().unwrap())],
			},
		};
		let state = state.reduce(&action, &mut context);
		let paths = state.nodes.collection(&context);
		assert_eq!(paths.iter().map(|(k, _)| k.as_str()).collect::<Vec<&str>>(), vec!["/", "/hello"]); // "/world" is empty now
		assert_eq!(names(&context, &state, "/"), Vec::<&str>::from(["hello"]));
		assert_eq!(names(&context, &state, "/hello"), Vec::<&str>::from(["world", "test.txt"]));
		assert_eq!(names(&context, &state, "/hello/world"), Vec::<&str>::from([]));
	}
}
