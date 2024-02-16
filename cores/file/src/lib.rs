use anyhow::anyhow;
use co_api::{
	reduce, AbsolutePath, Context, DagCollection, DagMap, DagSet, Date, Did, Path, PathExt, Reducer, ReducerAction,
	Tags,
};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct File {
	pub nodes: DagMap<AbsolutePath, DagSet<Node>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub enum Node {
	Folder(FolderNode),
	File(FileNode),
	Link(LinkNode),
}
impl Node {
	pub fn name(&self) -> &str {
		match self {
			Node::Folder(node) => &node.name,
			Node::File(node) => &node.name,
			Node::Link(node) => &node.name,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct FileNode {
	pub name: String,
	pub create_time: Date,
	pub modify_time: Date,
	pub size: u64,
	pub mode: u32,
	pub tags: Tags,
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
	pub contents: Path,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileAction {
	Create { path: AbsolutePath, node: Node },
	Remove { path: AbsolutePath },
}

impl Reducer for File {
	type Action = FileAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
		match &event.payload {
			FileAction::Create { path, node } => create(context, self, path, node).unwrap(),
			FileAction::Remove { path } => remove(context, self, path).unwrap(),
		}
	}
}

fn create(context: &mut dyn Context, mut state: File, path: &AbsolutePath, node: &Node) -> Result<File, anyhow::Error> {
	let path = path.normalize()?;

	// nodes
	let mut nodes = state.nodes.get(context.storage());

	// insert if name not exists already
	let mut path_nodes = nodes
		.get(&path)
		.ok_or(anyhow!("No such directory: {}", path))?
		.get(context.storage());
	if path_nodes.iter().find(|item| item.name() == node.name()).is_none() {
		// insert node
		path_nodes.insert(node.clone());

		// store
		nodes.insert(path.clone(), DagSet::create(context.storage_mut(), path_nodes));
		state.nodes.set(context.storage_mut(), nodes);
	}

	// result
	Ok(state)
}

fn remove(context: &mut dyn Context, mut state: File, path: &AbsolutePath) -> Result<File, anyhow::Error> {
	let path = path.normalize()?;
	let (parent_path, name) = path.parent_and_file_name_result()?;

	// nodes
	let mut nodes = state.nodes.get(context.storage());

	// remove
	let path_nodes: BTreeSet<Node> = nodes
		.get(&AbsolutePath::from_str_unchecked(parent_path))
		.cloned()
		.unwrap_or_default()
		.iter(context.storage())
		.filter(|node| node.name() != name)
		.collect();

	// store
	nodes.insert(AbsolutePath::from_str_unchecked(parent_path), DagSet::create(context.storage_mut(), path_nodes));
	state.nodes.set(context.storage_mut(), nodes);

	// result
	Ok(state)
}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<File>()
}
