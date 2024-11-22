use anyhow::anyhow;
use co_api::{
	tags, AbsolutePath, AbsolutePathOwned, Context, DagCollection, DagMap, DagSet, Date, Did, PathExt, PathOwned,
	Reducer, ReducerAction, Tags,
};
use libipld::Cid;
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
				create(context, self, path, node, &event.from, event.time, *recursive).unwrap()
			},
			FileAction::Remove { path, recursive } => remove(context, self, path, *recursive).unwrap(),
			FileAction::Modify { path, modifications } => modify(context, self, path, modifications).unwrap(),
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

	pub fn modify(&mut self, modification: &FileModification) -> anyhow::Result<()> {
		match self {
			Node::Folder(folder_node) => folder_node.modify(modification),
			Node::File(file_node) => file_node.modify(modification),
			Node::Link(link_node) => link_node.modify(modification),
		}
	}
}
impl FileNode {
	pub fn modify(&mut self, modification: &FileModification) -> anyhow::Result<()> {
		match modification {
			FileModification::Rename(name) => {
				self.name = name.to_owned();
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
	pub fn modify(&mut self, modification: &FileModification) -> anyhow::Result<()> {
		match modification {
			FileModification::Rename(name) => {
				self.name = name.to_owned();
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
	pub fn modify(&mut self, modification: &FileModification) -> anyhow::Result<()> {
		match modification {
			FileModification::Rename(name) => {
				self.name = name.to_owned();
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

fn create(
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
		if get_node(context, paths, &node_path)?.is_some() {
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

fn remove(
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
	let path_nodes: BTreeSet<Node> = nodes
		.get(parent_path)
		.cloned()
		.unwrap_or_default()
		.iter(context.storage())
		.filter(|node| node.name() != name)
		.collect();

	// store
	if path_nodes.is_empty() && parent_path != "/" {
		nodes.remove(parent_path);
	} else {
		nodes.insert(parent_path.to_owned(), DagSet::create(context.storage_mut(), path_nodes));
	}

	// apply to state
	state.nodes.set_collection(context.storage_mut(), nodes);

	// result
	Ok(state)
}

fn modify(
	context: &mut dyn Context,
	mut state: File,
	path: &AbsolutePath,
	modifications: &Vec<FileModification>,
) -> Result<File, anyhow::Error> {
	let path = path.normalize()?;
	let (parent_path, name) = path.parent_and_file_name_result()?;
	let parent_path = parent_path.to_owned();

	// update file
	state
		.nodes
		.try_update_key(context, &parent_path, |context, _, item| {
			item.try_update_one(
				context,
				|_, node| node.name() == name && node.is_file(),
				|_, node| {
					for modification in modifications {
						node.modify(modification)?;
					}
					Ok(())
				},
			)
		})
		.ok(); // ignore error as we just do nothing for invalid transactions

	// result
	Ok(state)
}

fn get_node(
	context: &mut dyn Context,
	paths: &BTreeMap<AbsolutePathOwned, DagSet<Node>>,
	path: &AbsolutePath,
) -> Result<Option<Node>, anyhow::Error> {
	let (parent_path, name) = path.parent_and_file_name_result()?;
	let nodes = match paths.get(parent_path) {
		Some(nodes) => nodes,
		None => return Ok(None),
	};
	Ok(nodes.collection(context.storage()).into_iter().find(|node| node.name() == name))
}

fn create_node(
	context: &mut dyn Context,
	paths: &mut BTreeMap<AbsolutePathOwned, DagSet<Node>>,
	parent_path: &AbsolutePath,
	node: Node,
) -> Result<(), anyhow::Error> {
	// validate parent exists
	match parent_path.as_str() {
		// root always exists
		"/" => {},
		// check if node exists
		_ => {
			get_node(context, paths, parent_path)?.ok_or(anyhow!("No such directory: {}", parent_path))?;
		},
	}

	// node
	let nodes = match paths.entry(parent_path.to_owned()) {
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
	use crate::{File, FileAction, FileNode, Node};
	use co_api::{AbsolutePath, Cid, Context, DagCollection, Reducer, ReducerAction};
	use co_storage::{MemoryStorage, Storage, StorageError};
	use libipld::{cbor::DagCborCodec, multihash::Code, Block, DefaultParams};
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

	#[test]
	fn test_delete_recursive() {
		let mut context = TestContext::default();
		let state = File::default();

		// create
		let block = Block::encode(DagCborCodec, Code::Blake3_256, "hello world").unwrap();
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
}
