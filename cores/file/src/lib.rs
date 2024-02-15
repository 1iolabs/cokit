use co_api::{reduce, Context, DagCollection, DagMap, DagSet, Date, Did, Reducer, ReducerAction, Tags};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub type FilePath = String;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct File {
	pub nodes: DagMap<FilePath, DagSet<Node>>,
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
	pub contents: FilePath,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileAction {
	Create { path: FilePath, node: Node },
	Remove { path: FilePath },
}

impl Reducer for File {
	type Action = FileAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
		let mut result = self;
		match &event.payload {
			FileAction::Create { path, node } => {
				let path = normalize_path(path);
				let mut nodes = result.nodes.get(context.storage());

				// insert if name not exists already
				let mut path_nodes = nodes.get(&path).cloned().unwrap_or_default().get(context.storage());
				if path_nodes.iter().find(|item| item.name() == node.name()).is_none() {
					// insert node
					path_nodes.insert(node.clone());

					// store
					nodes.insert(path.clone(), DagSet::create(context.storage_mut(), path_nodes));
					result.nodes.set(context.storage_mut(), nodes);
				}
			},
			FileAction::Remove { path } => {
				let path = normalize_path(path);
				let mut nodes = result.nodes.get(context.storage());

				// remove
				let (node_path, node_name) = path_and_file_name(&path);
				let path_nodes: BTreeSet<Node> = nodes
					.get(node_path)
					.cloned()
					.unwrap_or_default()
					.iter(context.storage())
					.filter(|node| node.name() != node_name)
					.collect();

				// store
				nodes.insert(node_path.to_owned(), DagSet::create(context.storage_mut(), path_nodes));
				result.nodes.set(context.storage_mut(), nodes);
			},
		}
		result
	}
}

/// Normalize path to connonized form.
/// Todo: Implement
fn normalize_path(path: &FilePath) -> FilePath {
	path.clone()
}

/// Normalize path to connonized form.
/// Todo: Implement
fn split_path<'a>(path: &'a FilePath) -> Vec<&'a str> {
	path.split("/").collect()
}

/// Normalize path to connonized form.
/// Todo: Implement
fn path_and_file_name(path: &FilePath) -> (&str, &str) {
	let file_name = file_name(path);
	(path.split_at(path.len() - file_name.len() - 1).0, file_name)
}

/// Normalize path to connonized form.
/// Todo: Implement
fn file_name(path: &FilePath) -> &str {
	split_path(path).last().unwrap_or(&"")
}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<File>()
}
