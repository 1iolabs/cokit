// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use core::str;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, fmt::Display, ops::Deref};

/// Path.
/// Can be relative or absolute.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Path(str);
impl Path {
	pub fn new(s: &str) -> Result<&Self, PathError> {
		Self::from_str(s)
	}

	pub fn new_unchecked(s: &str) -> &Self {
		Self::from_str_unchecked(s)
	}
}
impl PathExt for Path {
	type PathOwned = PathOwned;
	type Path = Path;

	fn validate(buf: &str) -> Result<(), PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		Ok(())
	}

	fn from_owned_unchecked(buf: String) -> Self::PathOwned {
		PathOwned(buf)
	}

	/// See: [`std::path::Path`]
	fn from_str_unchecked(s: &str) -> &Self::Path {
		unsafe { &*(s as *const str as *const Path) }
	}

	fn as_str(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		matches!(self.as_str().as_bytes().first(), Some(b'/'))
	}
}
impl From<&Path> for String {
	fn from(val: &Path) -> Self {
		val.0.to_owned()
	}
}
impl From<&Path> for PathOwned {
	fn from(val: &Path) -> Self {
		PathOwned(val.0.to_owned())
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
impl PartialEq<str> for Path {
	fn eq(&self, other: &str) -> bool {
		Self::from_str_unchecked(other) == self.as_path()
	}
}
impl Display for Path {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}
impl ToOwned for Path {
	type Owned = PathOwned;

	fn to_owned(&self) -> Self::Owned {
		PathOwned::from_owned_unchecked(self.as_str().to_owned())
	}
}
impl PartialEq<PathOwned> for Path {
	fn eq(&self, other: &PathOwned) -> bool {
		other.as_path() == self.as_path()
	}
}
impl AsRef<AbsolutePath> for Path {
	fn as_ref(&self) -> &AbsolutePath {
		AbsolutePath::from_str_unchecked(&self.0)
	}
}
impl AsRef<RelativePath> for Path {
	fn as_ref(&self) -> &RelativePath {
		RelativePath::from_str_unchecked(&self.0)
	}
}
impl AsRef<Path> for str {
	fn as_ref(&self) -> &Path {
		Path::from_str_unchecked(self)
	}
}
impl PartialEq<AbsolutePath> for Path {
	fn eq(&self, other: &AbsolutePath) -> bool {
		AsRef::<AbsolutePath>::as_ref(&self) == other
	}
}
impl PartialEq<RelativePath> for Path {
	fn eq(&self, other: &RelativePath) -> bool {
		AsRef::<RelativePath>::as_ref(&self) == other
	}
}

/// Owned  Path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PathOwned(String);
impl PathOwned {
	pub fn new(s: String) -> Result<Self, PathError> {
		Self::from_owned(s)
	}

	pub fn new_unchecked(s: String) -> Self {
		Self::from_owned_unchecked(s)
	}
}
impl PathExt for PathOwned {
	type PathOwned = PathOwned;
	type Path = Path;

	fn validate(buf: &str) -> Result<(), PathError> {
		Self::Path::validate(buf)
	}

	fn from_owned_unchecked(buf: String) -> Self::PathOwned {
		PathOwned(buf)
	}

	fn from_str_unchecked(buf: &str) -> &Self::Path {
		Self::Path::from_str_unchecked(buf)
	}

	fn as_str(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		self.as_path().has_root()
	}
}
impl Deref for PathOwned {
	type Target = Path;

	fn deref(&self) -> &Self::Target {
		Path::from_str_unchecked(&self.0)
	}
}
impl AsRef<Path> for PathOwned {
	fn as_ref(&self) -> &Path {
		Path::from_str_unchecked(&self.0)
	}
}
impl PartialEq<Path> for PathOwned {
	fn eq(&self, other: &Path) -> bool {
		other.as_path() == self.as_path()
	}
}
impl Borrow<Path> for PathOwned {
	fn borrow(&self) -> &Path {
		self
	}
}
impl AsRef<str> for PathOwned {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl PartialEq<str> for PathOwned {
	fn eq(&self, other: &str) -> bool {
		Self::from_str_unchecked(other) == self.as_path()
	}
}
impl From<PathOwned> for String {
	fn from(val: PathOwned) -> Self {
		val.0
	}
}
impl<'a> IntoIterator for &'a PathOwned {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl Display for PathOwned {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}
impl From<&AbsolutePath> for PathOwned {
	fn from(value: &AbsolutePath) -> Self {
		Self::new_unchecked(value.to_string())
	}
}
impl From<AbsolutePathOwned> for PathOwned {
	fn from(value: AbsolutePathOwned) -> Self {
		Self::new_unchecked(value.0)
	}
}
impl From<&RelativePath> for PathOwned {
	fn from(value: &RelativePath) -> Self {
		Self::new_unchecked(value.to_string())
	}
}
impl From<RelativePathOwned> for PathOwned {
	fn from(value: RelativePathOwned) -> Self {
		Self::new_unchecked(value.0)
	}
}

/// Absolute Path.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AbsolutePath(str);
impl AbsolutePath {
	pub fn new(s: &str) -> Result<&Self, PathError> {
		Self::from_str(s)
	}

	pub fn new_unchecked(s: &str) -> &Self {
		Self::from_str_unchecked(s)
	}
}
impl PathExt for AbsolutePath {
	type PathOwned = AbsolutePathOwned;
	type Path = AbsolutePath;

	fn validate(buf: &str) -> Result<(), PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		if !matches!(buf.as_bytes().first(), Some(b'/')) {
			return Err(PathError::InvalidArgument);
		}
		Ok(())
	}

	fn from_owned_unchecked(buf: String) -> Self::PathOwned {
		AbsolutePathOwned(buf)
	}

	/// See: [`std::path::Path`]
	fn from_str_unchecked(s: &str) -> &Self::Path {
		unsafe { &*(s as *const str as *const Self::Path) }
	}

	fn as_str(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		true
	}
}
impl From<&AbsolutePath> for String {
	fn from(val: &AbsolutePath) -> Self {
		val.0.to_owned()
	}
}
impl From<&AbsolutePath> for AbsolutePathOwned {
	fn from(val: &AbsolutePath) -> Self {
		val.to_path()
	}
}
impl<'a> IntoIterator for &'a AbsolutePath {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl ToOwned for AbsolutePath {
	type Owned = AbsolutePathOwned;

	fn to_owned(&self) -> Self::Owned {
		AbsolutePathOwned::from_owned_unchecked(self.as_str().to_owned())
	}
}
impl AsRef<str> for AbsolutePath {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}
impl PartialEq<str> for AbsolutePath {
	fn eq(&self, other: &str) -> bool {
		Self::from_str_unchecked(other) == self.as_path()
	}
}
impl Display for AbsolutePath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}
impl PartialEq<AbsolutePathOwned> for AbsolutePath {
	fn eq(&self, other: &AbsolutePathOwned) -> bool {
		other.as_path() == self.as_path()
	}
}
impl AsRef<AbsolutePath> for str {
	fn as_ref(&self) -> &AbsolutePath {
		AbsolutePath::from_str_unchecked(self)
	}
}

/// Owned Absolute Path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct AbsolutePathOwned(String);
impl AbsolutePathOwned {
	pub fn new(s: String) -> Result<Self, PathError> {
		Self::from_owned(s)
	}

	pub fn new_unchecked(s: String) -> Self {
		Self::from_owned_unchecked(s)
	}
}
impl PathExt for AbsolutePathOwned {
	type PathOwned = AbsolutePathOwned;
	type Path = AbsolutePath;

	fn validate(buf: &str) -> Result<(), PathError> {
		Self::Path::validate(buf)
	}

	fn from_owned_unchecked(buf: String) -> Self::PathOwned {
		AbsolutePathOwned(buf)
	}

	/// See: [`std::path::Path`]
	fn from_str_unchecked(buf: &str) -> &Self::Path {
		Self::Path::from_str_unchecked(buf)
	}

	fn as_str(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		self.as_path().has_root()
	}
}
impl Deref for AbsolutePathOwned {
	type Target = AbsolutePath;

	fn deref(&self) -> &Self::Target {
		AbsolutePath::from_str_unchecked(&self.0)
	}
}
impl AsRef<AbsolutePath> for AbsolutePathOwned {
	fn as_ref(&self) -> &AbsolutePath {
		AbsolutePath::from_str_unchecked(&self.0)
	}
}
impl PartialEq<AbsolutePath> for AbsolutePathOwned {
	fn eq(&self, other: &AbsolutePath) -> bool {
		self.0 == other.0
	}
}
impl Borrow<AbsolutePath> for AbsolutePathOwned {
	fn borrow(&self) -> &AbsolutePath {
		self
	}
}
impl TryFrom<String> for AbsolutePathOwned {
	type Error = PathError;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		Self::from_owned(value)
	}
}
impl TryFrom<&str> for AbsolutePathOwned {
	type Error = PathError;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		Self::from_str(value).map(|s| s.to_owned())
	}
}
impl From<AbsolutePathOwned> for String {
	fn from(val: AbsolutePathOwned) -> Self {
		val.0
	}
}
impl<'a> IntoIterator for &'a AbsolutePathOwned {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl AsRef<str> for AbsolutePathOwned {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl PartialEq<str> for AbsolutePathOwned {
	fn eq(&self, other: &str) -> bool {
		Self::from_str_unchecked(other) == self.as_path()
	}
}
impl Display for AbsolutePathOwned {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

/// Relative Path.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RelativePath(str);
impl PathExt for RelativePath {
	type PathOwned = RelativePathOwned;
	type Path = RelativePath;

	fn validate(buf: &str) -> Result<(), PathError> {
		if buf.is_empty() {
			return Err(PathError::InvalidArgument);
		}
		if matches!(buf.as_bytes().first(), Some(b'/')) {
			return Err(PathError::InvalidArgument);
		}
		Ok(())
	}

	fn from_owned_unchecked(buf: String) -> Self::PathOwned {
		RelativePathOwned(buf)
	}

	/// See: [`std::path::Path`]
	fn from_str_unchecked(s: &str) -> &Self::Path {
		unsafe { &*(s as *const str as *const Self::Path) }
	}

	fn as_str(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		false
	}
}
impl From<&RelativePath> for String {
	fn from(val: &RelativePath) -> Self {
		val.0.to_owned()
	}
}
impl From<&RelativePath> for RelativePathOwned {
	fn from(val: &RelativePath) -> Self {
		val.to_path()
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
impl PartialEq<str> for RelativePath {
	fn eq(&self, other: &str) -> bool {
		Self::from_str_unchecked(other) == self.as_path()
	}
}
impl Display for RelativePath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}
impl ToOwned for RelativePath {
	type Owned = RelativePathOwned;

	fn to_owned(&self) -> Self::Owned {
		RelativePathOwned::from_owned_unchecked(self.as_str().to_owned())
	}
}
impl PartialEq<RelativePathOwned> for RelativePath {
	fn eq(&self, other: &RelativePathOwned) -> bool {
		other.as_path() == self.as_path()
	}
}
impl AsRef<RelativePath> for str {
	fn as_ref(&self) -> &RelativePath {
		RelativePath::from_str_unchecked(self)
	}
}

/// OWned Relative Path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RelativePathOwned(String);
impl PathExt for RelativePathOwned {
	type PathOwned = RelativePathOwned;
	type Path = RelativePath;

	fn validate(buf: &str) -> Result<(), PathError> {
		Self::Path::validate(buf)
	}

	fn from_owned_unchecked(buf: String) -> Self::PathOwned {
		RelativePathOwned(buf)
	}

	fn from_str_unchecked(buf: &str) -> &Self::Path {
		Self::Path::from_str_unchecked(buf)
	}

	fn as_str(&self) -> &'_ str {
		&self.0
	}

	fn has_root(&self) -> bool {
		self.as_path().has_root()
	}
}
impl Deref for RelativePathOwned {
	type Target = RelativePath;

	fn deref(&self) -> &Self::Target {
		RelativePath::from_str_unchecked(&self.0)
	}
}
impl AsRef<RelativePath> for RelativePathOwned {
	fn as_ref(&self) -> &RelativePath {
		RelativePath::from_str_unchecked(&self.0)
	}
}
impl PartialEq<RelativePath> for RelativePathOwned {
	fn eq(&self, other: &RelativePath) -> bool {
		other.as_path() == self.as_path()
	}
}
impl Borrow<RelativePath> for RelativePathOwned {
	fn borrow(&self) -> &RelativePath {
		self
	}
}
impl From<RelativePathOwned> for String {
	fn from(val: RelativePathOwned) -> Self {
		val.0
	}
}
impl<'a> IntoIterator for &'a RelativePathOwned {
	type Item = Component<'a>;
	type IntoIter = Components<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.components()
	}
}
impl AsRef<str> for RelativePathOwned {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl PartialEq<str> for RelativePathOwned {
	fn eq(&self, other: &str) -> bool {
		Self::from_str_unchecked(other) == self.as_path()
	}
}
impl Display for RelativePathOwned {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
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

	/// Convert into vector of all names elements.
	pub fn into_vec_normal(self) -> Vec<String> {
		self.filter_map(|component| match component {
			Component::Normal(name) => Some(name.to_owned()),
			_ => None,
		})
		.collect()
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

	/// Test if component is empty.
	pub fn is_empty(&self) -> bool {
		matches!(self, Component::Normal(s) if s.len() == 0)
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

	#[error("No parent: {0}")]
	NoParent(PathOwned),

	#[error("Invalid argument")]
	InvalidArgument,
}

pub trait PathExt {
	type PathOwned: PathExt;
	type Path: PathExt + ?Sized;

	fn validate(buf: &str) -> Result<(), PathError>;
	fn from_owned_unchecked(buf: String) -> Self::PathOwned;
	fn from_str_unchecked(buf: &str) -> &Self::Path;

	fn from_owned(buf: String) -> Result<Self::PathOwned, PathError> {
		Self::validate(&buf)?;
		Ok(Self::from_owned_unchecked(buf))
	}

	fn from_str(buf: &str) -> Result<&Self::Path, PathError> {
		Self::validate(buf)?;
		Ok(Self::from_str_unchecked(buf))
	}

	fn as_path(&self) -> &Self::Path {
		Self::from_str_unchecked(self.as_str())
	}

	fn to_path(&self) -> Self::PathOwned {
		Self::from_owned_unchecked(self.as_str().to_owned())
	}

	fn as_str(&self) -> &str;

	fn has_root(&self) -> bool;

	/// Path components.
	fn components(&self) -> Components<'_> {
		Components { path: self.as_str(), has_root: self.has_root() }
	}

	/// Parent directory.
	fn parent(&self) -> Option<&Self::Path> {
		self.parent_and_file_name().map(|(p, _)| p)
	}

	/// Parent directory.
	fn parent_result(&self) -> Result<&Self::Path, PathError> {
		self.parent()
			.ok_or_else(|| PathError::NoParent(PathOwned::new_unchecked(self.as_str().to_owned())))
	}

	/// Parent directories as full path starting at root.
	///
	/// Example:
	/// ```rust
	/// use co_primitives::{Path, PathExt};
	/// let path = Path::from_str_unchecked("/hello/world/test.zip");
	/// let mut parents = path.parents();
	/// assert_eq!(Some(Path::from_str_unchecked("/")), parents.next());
	/// assert_eq!(Some(Path::from_str_unchecked("/hello")), parents.next());
	/// assert_eq!(Some(Path::from_str_unchecked("/hello/world")), parents.next());
	/// assert_eq!(None, parents.next());
	/// ```
	fn parents(&self) -> impl Iterator<Item = &Self::Path> {
		self.paths().take_while(|path| path.as_str().len() < self.as_str().len())
	}

	/// Components as full path starting at root.
	///
	/// Example:
	/// ```rust
	/// use co_primitives::{Path, PathExt};
	/// let path = Path::from_str_unchecked("/hello/world/test.zip");
	/// let mut paths = path.paths();
	/// assert_eq!(Some(Path::from_str_unchecked("/")), paths.next());
	/// assert_eq!(Some(Path::from_str_unchecked("/hello")), paths.next());
	/// assert_eq!(Some(Path::from_str_unchecked("/hello/world")), paths.next());
	/// assert_eq!(Some(Path::from_str_unchecked("/hello/world/test.zip")), paths.next());
	/// assert_eq!(None, paths.next());
	/// ```
	fn paths(&self) -> impl Iterator<Item = &Self::Path> {
		self.components().scan((0_usize, self.as_str()), |(index, path), component| {
			let end = *index + component.len();
			let result = &path[0..end];
			*index = if *index > 0 { end + 1 } else { end };
			Some(Self::from_str_unchecked(result))
		})
	}

	/// Path and filename.
	fn parent_and_file_name(&self) -> Option<(&Self::Path, &'_ str)> {
		match self.components().last() {
			Some(Component::Normal(name)) => {
				let path = self.as_str();
				let parent = match path.split_at(path.len() - name.len()) {
					("/", _) => "/",
					(p, _) if p.len() > 1 => &p[0..p.len() - 1],
					(p, _) => p,
				};
				Some((Self::from_str_unchecked(parent), name))
			},
			_ => None,
		}
	}

	/// Path and filename.
	fn parent_and_file_name_result(&self) -> Result<(&Self::Path, &str), PathError> {
		self.parent_and_file_name()
			.ok_or_else(|| PathError::NoParent(PathOwned::new_unchecked(self.as_str().to_owned())))
	}

	/// File name.
	fn file_name(&self) -> Option<&str> {
		self.parent_and_file_name().map(|(_, f)| f)
	}

	/// File name.
	fn file_name_result(&self) -> Result<&str, PathError> {
		self.file_name()
			.ok_or_else(|| PathError::NoParent(PathOwned::new_unchecked(self.as_str().to_owned())))
	}

	/// Normalize path to connonized form.
	fn normalize(&self) -> Result<Self::PathOwned, PathError> {
		Ok(Self::from_owned_unchecked(from_components(normalize_components(self.components())?)))
	}

	/// Join and normalize components into an path.
	fn join<'a: 'b, 'b>(
		&'a self,
		other: impl IntoIterator<Item = Component<'b>>,
	) -> Result<Self::PathOwned, PathError> {
		Ok(Self::from_owned_unchecked(join(self.components(), other)?))
	}

	/// Join and normalize other path.
	fn join_path(&self, other: &str) -> Result<Self::PathOwned, PathError> {
		let other_path = Path::from_str_unchecked(other);
		self.join(other_path)
	}
}

fn join<'a: 'b, 'b: 'a>(
	a: impl IntoIterator<Item = Component<'a>>,
	b: impl IntoIterator<Item = Component<'b>>,
) -> Result<String, PathError> {
	let components = a.into_iter().chain(b);
	let normalized = normalize_components(components)?;
	Ok(from_components(normalized))
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
			Component::ParentDir => {
				if index > 0 {
					// check component before
					match stack[index - 1] {
						// fail of we go beyound root
						Component::RootDir => return Err(PathError::NoRoot),
						// keep parent dir if previous is also an parent dir
						Component::ParentDir => {
							index += 1;
							continue;
						},
						_ => {},
					}

					// remove dir and parentdir
					stack.remove(index - 1);
					stack.remove(index - 1);

					// continue with next element
					index -= 1;
				} else {
					// keep parent (..) when on start
					index += 1;
				}
			},
			_ => {
				// keep
				index += 1;
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
	use crate::{AbsolutePath, Component, Path, PathExt, RelativePath};

	#[test]
	fn test_components() {
		let path = Path::from_str("/hello/world").unwrap();
		let mut components = path.components();
		assert_eq!(Some(Component::RootDir), components.next());
		assert_eq!(Some(Component::Normal("hello")), components.next());
		assert_eq!(Some(Component::Normal("world")), components.next());
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_components_empty_component() {
		let path = Path::from_str("/hello//world").unwrap();
		let mut components = path.components();
		assert_eq!(Some(Component::RootDir), components.next());
		assert_eq!(Some(Component::Normal("hello")), components.next());
		assert_eq!(Some(Component::CurDir), components.next());
		assert_eq!(Some(Component::Normal("world")), components.next());
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_components_empty() {
		let path = Path::from_owned_unchecked("".to_owned());
		let mut components = path.components();
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_relative_components() {
		let path = RelativePath::from_str("./hello/world").unwrap();
		let mut components = path.components();
		assert_eq!(Some(Component::CurDir), components.next());
		assert_eq!(Some(Component::Normal("hello")), components.next());
		assert_eq!(Some(Component::Normal("world")), components.next());
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_absolute_components() {
		let path = AbsolutePath::from_str("/hello/world").unwrap();
		let mut components = path.components();
		assert_eq!(Some(Component::RootDir), components.next());
		assert_eq!(Some(Component::Normal("hello")), components.next());
		assert_eq!(Some(Component::Normal("world")), components.next());
		assert_eq!(None, components.next());
	}

	#[test]
	fn test_parents() {
		let path = Path::from_str_unchecked("/hello/world/test.zip");
		let mut parents = path.parents();
		assert_eq!(Some(Path::from_str_unchecked("/")), parents.next());
		assert_eq!(Some(Path::from_str_unchecked("/hello")), parents.next());
		assert_eq!(Some(Path::from_str_unchecked("/hello/world")), parents.next());
		assert_eq!(None, parents.next());
	}

	#[test]
	fn test_paths() {
		let path = Path::from_str_unchecked("/hello/world/test.zip");
		let mut paths = path.paths();
		assert_eq!(Some(Path::from_str_unchecked("/")), paths.next());
		assert_eq!(Some(Path::from_str_unchecked("/hello")), paths.next());
		assert_eq!(Some(Path::from_str_unchecked("/hello/world")), paths.next());
		assert_eq!(Some(Path::from_str_unchecked("/hello/world/test.zip")), paths.next());
		assert_eq!(None, paths.next());
	}

	#[test]
	fn test_normalize() {
		fn normalize(s: &str) -> String {
			Path::from_str(s).unwrap().normalize().unwrap().into()
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
		assert_eq!(Some("test"), Path::from_str("/hello/test").unwrap().file_name());
		assert_eq!(Some("test.zip"), Path::from_str("hello/.././test.zip").unwrap().file_name());
		assert_eq!(None, Path::from_str("hello/.././test.zip/..").unwrap().file_name());
		assert_eq!(None, Path::from_str("/").unwrap().file_name());
	}

	#[test]
	fn test_parent() {
		assert_eq!(Some(Path::from_str_unchecked("/hello")), Path::from_str("/hello/test").unwrap().parent());
		assert_eq!(Some(Path::from_str_unchecked("/")), Path::from_str("/hello").unwrap().parent());
		assert_eq!(None, Path::from_str("/").unwrap().parent());
	}

	#[test]
	fn test_parent_and_file_name() {
		assert_eq!(
			Some((Path::from_str_unchecked("/hello"), "test")),
			Path::from_str("/hello/test").unwrap().parent_and_file_name()
		);
		assert_eq!(
			Some((Path::from_str_unchecked("hello/../."), "test.zip")),
			Path::from_str("hello/.././test.zip").unwrap().parent_and_file_name()
		);
		assert_eq!(None, Path::from_str("hello/.././test.zip/..").unwrap().parent_and_file_name());
		assert_eq!(None, Path::from_str("/").unwrap().parent_and_file_name());
		assert_eq!(
			Some((Path::from_str_unchecked("/"), "test")),
			Path::from_str("/test").unwrap().parent_and_file_name()
		);
	}

	#[test]
	fn test_join() {
		assert_eq!(
			"/hello/test/world",
			Path::from_str("/hello/test")
				.unwrap()
				.join(Path::from_str("world").unwrap())
				.unwrap()
				.as_str()
		);
		assert_eq!(
			"/world",
			Path::from_str("/hello/test")
				.unwrap()
				.join(Path::from_str("/world").unwrap())
				.unwrap()
				.as_str()
		);
	}
}
