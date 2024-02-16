use core::str;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Display};

/// Path.
/// Can be relative or absolute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Path(String);
impl Path {
	pub fn from_str(buf: &str) -> Result<Self, PathError> {
		Self::new(buf.to_owned())
	}

	pub fn from_str_unchecked(buf: &str) -> Self {
		Self::new_unchecked(buf.to_owned())
	}
}
impl PathExt for Path {
	fn new(buf: String) -> Result<Self, PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		Ok(Self(buf))
	}

	fn new_unchecked(buf: String) -> Self {
		Self(buf)
	}

	fn as_string(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		matches!(self.as_string().as_bytes().first(), Some(b'/'))
	}
}
impl Into<String> for Path {
	fn into(self) -> String {
		self.0
	}
}
impl<'a> IntoIterator for &'a Path {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl AsRef<str> for Path {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl Display for Path {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_string())
	}
}

/// Absolute Path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AbsolutePath(String);
impl AbsolutePath {
	pub fn from_str(buf: &str) -> Result<Self, PathError> {
		Self::new(buf.to_owned())
	}

	pub fn from_str_unchecked(buf: &str) -> Self {
		Self::new_unchecked(buf.to_owned())
	}
}
impl PathExt for AbsolutePath {
	fn new(buf: String) -> Result<Self, PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		if !matches!(buf.as_bytes().first(), Some(b'/')) {
			return Err(PathError::InvalidArgument);
		}
		Ok(Self(buf))
	}

	fn new_unchecked(buf: String) -> Self {
		Self(buf)
	}

	fn as_string(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		true
	}
}
impl Into<String> for AbsolutePath {
	fn into(self) -> String {
		self.0
	}
}
impl<'a> IntoIterator for &'a AbsolutePath {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl AsRef<str> for AbsolutePath {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl Display for AbsolutePath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_string())
	}
}

/// Relative Path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelativePath(String);
impl RelativePath {
	pub fn from_str(buf: &str) -> Result<Self, PathError> {
		Self::new(buf.to_owned())
	}

	pub fn from_str_unchecked(buf: &str) -> Self {
		Self::new_unchecked(buf.to_owned())
	}
}
impl PathExt for RelativePath {
	fn new(buf: String) -> Result<Self, PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		if matches!(buf.as_bytes().first(), Some(b'/')) {
			return Err(PathError::InvalidArgument);
		}
		Ok(Self(buf))
	}

	fn new_unchecked(buf: String) -> Self {
		Self(buf)
	}

	fn as_string(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		false
	}
}
impl Into<String> for RelativePath {
	fn into(self) -> String {
		self.0
	}
}
impl<'a> IntoIterator for &'a RelativePath {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl AsRef<str> for RelativePath {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl Display for RelativePath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_string())
	}
}

/// Path.
/// Can be relative or absolute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PathRef<'a>(Cow<'a, str>);
impl<'a> PathRef<'a> {
	pub fn from_str(buf: &'a str) -> Result<Self, PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		Ok(Self(Cow::Borrowed(buf)))
	}

	pub fn from_str_unchecked(buf: &'a str) -> Self {
		Self(Cow::Borrowed(buf))
	}
}
impl<'a> PathExt for PathRef<'a> {
	fn new(buf: String) -> Result<Self, PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		Ok(Self(Cow::Owned(buf)))
	}

	fn new_unchecked(buf: String) -> Self {
		Self(Cow::Owned(buf))
	}

	fn as_string(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		matches!(self.as_string().as_bytes().first(), Some(b'/'))
	}
}
impl<'a> Into<String> for PathRef<'a> {
	fn into(self) -> String {
		self.0.into_owned()
	}
}
impl<'a> IntoIterator for &'a PathRef<'a> {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl<'a> AsRef<str> for PathRef<'a> {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl<'a> Display for PathRef<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_string())
	}
}

/// Absolute Path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AbsolutePathRef<'a>(Cow<'a, str>);
impl<'a> AbsolutePathRef<'a> {
	pub fn from_str(buf: &'a str) -> Result<Self, PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		if !matches!(buf.as_bytes().first(), Some(b'/')) {
			return Err(PathError::InvalidArgument);
		}
		Ok(Self(Cow::Borrowed(buf)))
	}

	pub fn from_str_unchecked(buf: &'a str) -> Self {
		Self(Cow::Borrowed(buf))
	}
}
impl<'a> PathExt for AbsolutePathRef<'a> {
	fn new(buf: String) -> Result<Self, PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		if !matches!(buf.as_bytes().first(), Some(b'/')) {
			return Err(PathError::InvalidArgument);
		}
		Ok(Self(Cow::Owned(buf)))
	}

	fn new_unchecked(buf: String) -> Self {
		Self(Cow::Owned(buf))
	}

	fn as_string(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		true
	}
}
impl<'a> Into<String> for AbsolutePathRef<'a> {
	fn into(self) -> String {
		self.0.into_owned()
	}
}
impl<'a> IntoIterator for &'a AbsolutePathRef<'a> {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl<'a> AsRef<str> for AbsolutePathRef<'a> {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl<'a> Display for AbsolutePathRef<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_string())
	}
}

/// Relative Path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelativePathRef<'a>(Cow<'a, str>);
impl<'a> RelativePathRef<'a> {
	pub fn from_str(buf: &'a str) -> Result<Self, PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		if matches!(buf.as_bytes().first(), Some(b'/')) {
			return Err(PathError::InvalidArgument);
		}
		Ok(Self(Cow::Borrowed(buf)))
	}
}
impl<'a> PathExt for RelativePathRef<'a> {
	fn new(buf: String) -> Result<Self, PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		if matches!(buf.as_bytes().first(), Some(b'/')) {
			return Err(PathError::InvalidArgument);
		}
		Ok(Self(Cow::Owned(buf)))
	}

	fn new_unchecked(buf: String) -> Self {
		Self(Cow::Owned(buf))
	}

	fn as_string(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		false
	}
}
impl<'a> Into<String> for RelativePathRef<'a> {
	fn into(self) -> String {
		self.0.into_owned()
	}
}
impl<'a> IntoIterator for &'a RelativePathRef<'a> {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl<'a> AsRef<str> for RelativePathRef<'a> {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl<'a> Display for RelativePathRef<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_string())
	}
}

/// Path components.
pub struct Components<'a> {
	path: &'a str,
	has_root: bool,
}
impl<'a> Components<'a> {
	fn parse_single_component<'b>(&self, comp: &'b str) -> Option<Component<'b>> {
		match comp {
			"." => Some(Component::CurDir),
			".." => Some(Component::ParentDir),
			"" if self.has_root => Some(Component::RootDir),
			"" if !self.path.is_empty() => Some(Component::CurDir), // empty dir: `hello//world`
			"" => None,
			_ => Some(Component::Normal(comp)),
		}
	}

	fn parse_next_component(&self) -> (usize, Option<Component<'a>>) {
		let (extra, comp) = match self.path.as_bytes().iter().position(|b| is_sep_byte(*b)) {
			None => (0, self.path),
			Some(i) => (1, &self.path[..i]),
		};
		(comp.len() + extra, self.parse_single_component(comp))
	}
}
impl<'a> Iterator for Components<'a> {
	type Item = Component<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		let (index, comp) = self.parse_next_component();
		self.path = &self.path[index..];
		self.has_root = false;
		comp
	}
}

/// Path component.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Component<'a> {
	RootDir,
	CurDir,
	ParentDir,
	Normal(&'a str),
}
impl<'a> Component<'a> {
	pub fn as_string(&self) -> &'a str {
		match self {
			Component::RootDir => "/",
			Component::CurDir => ".",
			Component::ParentDir => "..",
			Component::Normal(s) => s,
		}
	}

	/// Actual length of hte component (without separators).
	pub fn len(&self) -> usize {
		match self {
			Component::RootDir => 1,
			Component::CurDir => 1,
			Component::ParentDir => 2,
			Component::Normal(s) => s.len(),
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum PathError {
	#[error("No root")]
	NoRoot,

	#[error("No parent")]
	NoParent,

	#[error("Invalid argument")]
	InvalidArgument,
}

pub trait PathExt: Clone {
	fn new(buf: String) -> Result<Self, PathError>;

	fn new_unchecked(buf: String) -> Self;

	fn as_string(&self) -> &'_ str;

	fn has_root(&self) -> bool;

	/// Path components.
	fn components(&self) -> Components<'_> {
		Components { path: self.as_string(), has_root: self.has_root() }
	}

	/// Parent directory.
	fn parent(&self) -> Option<&'_ str> {
		match self.components().last() {
			Some(Component::Normal(name)) => {
				let path = self.as_string();
				Some(path.split_at(path.len() - name.len() - 1).0)
			},
			_ => None,
		}
	}

	/// Parent directories as full path starting at root.
	fn parents<'a>(&'a self) -> impl Iterator<Item = &'a str> {
		self.components()
			.scan((0 as usize, self.as_string()), |(index, path), component| {
				let end = *index + component.len();
				if end == path.len() {
					return None
				}
				let result = &path[0..end];
				*index = if *index > 0 { end + 1 } else { end };
				Some(result)
			})
	}

	/// Path and filename.
	fn parent_and_file_name(&self) -> Option<(&'_ str, &'_ str)> {
		match self.components().last() {
			Some(Component::Normal(name)) => {
				let path = self.as_string();
				Some((path.split_at(path.len() - name.len() - 1).0, name))
			},
			_ => None,
		}
	}

	/// Path and filename.
	fn parent_and_file_name_result(&self) -> Result<(&'_ str, &'_ str), PathError> {
		self.parent_and_file_name().ok_or(PathError::NoParent)
	}

	/// File name.
	fn file_name(&self) -> Option<&'_ str> {
		self.components().last().and_then(|f| match f {
			Component::Normal(name) => Some(name),
			_ => None,
		})
	}

	/// File name.
	fn file_name_result(&self) -> Result<&'_ str, PathError> {
		self.file_name().ok_or(PathError::NoParent)
	}

	/// Normalize path to connonized form.
	fn normalize(&self) -> Result<Self, PathError> {
		Ok(Self::new_unchecked(from_components(normalize_components(self.components())?)))
	}

	/// Join and normalize components into an path.
	fn join<'a>(&'a self, other: impl IntoIterator<Item = Component<'a>>) -> Result<Self, PathError> {
		Ok(Self::new_unchecked(from_components::<'a>(normalize_components(self.components().chain(other))?)))
	}
}

fn normalize_components<'a>(
	components: impl IntoIterator<Item = Component<'a>>,
) -> Result<Vec<Component<'a>>, PathError> {
	let mut stack: Vec<_> = components.into_iter().filter(|c| !matches!(c, Component::CurDir)).collect();
	let mut index = 0;
	while index < stack.len() {
		match stack[index] {
			Component::CurDir => {
				// remove
				stack.remove(index);
			},
			Component::RootDir => {
				// remove all elements before index
				for _ in 0..index {
					stack.remove(0);
				}

				// continue with elements after root
				index = 1;
			},
			Component::ParentDir =>
				if index > 0 {
					// check component before
					match stack[index - 1] {
						// fail of we go beyound root
						Component::RootDir => return Err(PathError::NoRoot),
						// keep parent dir if previous is also an parent dir
						Component::ParentDir => {
							index = index + 1;
							continue;
						},
						_ => {},
					}

					// remove dir and parentdir
					stack.remove(index - 1);
					stack.remove(index - 1);

					// continue with next element
					index = index - 1;
				} else {
					// keep parent (..) when on start
					index = index + 1;
				},
			_ => {
				// keep
				index = index + 1;
			},
		}
	}
	Ok(stack)
}

fn from_components<'a>(components: impl IntoIterator<Item = Component<'a>>) -> String {
	let result: String = components
		.into_iter()
		.scan(false, |state, c| {
			let result_state = *state;
			match &c {
				Component::RootDir => {
					*state = false;
				},
				_ => {
					*state = true;
				},
			}
			Some((result_state, c))
		})
		.flat_map(|(separator, c)| match separator {
			false => ["", c.as_string()],
			true => ["/", c.as_string()],
		})
		.collect();
	result
}

#[inline]
pub fn is_sep_byte(b: u8) -> bool {
	b == b'/'
}

#[cfg(test)]
mod tests {
	use crate::{AbsolutePath, Component, Path, PathExt, PathRef, RelativePath};

	#[test]
	fn test_components() {
		let path = Path::new("/hello/world".to_owned()).unwrap();
		let mut components = path.components();
		assert_eq!(Some(Component::RootDir), components.next());
		assert_eq!(Some(Component::Normal("hello")), components.next());
		assert_eq!(Some(Component::Normal("world")), components.next());
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_components_empty_component() {
		let path = Path::new("/hello//world".to_owned()).unwrap();
		let mut components = path.components();
		assert_eq!(Some(Component::RootDir), components.next());
		assert_eq!(Some(Component::Normal("hello")), components.next());
		assert_eq!(Some(Component::CurDir), components.next());
		assert_eq!(Some(Component::Normal("world")), components.next());
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_components_empty() {
		let path = Path::new_unchecked("".to_owned());
		let mut components = path.components();
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_relative_components() {
		let path = RelativePath::new("./hello/world".to_owned()).unwrap();
		let mut components = path.components();
		assert_eq!(Some(Component::CurDir), components.next());
		assert_eq!(Some(Component::Normal("hello")), components.next());
		assert_eq!(Some(Component::Normal("world")), components.next());
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_absolute_components() {
		let path = AbsolutePath::new("/hello/world".to_owned()).unwrap();
		let mut components = path.components();
		assert_eq!(Some(Component::RootDir), components.next());
		assert_eq!(Some(Component::Normal("hello")), components.next());
		assert_eq!(Some(Component::Normal("world")), components.next());
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_parents() {
		let path = PathRef::from_str_unchecked("/hello/world/test.zip");
		let mut parents = path.parents();
		assert_eq!(Some("/"), parents.next());
		assert_eq!(Some("/hello"), parents.next());
		assert_eq!(Some("/hello/world"), parents.next());
		assert_eq!(None, parents.next());
	}

	#[test]
	fn test_normalize() {
		fn normalize(s: &str) -> String {
			Path::new(s.to_owned()).unwrap().normalize().unwrap().into()
		}
		assert_eq!("/hello/test", normalize("/hello/test"));
		assert_eq!("test", normalize("hello/.././test"));
		assert_eq!("/test/hello", normalize("/test//hello"));
		assert_eq!("../test", normalize("../test"));
		assert_eq!("/", normalize("/"));
		assert_eq!("/", normalize("//"));
		assert_eq!("/test", normalize("/test/"));
		assert_eq!("/test", normalize("/test//"));
		assert_eq!("../test", normalize("./../test"));
		assert_eq!("../../test", normalize("./../../test"));
		assert_eq!("../../../test", normalize("./../../../test"));
	}

	#[test]
	fn test_file_name() {
		assert_eq!(Some("test"), Path::new("/hello/test".to_owned()).unwrap().file_name());
		assert_eq!(Some("test.zip"), Path::new("hello/.././test.zip".to_owned()).unwrap().file_name());
		assert_eq!(None, Path::new("hello/.././test.zip/..".to_owned()).unwrap().file_name());
		assert_eq!(None, Path::new("/".to_owned()).unwrap().file_name());
	}

	#[test]
	fn test_path_and_file_name() {
		assert_eq!(Some(("/hello", "test")), Path::new("/hello/test".to_owned()).unwrap().parent_and_file_name());
		assert_eq!(
			Some(("hello/../.", "test.zip")),
			Path::new("hello/.././test.zip".to_owned()).unwrap().parent_and_file_name()
		);
		assert_eq!(None, Path::new("hello/.././test.zip/..".to_owned()).unwrap().parent_and_file_name());
		assert_eq!(None, Path::new("/".to_owned()).unwrap().parent_and_file_name());
	}

	#[test]
	fn test_join() {
		assert_eq!(
			"/hello/test/world",
			Path::from_str("/hello/test")
				.unwrap()
				.join(&Path::from_str("world").unwrap())
				.unwrap()
				.as_string()
		);
		assert_eq!(
			"/world",
			Path::from_str("/hello/test")
				.unwrap()
				.join(&Path::from_str("/world").unwrap())
				.unwrap()
				.as_string()
		);
	}
}
