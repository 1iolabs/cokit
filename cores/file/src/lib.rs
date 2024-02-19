use anyhow::anyhow;
use co_api::{
	reduce, tags, AbsolutePath, AbsolutePathOwned, Context, DagCollection, DagMap, DagSet, Date, Did, PathExt,
	PathOwned, Reducer, ReducerAction, Tags,
};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
	pub contents: PathOwned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileAction {
	Create { path: AbsolutePathOwned, node: Node, recursive: bool },
	Remove { path: AbsolutePathOwned },
}

impl Reducer for File {
	type Action = FileAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
		match &event.payload {
			FileAction::Create { path, node, recursive } =>
				create(context, self, &path, node, &event.from, event.time, *recursive).unwrap(),
			FileAction::Remove { path } => remove(context, self, path).unwrap(),
		}
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
	state.nodes.update(context, |context, mut paths| {
		// recursive?
		if recursive {
			for parent in path.parents() {
				if !paths.contains_key(parent) {
					create_folder(context, &mut paths, parent, from, time)?;
				}
			}
		} else if !paths.contains_key(AbsolutePath::new_unchecked("/")) {
			// implicitly create root on first create
			create_folder(context, &mut paths, AbsolutePath::new_unchecked("/"), from, time)?;
		}

		// insert if name not exists already
		create_node(context, &mut paths, &path, node.clone())
	})?;

	// result
	Ok(state)
}

fn create_node(
	context: &mut dyn Context,
	paths: &mut BTreeMap<AbsolutePathOwned, DagSet<Node>>,
	parent_path: &AbsolutePath,
	node: Node,
) -> Result<(), anyhow::Error> {
	paths
		.get_mut(parent_path)
		.ok_or(anyhow!("No such directory: {}", parent_path))?
		.update(context, |_, nodes| {
			// insert node if name not exists yet
			if nodes.iter().find(|item| item.name() == node.name()).is_none() {
				nodes.insert(node);
			}
			Ok(())
		})
}

fn create_folder(
	context: &mut dyn Context,
	paths: &mut BTreeMap<AbsolutePathOwned, DagSet<Node>>,
	path: &AbsolutePath,
	from: &Did,
	time: Date,
) -> Result<(), anyhow::Error> {
	let (parent_path, name) = match path.as_ref() {
		"/" => (AbsolutePath::new_unchecked("/"), ""),
		_ => path.parent_and_file_name_result()?,
	};
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

fn remove(context: &mut dyn Context, mut state: File, path: &AbsolutePath) -> Result<File, anyhow::Error> {
	let path = path.normalize()?;
	let (parent_path, name) = path.parent_and_file_name_result()?;

	// nodes
	let mut nodes = state.nodes.get(context.storage());

	// remove
	let path_nodes: BTreeSet<Node> = nodes
		.get(parent_path)
		.cloned()
		.unwrap_or_default()
		.iter(context.storage())
		.filter(|node| node.name() != name)
		.collect();

	// store
	if path_nodes.is_empty() {
		nodes.remove(parent_path);
	} else {
		nodes.insert(parent_path.to_owned(), DagSet::create(context.storage_mut(), path_nodes));
	}
	state.nodes.set(context.storage_mut(), nodes);

	// result
	Ok(state)
}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<File>()
}
