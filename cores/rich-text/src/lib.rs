// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use anyhow::anyhow;
use cid::Cid;
use co_api::{
	co, BlockStorage, BlockStorageExt, CoMap, CoreBlockStorage, IsDefault, LazyTransaction, Link, OptionLink, Reducer,
	ReducerAction, TagValue, WeakCid,
};
use futures::{pin_mut, FutureExt, Stream, TryStreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	ops::Range,
};

/// Rich text actions.
#[co]
#[derive(derive_more::From)]
pub enum RichTextAction {
	Insert(InsertAction),
	Delete(DeleteAction),
	Format(FormatAction),
}

#[co]
pub struct InsertAction {
	/// The position to insert.
	#[serde(rename = "l")]
	pub at: InsertionPoint,

	/// The text to insert.
	#[serde(rename = "t")]
	pub text: String,

	/// The attributes.
	#[serde(rename = "a", default, skip_serializing_if = "IsDefault::is_default")]
	pub attributes: AttributesOperation,
}

#[co]
pub struct DeleteAction {
	/// The scalar-start byte position to delete.
	#[serde(rename = "l")]
	pub at: Position,

	/// The inclusive final byte to delete.
	/// If omitted, only the scalar at `at` is deleted.
	#[serde(rename = "r", default, skip_serializing_if = "IsDefault::is_default")]
	pub last: Option<Position>,
}

#[co]
pub struct FormatAction {
	/// The scalar-start byte position to format.
	#[serde(rename = "l")]
	pub at: Position,

	/// The inclusive final byte to format.
	/// If omitted, only the scalar at `at` is formatted.
	#[serde(rename = "r", default, skip_serializing_if = "IsDefault::is_default")]
	pub last: Option<Position>,

	/// The attributes.
	#[serde(rename = "a", default, skip_serializing_if = "IsDefault::is_default")]
	pub attributes: AttributesOperation,
}

#[co]
pub enum AttributesOperation {
	#[serde(rename = "m")]
	Merge(Attributes),
	#[serde(rename = "r")]
	Replace(Attributes),
	#[serde(rename = "x")]
	Remove(BTreeSet<String>),
	#[serde(rename = "d")]
	RemoveAll,
}
impl Default for AttributesOperation {
	fn default() -> Self {
		AttributesOperation::Merge(Default::default())
	}
}

#[co]
pub enum InsertionPoint {
	/// First position.
	#[serde(rename = "l")]
	Start,
	/// Last position.
	#[serde(rename = "r")]
	End,
	/// Before the scalar starting at this byte position.
	#[serde(rename = "b")]
	Before(Position),
	/// After the scalar starting at this byte position, including a deleted scalar.
	#[serde(rename = "a")]
	After(Position),
}
impl InsertionPoint {
	pub fn position(&self, state: &RichText) -> Option<Position> {
		match self {
			InsertionPoint::Start => state.left,
			InsertionPoint::End => state.right,
			InsertionPoint::Before(position) | InsertionPoint::After(position) => Some(*position),
		}
	}
}

/// A UTF-8 byte position within text created by one action.
#[co]
#[derive(Default, Copy)]
pub struct Position(WeakCid, usize);
impl Position {
	/// The preceding byte position.
	pub fn left(&self) -> Option<Self> {
		if self.1 > 0 {
			Some(Self(self.0, self.1 - 1))
		} else {
			None
		}
	}

	/// The following byte position.
	pub fn right(&self) -> Self {
		Self(self.0, self.1 + 1)
	}

	/// The position after `by` bytes.
	pub fn right_by(&self, by: usize) -> Self {
		Self(self.0, self.1 + by)
	}
}

#[co(state)]
pub struct RichText {
	/// First position.
	#[serde(rename = "l", default, skip_serializing_if = "IsDefault::is_default")]
	pub left: Option<Position>,

	/// Last position.
	#[serde(rename = "r", default, skip_serializing_if = "IsDefault::is_default")]
	pub right: Option<Position>,

	/// Runs.
	#[serde(rename = "i", default, skip_serializing_if = "IsDefault::is_default")]
	pub runs: CoMap<Position, Run>,
}
impl Reducer<RichTextAction> for RichText {
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<RichTextAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let event = storage.get_value(&event_link).await?;
		let mut state = storage.get_value_or_default(&state_link).await?;
		// TODO: replace event_link with actual head cid event_link has the risk of duplicates
		reduce(storage, &mut state, event_link.into(), event.payload).await?;
		Ok(storage.set_value(&state).await?)
	}
}
impl RichText {
	// Stream runs.
	pub fn runs<S>(&self, storage: S) -> impl Stream<Item = anyhow::Result<Run>> + use<'_, S>
	where
		S: BlockStorage + Clone + 'static,
	{
		async_stream::try_stream! {
			let runs = self.runs.open(&storage).await?;
			let mut position = match self.left {
				Some(position) => position,
				None => {
					// empty
					return;
				}
			};
			loop {
				let run = runs.get(&position).await?.ok_or(anyhow!("Position not found: {:?}", position))?;
				let run_right = run.right;

				// run
				yield run;

				// next run
				position = match run_right {
					Some(position) => position,
					None => {
						break;
					}
				}
			}
		}
	}

	/// Stream Unicode scalars with UTF-8 byte positions and formatting.
	pub fn chars<S>(
		&self,
		storage: S,
	) -> impl Stream<Item = anyhow::Result<(char, Position, OptionLink<Attributes>)>> + use<'_, S>
	where
		S: BlockStorage + Clone + 'static,
	{
		async_stream::try_stream! {
			let runs = self.runs.open(&storage).await?;
			let mut position = match self.left {
				Some(position) => position,
				None => {
					// empty
					return;
				}
			};
			loop {
				let run = runs.get(&position).await?.ok_or(anyhow!("Position not found: {:?}", position))?;

				// chars
				if !run.deleted {
					for char in run.text.chars() {
						yield (char, position, run.attributes);
						position = position.right_by(char.len_utf8());
					}
				}

				// next run
				position = match run.right {
					Some(position) => position,
					None => {
						break;
					}
				}
			}
		}
	}

	/// Get plain text.
	pub async fn plain_text<S>(&self, storage: &S) -> anyhow::Result<String>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.chars(storage.clone())
			.map_ok(|(char, _, _)| char)
			.try_collect::<String>()
			.await
	}
}

#[co]
pub struct Run {
	#[serde(rename = "i")]
	pub id: Position,
	#[serde(rename = "t", default, skip_serializing_if = "IsDefault::is_default")]
	pub text: String,
	#[serde(rename = "a", default, skip_serializing_if = "IsDefault::is_default")]
	pub attributes: OptionLink<Attributes>,

	#[serde(rename = "l", default, skip_serializing_if = "IsDefault::is_default")]
	pub left: Option<Position>,
	#[serde(rename = "r", default, skip_serializing_if = "IsDefault::is_default")]
	pub right: Option<Position>,

	#[serde(rename = "d", default, skip_serializing_if = "IsDefault::is_default")]
	pub deleted: bool,
}
impl Run {
	/// The first byte in this run.
	pub fn first(&self) -> Position {
		self.id
	}

	/// The last byte in this run.
	pub fn last(&self) -> Position {
		assert!(!self.text.is_empty());
		self.id.right_by(self.text.len() - 1)
	}

	/// The half-open UTF-8 byte range in the originating action.
	pub fn range(&self) -> Range<usize> {
		self.id.1..self.id.1 + self.text.len()
	}

	/// Whether this run contains the byte position.
	pub fn contains(&self, at: Position) -> bool {
		self.id.0 == at.0 && self.range().contains(&at.1)
	}

	fn text_offset(&self, at: Position) -> Option<usize> {
		if self.id.0 != at.0 {
			return None;
		}
		let offset = at.1.checked_sub(self.id.1)?;
		(offset < self.text.len()).then_some(offset)
	}

	fn is_char_start(&self, at: Position) -> bool {
		self.text_offset(at).is_some_and(|offset| self.text.is_char_boundary(offset))
	}

	fn is_char_end(&self, at: Position) -> bool {
		self.text_offset(at)
			.and_then(|offset| offset.checked_add(1))
			.is_some_and(|offset| self.text.is_char_boundary(offset))
	}

	fn char_last(&self, at: Position) -> Option<Position> {
		let offset = self.text_offset(at)?;
		if !self.text.is_char_boundary(offset) {
			return None;
		}
		let char = self.text[offset..].chars().next()?;
		Some(at.right_by(char.len_utf8() - 1))
	}
}

fn action_last(run: &Run, at: Position, last: Option<Position>) -> anyhow::Result<Position> {
	if !run.is_char_start(at) {
		return Err(anyhow!("Invalid range"));
	}
	last.map_or_else(|| run.char_last(at).ok_or_else(|| anyhow!("Invalid range")), Ok)
}

#[co]
#[derive(Default)]
pub struct Attributes {
	pub values: BTreeMap<String, TagValue>,
}
impl Attributes {
	pub fn with_attribute(mut self, name: impl Into<String>, value: impl Into<TagValue>) -> Self {
		self.values.insert(name.into(), value.into());
		self
	}

	pub fn with_merge(mut self, other: Attributes) -> Self {
		self.values.extend(other.values);
		self
	}
}

async fn reduce<S>(storage: &S, state: &mut RichText, head: Cid, action: RichTextAction) -> anyhow::Result<()>
where
	S: BlockStorage + Clone + 'static,
{
	let mut transaction = Transaction::open(storage, state).await?;
	match action {
		RichTextAction::Insert(action) => {
			reduce_text_insert(storage, state, &mut transaction, head, action)
				.boxed()
				.await?
		},
		RichTextAction::Delete(action) => {
			reduce_text_delete(storage, state, &mut transaction, head, action)
				.boxed()
				.await?
		},
		RichTextAction::Format(action) => {
			reduce_text_format(storage, state, &mut transaction, head, action)
				.boxed()
				.await?
		},
	}
	if transaction.runs.is_mut_access() {
		state.runs = transaction.runs.get_mut().await?.store().await?;
	}
	Ok(())
}

struct Transaction<S>
where
	S: BlockStorage + Clone + 'static,
{
	runs: LazyTransaction<S, CoMap<Position, Run>>,
}
impl<S> Transaction<S>
where
	S: BlockStorage + Clone + 'static,
{
	async fn open(storage: &S, state: &RichText) -> anyhow::Result<Self> {
		Ok(Self { runs: state.runs.open_lazy(storage).await? })
	}

	/// Find the run at position.
	pub async fn find_run(&mut self, at: Position) -> anyhow::Result<Option<Run>> {
		let mut position = at;
		let runs = self.runs.get().await?;
		loop {
			if let Some(run) = runs.get(&position).await? {
				return Ok(Some(run));
			} else if let Some(next_position) = position.left() {
				position = next_position;
			} else {
				return Ok(None);
			}
		}
	}

	/// Get the run at position.
	pub async fn get_run(&mut self, at: Position) -> anyhow::Result<Run> {
		self.find_run(at)
			.await?
			.ok_or_else(|| anyhow!("InsertionPoint not found: {:?}", at))
	}
}

async fn reduce_text_insert<S>(
	storage: &S,
	state: &mut RichText,
	transaction: &mut Transaction<S>,
	head: Cid,
	action: InsertAction,
) -> anyhow::Result<()>
where
	S: BlockStorage + Clone + 'static,
{
	if action.text.is_empty() {
		return Err(anyhow!("Invalid insertion: empty text"));
	}
	let insertion_point = normalize_insertion_point(state, action.at);

	// position
	let id = Position(head.into(), 0);

	// verify that position is new
	if transaction.find_run(id).await?.is_some() {
		return Err(anyhow!("Position already exists: {:?}", id));
	}

	// attributes
	let attributes =
		attributes_insertion_point(storage, state, transaction, &insertion_point, &action.attributes).await?;
	let attributes_link = storage.set_value(&attributes).await?.into();

	// create
	let create_run =
		Run { id, text: action.text, attributes: attributes_link, left: None, right: None, deleted: false };

	// find
	match insertion_point {
		InsertionPoint::Start => {
			match state.left {
				// is empty?
				None => {
					// `[]` + [C] = [C]`
					//  ^
					state.left = Some(create_run.id);
					state.right = Some(create_run.last());

					// store
					transaction.runs.get_mut().await?.insert(create_run.id, create_run).await?;
				},
				Some(first) => {
					let first_run = transaction.get_run(first).await?;
					insert_before(storage, state, transaction, create_run, first_run).await?;
				},
			}
		},
		InsertionPoint::End => {
			let last_id = state.right.expect("normalize `at` to start when empty");
			let last_run = transaction.get_run(last_id).await?;
			insert_after(storage, state, transaction, create_run, last_run).await?;
		},
		InsertionPoint::Before(at) => {
			let run = transaction.get_run(at).await?;
			if run.id == at {
				insert_before(storage, state, transaction, create_run, run).await?;
			} else {
				let (_left, right) = split(storage, state, transaction, run, at).await?;
				insert_before(storage, state, transaction, create_run, right).await?;
			}
		},
		InsertionPoint::After(at) => {
			let run = transaction.get_run(at).await?;
			let last = run.char_last(at).ok_or_else(|| anyhow!("Invalid position: {:?}", at))?;
			let run = if run.last() == last {
				run
			} else {
				let (left, right) = split(storage, state, transaction, run, last.right()).await?;
				relink_successor(transaction, &right).await?;
				left
			};
			insert_after(storage, state, transaction, create_run, run).await?;
		},
	}
	Ok(())
}

async fn reduce_text_delete<S>(
	storage: &S,
	state: &mut RichText,
	transaction: &mut Transaction<S>,
	_head: Cid,
	action: DeleteAction,
) -> anyhow::Result<()>
where
	S: BlockStorage + Clone + 'static,
{
	let at = action.at;
	let mut run = transaction.get_run(at).await?;

	// last
	let last = action_last(&run, at, action.last)?;

	// apply
	loop {
		let next_id = run.right;

		// split off left?
		if run.contains(at) && run.id != at {
			let (_left, right) = split(storage, state, transaction, run, at).await?;
			// `run` starts now at `at`
			run = right;
		}

		// slipt off right?
		let is_last = if run.contains(last) {
			if !run.is_char_end(last) {
				return Err(anyhow!("Invalid range"));
			}

			// split
			if run.last() != last {
				let (left, _right) = split(storage, state, transaction, run, last.right()).await?;
				// `run` ends now with `last`
				run = left;
			}

			// this run contains the last position
			true
		} else {
			// does not contain the last position
			false
		};

		// delete
		run.deleted = true;
		transaction.runs.get_mut().await?.insert(run.id, run).await?;

		// next
		if is_last {
			break;
		}
		let next_id = next_id.ok_or_else(|| anyhow!("Invalid range"))?;
		run = transaction.get_run(next_id).await?;
	}

	Ok(())
}

async fn reduce_text_format<S>(
	storage: &S,
	state: &mut RichText,
	transaction: &mut Transaction<S>,
	_head: Cid,
	action: FormatAction,
) -> anyhow::Result<()>
where
	S: BlockStorage + Clone + 'static,
{
	let at = action.at;
	let mut run = transaction.get_run(at).await?;
	let last = action_last(&run, at, action.last)?;

	// attributes
	let attributes = attributes_run(storage, Some(&run), &action.attributes).await?;
	let attributes_link = storage.set_value(&attributes).await?.into();

	// apply
	loop {
		let next_id = run.right;

		// split off left?
		if run.contains(at) && run.id != at {
			let (_left, right) = split(storage, state, transaction, run, at).await?;
			// `run` starts now at `at`
			run = right;
		}

		// slipt off right?
		let is_last = if run.contains(last) {
			if !run.is_char_end(last) {
				return Err(anyhow!("Invalid range"));
			}

			// split
			if run.last() != last {
				let (left, _right) = split(storage, state, transaction, run, last.right()).await?;
				// `run` ends now with `last`
				run = left;
			}

			// this run contains the last position
			true
		} else {
			// does not contain the last position
			false
		};

		// change the whole run
		run.attributes = attributes_link;
		transaction.runs.get_mut().await?.insert(run.id, run).await?;

		// next
		if is_last {
			break;
		}
		let next_id = next_id.ok_or_else(|| anyhow!("Invalid range"))?;
		run = transaction.get_run(next_id).await?;
	}

	Ok(())
}

/// Insert `create_run` before `run`.
///
/// ```text
/// `[A][C] + [B] = [A][B][C]`
///      ^
/// ```
async fn insert_before<S>(
	_storage: &S,
	state: &mut RichText,
	transaction: &mut Transaction<S>,
	mut create_run: Run,
	mut run: Run,
) -> anyhow::Result<()>
where
	S: BlockStorage + Clone + 'static,
{
	// link: create
	create_run.left = run.left;
	create_run.right = Some(run.first());

	// set as new start
	if state.left == Some(run.first()) {
		state.left = Some(create_run.first());
	}

	// link runs
	let left_run = if let Some(left_run) = run.left {
		let mut left_run = transaction.get_run(left_run).await?;
		left_run.right = Some(create_run.first());
		Some(left_run)
	} else {
		None
	};
	run.left = Some(create_run.first());

	// store (left, create, right)
	if let Some(left_run) = left_run {
		transaction.runs.get_mut().await?.insert(left_run.id, left_run).await?;
	}
	transaction.runs.get_mut().await?.insert(create_run.id, create_run).await?;
	transaction.runs.get_mut().await?.insert(run.id, run).await?;

	Ok(())
}

/// Insert `create_run` after `run`.
///
/// ```text
/// `[A][B][D] + [C] = [A][B][C][D]`
///      ^
/// ```
async fn insert_after<S>(
	_storage: &S,
	state: &mut RichText,
	transaction: &mut Transaction<S>,
	mut create_run: Run,
	mut run: Run,
) -> anyhow::Result<()>
where
	S: BlockStorage + Clone + 'static,
{
	// link: create
	create_run.left = Some(run.last());
	create_run.right = run.right;

	// set as new end
	if state.right == Some(run.last()) {
		state.right = Some(create_run.last());
	}

	// link runs
	let right_run = if let Some(right_run) = run.right {
		let mut left_run = transaction.get_run(right_run).await?;
		left_run.left = Some(create_run.last());
		Some(left_run)
	} else {
		None
	};
	run.right = Some(create_run.first());

	// store (left, create, right)
	transaction.runs.get_mut().await?.insert(run.id, run).await?;
	transaction.runs.get_mut().await?.insert(create_run.id, create_run).await?;
	if let Some(right_run) = right_run {
		transaction.runs.get_mut().await?.insert(right_run.id, right_run).await?;
	}

	Ok(())
}

/// Point the run following `run` back at the last byte of `run`.
async fn relink_successor<S>(transaction: &mut Transaction<S>, run: &Run) -> anyhow::Result<()>
where
	S: BlockStorage + Clone + 'static,
{
	if let Some(successor) = run.right {
		let mut successor = transaction.get_run(successor).await?;
		if successor.left != Some(run.last()) {
			successor.left = Some(run.last());
			transaction.runs.get_mut().await?.insert(successor.id, successor).await?;
		}
	}
	Ok(())
}

/// Split `run` before `at`.
/// - Does not change text or identifiers just replaces one run with two.
/// - Does not delete: `run` is reused as left.
async fn split<S>(
	_storage: &S,
	_state: &mut RichText,
	transaction: &mut Transaction<S>,
	run: Run,
	at: Position,
) -> anyhow::Result<(Run, Run)>
where
	S: BlockStorage + Clone + 'static,
{
	// validate
	let text_offset = run
		.text_offset(at)
		.filter(|offset| run.text.is_char_boundary(*offset))
		.ok_or_else(|| anyhow!("Invalid range"))?;

	// text
	let text_left = run.text[0..text_offset].to_owned();
	let text_right = run.text[text_offset..].to_owned();

	// use run as left
	let mut left = run.clone();
	left.text = text_left;
	left.right = Some(at);

	// insert right
	let mut right = run.clone();
	right.text = text_right;
	right.id = at;
	right.left = Some(left.last());

	// store
	transaction.runs.get_mut().await?.insert(left.id, left.clone()).await?;
	transaction.runs.get_mut().await?.insert(right.id, right.clone()).await?;

	// result
	Ok((left, right))
}

fn normalize_insertion_point(state: &RichText, at: InsertionPoint) -> InsertionPoint {
	match at {
		InsertionPoint::Start => InsertionPoint::Start,
		InsertionPoint::End => {
			if state.left.is_none() {
				InsertionPoint::Start
			} else {
				InsertionPoint::End
			}
		},
		InsertionPoint::Before(position) => {
			if Some(position) == state.left {
				InsertionPoint::Start
			} else {
				InsertionPoint::Before(position)
			}
		},
		InsertionPoint::After(position) => InsertionPoint::After(position),
	}
}

/// Get attributes for `AttributesOperation` at `insertion_point`.
async fn attributes_insertion_point<S>(
	storage: &S,
	state: &RichText,
	transaction: &mut Transaction<S>,
	insertion_point: &InsertionPoint,
	attributes: &AttributesOperation,
) -> anyhow::Result<Attributes>
where
	S: BlockStorage + Clone + 'static,
{
	Ok(match attributes {
		AttributesOperation::Merge(_) | AttributesOperation::Remove(_) => {
			let run = if let Some(position) = insertion_point.position(state) {
				Some(transaction.get_run(position).await?)
			} else {
				None
			};
			attributes_run(storage, run.as_ref(), attributes).await?
		},
		AttributesOperation::Replace(attributes) => attributes.clone(),
		AttributesOperation::RemoveAll => Default::default(),
	})
}

/// Get attributes for `AttributesOperation` at `insertion_point`.
async fn attributes_run<S>(
	storage: &S,
	run: Option<&Run>,
	attributes: &AttributesOperation,
) -> anyhow::Result<Attributes>
where
	S: BlockStorage + Clone + 'static,
{
	Ok(match attributes {
		AttributesOperation::Merge(attributes) => {
			if let Some(run) = run {
				let mut run_attributes = storage.get_value_or_default(&run.attributes).await?;
				run_attributes.values.extend(attributes.values.clone());
				run_attributes
			} else {
				attributes.clone()
			}
		},
		AttributesOperation::Replace(attributes) => attributes.clone(),
		AttributesOperation::Remove(attribute_names) => {
			if let Some(run) = run {
				let mut run_attributes = storage.get_value_or_default(&run.attributes).await?;
				for attribute_name in attribute_names {
					run_attributes.values.remove(attribute_name);
				}
				run_attributes
			} else {
				Default::default()
			}
		},
		AttributesOperation::RemoveAll => Default::default(),
	})
}

/// A UTF-8 byte-indexed view of rich text.
pub struct TextModel<S> {
	storage: S,
	state: OptionLink<RichText>,
}
impl<S> TextModel<S>
where
	S: BlockStorage + Clone + 'static,
{
	/// A model over `state`; an absent state is an empty document.
	pub fn new(storage: S, state: OptionLink<RichText>) -> Self {
		Self { storage, state }
	}

	pub async fn plain_text(&self) -> anyhow::Result<String> {
		if let Some(state) = self.storage.get_value_or_none(&self.state).await? {
			Ok(state.plain_text(&self.storage).await?)
		} else {
			Ok(String::new())
		}
	}

	pub fn runs(&self) -> impl Stream<Item = Result<(String, Attributes), anyhow::Error>> + use<S> {
		let storage = self.storage.clone();
		let state = self.state;
		async_stream::try_stream! {
			if let Some(state) = storage.get_value_or_none(&state).await? {
				let mut last_attributes = OptionLink::none();
				let mut text = String::new();

				// characters
				for await item in state.chars(storage.clone()) {
					let (char, _position, attributes) = item?;

					// next run?
					if last_attributes != attributes {
						if !text.is_empty() {
							let run_text = text;
							let run_attributes = storage.get_value_or_default(&last_attributes).await?;
							yield (run_text, run_attributes);
							text = String::new();
						}
						last_attributes = attributes;
					}

					// append
					text.push(char);
				}

				// last run?
				if !text.is_empty() {
					let run_text = text;
					let run_attributes = storage.get_value_or_default(&last_attributes).await?;
					yield (run_text, run_attributes);
				}
			}
		}
	}

	/// UTF-8 byte index for a scalar-start position.
	pub async fn index(&self, at: &Position) -> anyhow::Result<usize> {
		self.position_index(at, false, false).await
	}

	/// Index of text inserted before the scalar or empty anchor at `at`.
	async fn insertion_index(&self, at: &Position) -> anyhow::Result<usize> {
		self.position_index(at, true, false).await
	}

	/// Index of text inserted after the scalar at `at`; a deleted scalar contributes no bytes.
	async fn after_index(&self, at: &Position) -> anyhow::Result<usize> {
		self.position_index(at, false, true).await
	}

	async fn position_index(
		&self,
		at: &Position,
		include_empty_anchor: bool,
		include_scalar: bool,
	) -> anyhow::Result<usize> {
		let state = self.storage.get_value_or_default(&self.state).await?;

		// walk runs
		let mut index = 0;
		let runs = state.runs(self.storage.clone());
		pin_mut!(runs);
		while let Some(run) = runs.try_next().await? {
			if include_empty_anchor && run.id == *at && run.text.is_empty() {
				return Ok(index);
			}
			// done?
			if run.contains(*at) {
				if !run.is_char_start(*at) {
					return Err(anyhow!("Invalid position: {:?}", at));
				}
				if run.deleted {
					return Ok(index);
				}
				let offset = at.1 - run.id.1;
				let scalar =
					if include_scalar { run.text[offset..].chars().next().map_or(0, char::len_utf8) } else { 0 };
				return Ok(index + offset + scalar);
			}

			// index
			if !run.deleted {
				index += run.text.len();
			}
		}
		Err(anyhow!("Position not found: {:?}", at))
	}

	/// Half-open UTF-8 byte range for action positions.
	pub async fn range(&self, at: &Position, last: &Option<Position>) -> anyhow::Result<Range<usize>> {
		let state = self.storage.get_value_or_default(&self.state).await?;

		// walk runs
		let mut index = 0;
		let mut start_found = false;
		let mut start = 0;
		let mut last = *last;
		let runs = state.runs(self.storage.clone());
		pin_mut!(runs);
		while let Some(run) = runs.try_next().await? {
			// done?
			if !start_found && run.contains(*at) {
				if !run.is_char_start(*at) {
					return Err(anyhow!("Invalid position: {:?}", at));
				}
				start = if !run.deleted { index + at.1 - run.id.1 } else { index };
				start_found = true;
				if last.is_none() {
					last = run.char_last(*at);
				} else if last.is_some_and(|last| run.contains(last) && last.1 < at.1) {
					return Err(anyhow!("Invalid range"));
				}
			}
			if start_found {
				let last = last.ok_or_else(|| anyhow!("Invalid range"))?;
				if run.contains(last) {
					if !run.is_char_end(last) {
						return Err(anyhow!("Invalid position: {:?}", last));
					}
					return Ok(Range { start, end: if !run.deleted { index + last.1 - run.id.1 + 1 } else { index } });
				}
			}

			// index
			if !run.deleted {
				index += run.text.len();
			}
		}
		if !start_found {
			return Err(anyhow!("Position not found: {:?}", at));
		}
		Err(anyhow!("Position not found: {:?}", last))
	}

	/// Scalar-start position at a UTF-8 byte index.
	pub async fn position(&self, index: usize) -> anyhow::Result<Option<Position>> {
		let state = self.storage.get_value_or_default(&self.state).await?;
		let mut current = 0;
		let chars = state.chars(self.storage.clone());
		pin_mut!(chars);
		while let Some((char, position, _attributes)) = chars.try_next().await? {
			if current == index {
				return Ok(Some(position));
			}
			current += char.len_utf8();
			if index < current {
				return Err(anyhow!("Invalid index: {}", index));
			}
		}
		if current == index {
			Ok(None)
		} else {
			Err(anyhow!("Index not found: {}", index))
		}
	}

	/// Action positions for a half-open UTF-8 byte range.
	pub async fn position_range(&self, range: &Range<usize>) -> anyhow::Result<(Option<Position>, Option<Position>)> {
		// validate
		if range.is_empty() {
			return Err(anyhow!("Invalid range: {:?}", range));
		}

		// find positions
		let state = self.storage.get_value_or_default(&self.state).await?;
		let mut index = 0;
		let mut at = None;
		let mut scalar_count = 0;
		let chars = state.chars(self.storage.clone());
		pin_mut!(chars);
		while let Some((char, position, _attributes)) = chars.try_next().await? {
			let end = index + char.len_utf8();
			if at.is_none() {
				if range.start == index {
					at = Some(position);
				} else if range.start < end {
					return Err(anyhow!("Invalid range: {:?}", range));
				}
			}
			if at.is_some() {
				if range.end < end {
					return Err(anyhow!("Invalid range: {:?}", range));
				}
				scalar_count += 1;
				let last = position.right_by(char.len_utf8() - 1);
				if range.end == end {
					return Ok((at, if scalar_count == 1 { None } else { Some(last) }));
				}
			}
			index = end;
		}
		Err(anyhow!("Invalid range: {:?}", range))
	}

	/// Insert action for `text` typed at a UTF-8 byte index.
	/// The text anchors to the preceding scalar and carries the attributes resolved at the cursor.
	/// `text` must not be empty.
	pub async fn insert(
		&self,
		index: usize,
		text: String,
		attributes: AttributesOperation,
	) -> anyhow::Result<RichTextAction> {
		if text.is_empty() {
			return Err(anyhow!("Invalid insertion: empty text"));
		}
		let state = self.storage.get_value_or_default(&self.state).await?;
		let (previous, current) = self.scalars_around(&state, index).await?;
		let cursor = current.map_or(InsertionPoint::End, InsertionPoint::Before);
		let at = match (previous, current) {
			(Some(previous), _) => InsertionPoint::After(previous),
			(None, Some(current)) => InsertionPoint::Before(current),
			(None, None) if state.left.is_none() => InsertionPoint::Start,
			(None, None) => InsertionPoint::End,
		};
		let mut transaction = Transaction::open(&self.storage, &state).await?;
		let attributes =
			attributes_insertion_point(&self.storage, &state, &mut transaction, &cursor, &attributes).await?;
		Ok(InsertAction { at, text, attributes: AttributesOperation::Replace(attributes) }.into())
	}

	/// Start positions of the scalar before and of the scalar at a UTF-8 byte index.
	async fn scalars_around(
		&self,
		state: &RichText,
		index: usize,
	) -> anyhow::Result<(Option<Position>, Option<Position>)> {
		let mut current = 0;
		let mut previous = None;
		let chars = state.chars(self.storage.clone());
		pin_mut!(chars);
		while let Some((char, position, _attributes)) = chars.try_next().await? {
			if current == index {
				return Ok((previous, Some(position)));
			}
			current += char.len_utf8();
			if index < current {
				return Err(anyhow!("Invalid index: {}", index));
			}
			previous = Some(position);
		}
		if current != index {
			return Err(anyhow!("Index not found: {}", index));
		}
		Ok((previous, None))
	}

	pub async fn delete(&self, range: Range<usize>) -> anyhow::Result<RichTextAction> {
		// range
		let (at, last) = self.position_range(&range).await?;
		let at = at.ok_or_else(|| anyhow!("Index not found: {}", range.start))?;

		// result
		Ok(DeleteAction { at, last }.into())
	}

	pub async fn format(&self, range: Range<usize>, attributes: AttributesOperation) -> anyhow::Result<RichTextAction> {
		// range
		let (at, last) = self.position_range(&range).await?;
		let at = at.ok_or_else(|| anyhow!("Index not found: {}", range.start))?;

		// result
		Ok(FormatAction { at, last, attributes }.into())
	}

	pub async fn text_change(&self, actions: &[RichTextAction]) -> anyhow::Result<Vec<TextModelChange>> {
		let mut result = Vec::new();
		let state = self.storage.get_value_or_default(&self.state).await?;
		let mut transaction = Transaction::open(&self.storage, &state).await?;
		for action in actions {
			match action {
				RichTextAction::Insert(action) => {
					if action.text.is_empty() {
						return Err(anyhow!("Invalid insertion: empty text"));
					}
					let insertion_point = normalize_insertion_point(&state, action.at.clone());
					let index = match &insertion_point {
						InsertionPoint::Start => 0,
						InsertionPoint::End => {
							let mut index = 0;
							let runs = state.runs(self.storage.clone());
							pin_mut!(runs);
							while let Some(run) = runs.try_next().await? {
								if !run.deleted {
									index += run.text.len();
								}
							}
							index
						},
						InsertionPoint::Before(position) => self.insertion_index(position).await?,
						InsertionPoint::After(position) => self.after_index(position).await?,
					};
					let attributes = attributes_insertion_point(
						&self.storage,
						&state,
						&mut transaction,
						&insertion_point,
						&action.attributes,
					)
					.await?;
					result.push(TextModelChange::Insert { index, text: action.text.clone(), attributes });
				},
				RichTextAction::Delete(action) => {
					let range = self.range(&action.at, &action.last).await?;
					if !range.is_empty() {
						result.push(TextModelChange::Delete { range });
					}
				},
				RichTextAction::Format(action) => {
					let range = self.range(&action.at, &action.last).await?;
					if !range.is_empty() {
						let attributes = attributes_insertion_point(
							&self.storage,
							&state,
							&mut transaction,
							&InsertionPoint::Before(action.at),
							&action.attributes,
						)
						.await?;
						result.push(TextModelChange::Format { range, attributes });
					}
				},
			}
		}
		Ok(result)
	}
}

#[derive(Debug, Clone)]
pub enum TextModelChange {
	Insert { index: usize, text: String, attributes: Attributes },
	Delete { range: Range<usize> },
	Format { range: Range<usize>, attributes: Attributes },
}

#[cfg(test)]
mod tests {
	use crate::{
		Attributes, AttributesOperation, DeleteAction, FormatAction, InsertAction, InsertionPoint, Position, RichText,
		RichTextAction, Run, TextModel, TextModelChange,
	};
	use cid::Cid;
	use co_api::{
		from_cbor, to_cbor, BlockStorage, BlockStorageExt, CoMap, CoTryStreamExt, CoreBlockStorage, Date, Link,
		OptionLink, Reducer, ReducerAction,
	};
	use co_identity::{Identity, IdentityResolver, LocalIdentity, LocalIdentityResolver};
	use co_log::{IdentityEntryVerifier, Log};
	use co_storage::MemoryBlockStorage;
	use futures::{FutureExt, StreamExt, TryStreamExt};
	use std::collections::{BTreeMap, BTreeSet};

	async fn apply<S>(
		storage: &S,
		state: RichText,
		action_link: Link<ReducerAction<RichTextAction>>,
	) -> anyhow::Result<RichText>
	where
		S: BlockStorage + Clone + 'static,
	{
		let state_link = storage.set_value(&state).await?;
		let next_state_link =
			RichText::reduce(state_link.into(), action_link, &CoreBlockStorage::new(storage.clone(), true))
				.boxed()
				.await?;
		Ok(storage.get_value(&next_state_link).await?)
	}

	async fn try_dispatch<S>(
		storage: &S,
		time: &mut Date,
		state: RichText,
		action: impl Into<RichTextAction>,
	) -> anyhow::Result<RichText>
	where
		S: BlockStorage + Clone + 'static,
	{
		let action = ReducerAction { core: "".to_owned(), from: "".to_owned(), payload: action.into(), time: *time };
		*time += 1;
		let action_link = storage.set_value(&action).await?;
		apply(storage, state, action_link).await
	}

	async fn dispatch<S>(storage: &S, time: &mut Date, state: RichText, action: impl Into<RichTextAction>) -> RichText
	where
		S: BlockStorage + Clone + 'static,
	{
		try_dispatch(storage, time, state, action).await.unwrap()
	}

	async fn text_model(storage: &MemoryBlockStorage, state: &RichText) -> TextModel<MemoryBlockStorage> {
		let state = storage.set_value(state).await.unwrap();
		TextModel { storage: storage.clone(), state: state.into() }
	}

	// the state an earlier core stored for an empty insertion before the scalar at `offset`:
	// one run split around an inherited empty run
	async fn inherited_empty_anchor(
		storage: &MemoryBlockStorage,
		text: &str,
		first: Position,
		offset: usize,
	) -> (RichText, Position) {
		let anchor = Position((*storage.set_value(&"anchor".to_owned()).await.unwrap().cid()).into(), 0);
		let left = Run {
			id: first,
			text: text[..offset].to_owned(),
			attributes: Default::default(),
			left: None,
			right: Some(anchor),
			deleted: false,
		};
		let empty = Run {
			id: anchor,
			text: String::new(),
			attributes: Default::default(),
			left: Some(first.right_by(offset - 1)),
			right: Some(first.right_by(offset)),
			deleted: false,
		};
		let right = Run {
			id: first.right_by(offset),
			text: text[offset..].to_owned(),
			attributes: Default::default(),
			left: Some(anchor),
			right: None,
			deleted: false,
		};
		let runs = CoMap::from_iter(storage, [(first, left), (anchor, empty), (first.right_by(offset), right)])
			.await
			.unwrap();
		let state = RichText { left: Some(first), right: Some(first.right_by(text.len() - 1)), runs };
		(state, anchor)
	}

	// the reducer rejects `action` and leaves `state` untouched
	async fn assert_rejected(storage: &MemoryBlockStorage, state: &RichText, action: impl Into<RichTextAction>) {
		let mut applied = state.clone();
		let result = crate::reduce(storage, &mut applied, Cid::default(), action.into())
			.boxed()
			.await;
		assert!(result.is_err());
		assert_eq!(to_cbor(&applied).unwrap(), to_cbor(state).unwrap());
	}

	async fn scalar_positions(storage: &MemoryBlockStorage, state: &RichText) -> Vec<Position> {
		state
			.chars(storage.clone())
			.map_ok(|(_char, position, _attributes)| position)
			.try_collect::<Vec<_>>()
			.await
			.unwrap()
	}

	// the index the model predicts for a single insert action
	async fn insert_index(model: &TextModel<MemoryBlockStorage>, action: &RichTextAction) -> usize {
		match model.text_change(std::slice::from_ref(action)).await.unwrap().as_slice() {
			[TextModelChange::Insert { index, .. }] => *index,
			changes => panic!("expected one insert change: {changes:?}"),
		}
	}

	fn signed_log(heads: BTreeSet<Cid>) -> Log {
		Log::new(b"rich-text".to_vec(), IdentityEntryVerifier::new(LocalIdentityResolver::new().boxed()), heads)
	}

	fn identity(name: &str) -> LocalIdentity {
		LocalIdentityResolver::new()
			.private_identity(&format!("did:local:{name}"))
			.unwrap()
	}

	// replay a log oldest entry first, as the application does
	async fn replay(storage: &MemoryBlockStorage, log: &Log) -> (RichText, Cid) {
		let mut entries = log.stream(storage).try_collect::<Vec<_>>().await.unwrap();
		entries.reverse();
		let mut state = RichText::default();
		for entry in entries {
			state = apply(storage, state, Link::new(entry.entry().payload)).await.unwrap();
		}
		let cid = *storage.set_value(&state).await.unwrap().cid();
		(state, cid)
	}

	// author `text` scalar by scalar at `index`, like forward typing, pushing each action to `log`
	async fn type_text(
		storage: &MemoryBlockStorage,
		log: &mut Log,
		identity: &LocalIdentity,
		time: &mut Date,
		mut index: usize,
		text: &str,
		attributes: AttributesOperation,
	) {
		for char in text.chars() {
			let (state, _cid) = replay(storage, log).await;
			let model = text_model(storage, &state).await;
			let action = model.insert(index, char.to_string(), attributes.clone()).await.unwrap();
			assert_eq!(insert_index(&model, &action).await, index);
			let mut expected = state.plain_text(storage).await.unwrap();
			expected.insert(index, char);
			let action = ReducerAction {
				core: "rich-text".to_owned(),
				from: identity.identity().to_owned(),
				payload: action,
				time: *time,
			};
			*time += 1;
			log.push_event(storage, identity, &action).await.unwrap();
			let (state, _cid) = replay(storage, log).await;
			assert_eq!(state.plain_text(storage).await.unwrap(), expected);
			index += char.len_utf8();
		}
	}

	// two peers fork from `shared`, type at the same `index` while disconnected, then merge in both directions
	async fn merge_typing(
		storage: &MemoryBlockStorage,
		time: &mut Date,
		shared: &str,
		index: usize,
		(identity_a, text_a): (&LocalIdentity, &str),
		(identity_b, text_b): (&LocalIdentity, &str),
	) -> RichText {
		let mut log_a = signed_log(Default::default());
		type_text(storage, &mut log_a, identity_a, time, 0, shared, Default::default()).await;
		let mut log_b = signed_log(log_a.heads().clone());
		type_text(storage, &mut log_a, identity_a, time, index, text_a, Default::default()).await;
		type_text(storage, &mut log_b, identity_b, time, index, text_b, Default::default()).await;

		let mut merged_a = log_a.clone();
		assert!(merged_a.join(storage, &log_b).await.unwrap());
		let mut merged_b = log_b.clone();
		assert!(merged_b.join(storage, &log_a).await.unwrap());
		assert_eq!(merged_a.heads(), merged_b.heads());
		let (state, cid) = replay(storage, &merged_a).await;
		assert_eq!(replay(storage, &merged_b).await.1, cid);
		assert_eq!(replay(storage, &signed_log(merged_a.heads().clone())).await.1, cid);
		assert_linked(storage, &state).await;
		state
	}

	// every run links to its neighbors' first and last bytes and the ends match the state
	async fn assert_linked(storage: &MemoryBlockStorage, state: &RichText) {
		let runs = state.runs(storage.clone()).try_collect::<Vec<_>>().await.unwrap();
		assert_eq!(runs.first().map(Run::first), state.left);
		assert_eq!(runs.first().and_then(|run| run.left), None);
		assert_eq!(runs.last().and_then(|run| run.right), None);
		assert_eq!(runs.last().map(Run::last), state.right);
		for pair in runs.windows(2) {
			assert_eq!(pair[0].right, Some(pair[1].first()));
			assert_eq!(pair[1].left, Some(pair[0].last()));
		}
	}

	#[test]
	fn test_run() {
		let head = Cid::default().into();
		let run = Run {
			id: Position(head, 0),
			text: "hello".to_owned(),
			attributes: Default::default(),
			deleted: false,
			left: None,
			right: None,
		};
		assert_eq!(run.first(), Position(head, 0));
		assert_eq!(run.last(), Position(head, 4));
		assert_eq!(run.range(), 0..5);
		assert!(run.contains(Position(head, 0)));
		assert!(run.contains(Position(head, 1)));
		assert!(run.contains(Position(head, 2)));
		assert!(run.contains(Position(head, 3)));
		assert!(run.contains(Position(head, 4)));
		assert!(!run.contains(Position(head, 5)));

		let run = Run { text: "Aé中😀B".to_owned(), ..run };
		assert_eq!(run.last(), Position(head, 10));
		assert_eq!(run.range(), 0..11);
		for offset in [0, 1, 3, 6, 10] {
			assert!(run.is_char_start(Position(head, offset)));
		}
		for offset in [0, 2, 5, 9, 10] {
			assert!(run.is_char_end(Position(head, offset)));
		}
	}

	#[tokio::test]
	async fn test_utf8_byte_positions_and_split_boundaries() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let text = "Aé中😀B";
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: text.to_owned() },
		)
		.await;
		let characters = state
			.chars(storage.clone())
			.map_ok(|(char, position, _attributes)| (char, position.1))
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		assert_eq!(characters, vec![('A', 0), ('é', 1), ('中', 3), ('😀', 6), ('B', 10)]);

		let id = state.left.unwrap();
		for offset in [0, 1, 3, 6, 10] {
			let mut expected = text.to_owned();
			expected.insert(offset, '|');
			let next = dispatch(
				&storage,
				&mut time,
				state.clone(),
				InsertAction {
					at: InsertionPoint::Before(id.right_by(offset)),
					attributes: Default::default(),
					text: "|".to_owned(),
				},
			)
			.await;
			assert_eq!(next.plain_text(&storage).await.unwrap(), expected);
		}
		let next = dispatch(
			&storage,
			&mut time,
			state.clone(),
			InsertAction { at: InsertionPoint::End, attributes: Default::default(), text: "|".to_owned() },
		)
		.await;
		assert_eq!(next.plain_text(&storage).await.unwrap(), "Aé中😀B|");

		for offset in [2, 4, 5, 7, 8, 9, 11] {
			let result = try_dispatch(
				&storage,
				&mut time,
				state.clone(),
				InsertAction {
					at: InsertionPoint::Before(id.right_by(offset)),
					attributes: Default::default(),
					text: "|".to_owned(),
				},
			)
			.await;
			assert!(result.is_err(), "offset {offset} must be rejected");
		}
		assert_eq!(state.plain_text(&storage).await.unwrap(), text);
	}

	#[tokio::test]
	async fn test_omitted_last_affects_one_utf8_scalar() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let text = "Aé中😀B";
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: text.to_owned() },
		)
		.await;
		let characters = state
			.chars(storage.clone())
			.map_ok(|(char, position, attributes)| (char, position, attributes))
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		let base_attributes = characters[0].2;
		let marked = Attributes::default().with_attribute("marked", true);
		let marked_link = storage.set_value(&marked).await.unwrap().into();

		for (target, (char, position, _attributes)) in characters.iter().enumerate() {
			let mut expected = text.to_owned();
			expected.replace_range(position.1..position.1 + char.len_utf8(), "");
			let deleted =
				dispatch(&storage, &mut time, state.clone(), DeleteAction { at: *position, last: None }).await;
			assert_eq!(deleted.plain_text(&storage).await.unwrap(), expected);

			let formatted = dispatch(
				&storage,
				&mut time,
				state.clone(),
				FormatAction { at: *position, last: None, attributes: AttributesOperation::Merge(marked.clone()) },
			)
			.await;
			let attributes = formatted
				.chars(storage.clone())
				.map_ok(|(_char, _position, attributes)| attributes)
				.try_collect::<Vec<_>>()
				.await
				.unwrap();
			for (index, attributes) in attributes.into_iter().enumerate() {
				assert_eq!(attributes, if index == target { marked_link } else { base_attributes });
			}
		}
	}

	#[tokio::test]
	async fn test_utf8_ranges_cross_tombstones() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "Aé中😀B".to_owned() },
		)
		.await;
		let positions = state
			.chars(storage.clone())
			.map_ok(|(_char, position, _attributes)| position)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		let last = positions[3].right_by('😀'.len_utf8() - 1);

		let deleted =
			dispatch(&storage, &mut time, state.clone(), DeleteAction { at: positions[1], last: Some(last) }).await;
		assert_eq!(deleted.plain_text(&storage).await.unwrap(), "AB");

		let tombstoned = dispatch(&storage, &mut time, state, DeleteAction { at: positions[2], last: None }).await;
		let deleted =
			dispatch(&storage, &mut time, tombstoned, DeleteAction { at: positions[1], last: Some(last) }).await;
		assert_eq!(deleted.plain_text(&storage).await.unwrap(), "AB");
	}

	#[tokio::test]
	async fn test_invalid_utf8_ranges_leave_state_unchanged() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let text = "Aé中😀B";
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: text.to_owned() },
		)
		.await;
		let id = state.left.unwrap();
		let actions = [
			RichTextAction::Delete(DeleteAction { at: id.right_by(2), last: None }),
			RichTextAction::Delete(DeleteAction { at: id.right_by(1), last: Some(id.right_by(11)) }),
			RichTextAction::Format(FormatAction {
				at: id.right_by(1),
				last: Some(id.right_by(7)),
				attributes: AttributesOperation::RemoveAll,
			}),
			RichTextAction::Format(FormatAction {
				at: id.right_by(3),
				last: Some(id),
				attributes: AttributesOperation::RemoveAll,
			}),
		];

		for action in actions {
			assert!(try_dispatch(&storage, &mut time, state.clone(), action).await.is_err());
			assert_eq!(state.plain_text(&storage).await.unwrap(), text);
		}
	}

	#[tokio::test]
	async fn test_concurrent_utf8_insert_delete_is_deterministic() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "é中".to_owned() },
		)
		.await;
		let positions = state
			.chars(storage.clone())
			.map_ok(|(_char, position, _attributes)| position)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		let insert = ReducerAction {
			core: "".to_owned(),
			from: "insert".to_owned(),
			payload: InsertAction {
				at: InsertionPoint::Before(positions[1]),
				attributes: Default::default(),
				text: "😀".to_owned(),
			}
			.into(),
			time: 10,
		};
		let delete = ReducerAction {
			core: "".to_owned(),
			from: "delete".to_owned(),
			payload: DeleteAction { at: positions[0], last: None }.into(),
			time: 10,
		};
		let insert_link = storage.set_value(&insert).await.unwrap();
		let delete_link = storage.set_value(&delete).await.unwrap();

		let insert_delete = apply(&storage, state.clone(), insert_link).await.unwrap();
		let insert_delete = apply(&storage, insert_delete, delete_link).await.unwrap();
		let delete_insert = apply(&storage, state.clone(), delete_link).await.unwrap();
		let delete_insert = apply(&storage, delete_insert, insert_link).await.unwrap();

		assert_eq!(insert_delete.plain_text(&storage).await.unwrap(), "😀中");
		assert_eq!(delete_insert.plain_text(&storage).await.unwrap(), "😀中");
		let insert_delete_runs = insert_delete.runs(storage.clone()).try_collect::<Vec<_>>().await.unwrap();
		let delete_insert_runs = delete_insert.runs(storage.clone()).try_collect::<Vec<_>>().await.unwrap();
		assert_eq!(insert_delete_runs, delete_insert_runs);
	}

	#[tokio::test]
	async fn test_insert() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;

		let state = RichText::default();
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "");

		// insert at start
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "hello".to_owned() },
		)
		.await;
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "hello");

		// insert at end
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction { at: InsertionPoint::End, attributes: Default::default(), text: "world".to_owned() },
		)
		.await;
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "helloworld");

		// insert between two runs
		let (_char, position, _attributes) = state.chars(storage.clone()).skip(5).try_first().await.unwrap().unwrap();
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction { at: InsertionPoint::Before(position), attributes: Default::default(), text: " ".to_owned() },
		)
		.await;
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "hello world");
	}

	#[tokio::test]
	async fn test_insert_split() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;

		let state = RichText::default();
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "");

		// insert
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "helloworld".to_owned() },
		)
		.await;
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "helloworld");

		// split runs
		let (_char, position, _attributes) = state.chars(storage.clone()).skip(5).try_first().await.unwrap().unwrap();
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction { at: InsertionPoint::Before(position), attributes: Default::default(), text: " ".to_owned() },
		)
		.await;
		// println!("position: {:?}", position);
		// println!("state: {:?}", state);
		// println!("runs: {:?}", state.runs.stream(&storage).map_ok(|(_, run)| run).try_collect::<Vec<_>>().await);
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "hello world");
	}

	#[tokio::test]
	async fn test_format() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;

		let attributes0 = Attributes::default().with_attribute("hello", "world");
		let attributes1 = Attributes::default().with_attribute("test", "123");
		let attributes2 = Attributes::default()
			.with_attribute("hello", "world")
			.with_attribute("test", "123");
		let attributes0_link = storage.set_value(&attributes0).await.unwrap().into();
		let attributes2_link = storage.set_value(&attributes2).await.unwrap().into();

		// default
		let state = RichText::default();
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "");

		// insert
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction {
				at: InsertionPoint::Start,
				attributes: AttributesOperation::Merge(attributes0),
				text: "helloworld".to_owned(),
			},
		)
		.await;

		// split runs
		let characters = state
			.chars(storage.clone())
			.map_ok(|(_, p, _)| p)
			.skip(3)
			.take(2)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		let first = *characters.first().unwrap();
		let last = *characters.last().unwrap();
		let state = dispatch(
			&storage,
			&mut time,
			state,
			FormatAction { at: first, last: Some(last), attributes: AttributesOperation::Merge(attributes1) },
		)
		.await;
		// println!("position: {:?}", position);
		// println!("state: {:?}", state);
		// println!("runs: {:?}", state.runs.stream(&storage).map_ok(|(_, run)| run).try_collect::<Vec<_>>().await);
		let characters = state
			.chars(storage.clone())
			.map_ok(|(char, _position, attributes)| (char, attributes))
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		// println!("attributes0_link: {:?}", storage.get_value_or_default(&attributes0_link).await.unwrap());
		// println!("characters[0].1: {:?}", storage.get_value_or_default(&characters[0].1).await.unwrap());
		// println!("characters: {:?}", characters);
		assert_eq!(characters.len(), 10);
		assert_eq!(characters[0], ('h', attributes0_link));
		assert_eq!(characters[1], ('e', attributes0_link));
		assert_eq!(characters[2], ('l', attributes0_link));
		assert_eq!(characters[3], ('l', attributes2_link));
		assert_eq!(characters[4], ('o', attributes2_link));
		assert_eq!(characters[5], ('w', attributes0_link));
		assert_eq!(characters[6], ('o', attributes0_link));
		assert_eq!(characters[7], ('r', attributes0_link));
		assert_eq!(characters[8], ('l', attributes0_link));
		assert_eq!(characters[9], ('d', attributes0_link));
	}

	#[tokio::test]
	async fn test_delete() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;

		// default
		let state = RichText::default();
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "");

		// insert
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "hello".to_owned() },
		)
		.await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction { at: InsertionPoint::End, attributes: Default::default(), text: "world".to_owned() },
		)
		.await;

		// split runs
		let characters = state
			.chars(storage.clone())
			.map_ok(|(_, p, _)| p)
			.skip(4)
			.take(2)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		let first = *characters.first().unwrap();
		let last = *characters.last().unwrap();
		let state = dispatch(&storage, &mut time, state, DeleteAction { at: first, last: Some(last) }).await;
		assert_eq!(state.plain_text(&storage).await.unwrap().as_str(), "hellorld");
	}

	#[tokio::test]
	async fn test_text_model_range_uses_last_position_for_span_end() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;

		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "hello".to_owned() },
		)
		.await;

		let positions = state
			.chars(storage.clone())
			.map_ok(|(_char, position, _attributes)| position)
			.skip(1)
			.take(3)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();

		let at = *positions.first().unwrap();
		let last = *positions.last().unwrap();

		let state_link = storage.set_value(&state).await.unwrap();
		let model = TextModel { storage: storage.clone(), state: state_link.into() };

		let range = model.range(&at, &Some(last)).await.unwrap();

		assert_eq!(range, 1..4);
	}

	#[tokio::test]
	async fn test_text_model_utf8_byte_round_trips() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "Aé中😀B".to_owned() },
		)
		.await;
		let positions = state
			.chars(storage.clone())
			.map_ok(|(_char, position, _attributes)| position)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		let model = text_model(&storage, &state).await;
		let ranges = [0..1, 1..3, 3..6, 6..10, 10..11];

		for (position, range) in positions.iter().zip(ranges.iter()) {
			assert_eq!(model.position(range.start).await.unwrap(), Some(*position));
			assert_eq!(model.index(position).await.unwrap(), range.start);
			let (at, last) = model.position_range(range).await.unwrap();
			assert_eq!((at, last), (Some(*position), None));
			assert_eq!(model.range(&at.unwrap(), &last).await.unwrap(), *range);
		}
		assert_eq!(model.position(11).await.unwrap(), None);
		for index in [2, 4, 5, 7, 8, 9, 12] {
			assert!(model.position(index).await.is_err(), "index {index} must be rejected");
		}
		assert!(model.index(&positions[1].right()).await.is_err());

		let range = 1..10;
		let (at, last) = model.position_range(&range).await.unwrap();
		assert_eq!(at, Some(positions[1]));
		assert_eq!(last, Some(positions[3].right_by('😀'.len_utf8() - 1)));
		assert_eq!(model.range(&at.unwrap(), &last).await.unwrap(), range);

		for range in [0..0, 1..2, 2..3, 10..12, 11..12] {
			assert!(model.position_range(&range).await.is_err(), "range {range:?} must be rejected");
		}
		assert!(model.range(&positions[1], &Some(positions[3].right())).await.is_err());
		assert!(model.range(&positions[2], &Some(positions[0])).await.is_err());
	}

	#[tokio::test]
	async fn test_text_model_utf8_edits_and_changes() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "Aé中😀B".to_owned() },
		)
		.await;
		let model = text_model(&storage, &state).await;
		let insert = model.insert(3, "|".to_owned(), Default::default()).await.unwrap();
		let insert_end = model.insert(11, "!".to_owned(), Default::default()).await.unwrap();
		assert!(model.insert(2, "|".to_owned(), Default::default()).await.is_err());
		assert!(model.insert(12, "|".to_owned(), Default::default()).await.is_err());
		assert!(model.insert(3, String::new(), Default::default()).await.is_err());

		let deleted = model.delete(1..10).await.unwrap();
		let single = model.delete(1..3).await.unwrap();
		let marked = Attributes::default().with_attribute("marked", true);
		let formatted = model.format(1..10, AttributesOperation::Merge(marked.clone())).await.unwrap();
		match &single {
			RichTextAction::Delete(action) => assert_eq!(action.last, None),
			_ => panic!("expected delete action"),
		}

		let inserted = Box::pin(dispatch(&storage, &mut time, state.clone(), insert.clone())).await;
		assert_eq!(inserted.plain_text(&storage).await.unwrap(), "Aé|中😀B");
		let inserted = Box::pin(dispatch(&storage, &mut time, state.clone(), insert_end.clone())).await;
		assert_eq!(inserted.plain_text(&storage).await.unwrap(), "Aé中😀B!");
		let (anchored, anchor) = inherited_empty_anchor(&storage, "Aé中😀B", state.left.unwrap(), 3).await;
		assert_eq!(anchored.plain_text(&storage).await.unwrap(), "Aé中😀B");
		let anchored_model = text_model(&storage, &anchored).await;
		assert_eq!(anchored_model.position(3).await.unwrap(), model.position(3).await.unwrap());
		assert!(anchored_model.index(&anchor).await.is_err());
		let empty_before_anchor: RichTextAction =
			InsertAction { at: InsertionPoint::Before(anchor), attributes: Default::default(), text: String::new() }
				.into();
		assert!(anchored_model.text_change(&[empty_before_anchor]).await.is_err());
		let before_anchor: RichTextAction =
			InsertAction { at: InsertionPoint::Before(anchor), attributes: Default::default(), text: "|".to_owned() }
				.into();
		let anchor_changes = anchored_model.text_change(std::slice::from_ref(&before_anchor)).await.unwrap();
		match &anchor_changes[0] {
			TextModelChange::Insert { index, .. } => assert_eq!(*index, 3),
			_ => panic!("expected insert change"),
		}
		let inserted = Box::pin(dispatch(&storage, &mut time, anchored, before_anchor)).await;
		assert_eq!(inserted.plain_text(&storage).await.unwrap(), "Aé|中😀B");
		let linked = inserted.runs(storage.clone()).try_collect::<Vec<_>>().await.unwrap();
		assert!(linked.iter().any(|run| run.id == anchor && run.text.is_empty()));
		let deleted_state = dispatch(&storage, &mut time, state.clone(), deleted.clone()).await;
		assert_eq!(deleted_state.plain_text(&storage).await.unwrap(), "AB");

		let formatted_state = Box::pin(dispatch(&storage, &mut time, state.clone(), formatted.clone())).await;
		let attributes = formatted_state
			.chars(storage.clone())
			.map_ok(|(_char, _position, attributes)| attributes)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		let base_attributes = attributes[0];
		let marked_link = storage.set_value(&marked).await.unwrap().into();
		assert_eq!(attributes, vec![base_attributes, marked_link, marked_link, marked_link, base_attributes]);

		let changes = model.text_change(&[insert, insert_end, deleted, formatted]).await.unwrap();
		assert_eq!(changes.len(), 4);
		for (change, index, text) in [(&changes[0], 3, "|"), (&changes[1], 11, "!")] {
			match change {
				TextModelChange::Insert { index: actual, text: actual_text, .. } => {
					assert_eq!((*actual, actual_text.as_str()), (index, text));
				},
				_ => panic!("expected insert change"),
			}
		}
		match &changes[2] {
			TextModelChange::Delete { range } => assert_eq!(range, &(1..10)),
			_ => panic!("expected delete change"),
		}
		match &changes[3] {
			TextModelChange::Format { range, attributes } => {
				assert_eq!(range, &(1..10));
				assert_eq!(attributes, &marked);
			},
			_ => panic!("expected format change"),
		}
	}

	#[tokio::test]
	async fn test_text_model_tombstoned_utf8_ranges() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "Aé中😀B".to_owned() },
		)
		.await;
		let positions = state
			.chars(storage.clone())
			.map_ok(|(_char, position, _attributes)| position)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		let state = dispatch(&storage, &mut time, state, DeleteAction { at: positions[2], last: None }).await;
		let model = text_model(&storage, &state).await;
		assert_eq!(model.plain_text().await.unwrap(), "Aé😀B");
		assert_eq!(model.index(&positions[3]).await.unwrap(), 3);
		assert_eq!(model.position(3).await.unwrap(), Some(positions[3]));

		let last = positions[3].right_by('😀'.len_utf8() - 1);
		assert_eq!(model.range(&positions[1], &Some(last)).await.unwrap(), 1..7);
		assert_eq!(model.range(&positions[2], &None).await.unwrap(), 3..3);
		let changes = model
			.text_change(&[
				DeleteAction { at: positions[2], last: None }.into(),
				DeleteAction { at: positions[1], last: Some(last) }.into(),
			])
			.await
			.unwrap();
		assert_eq!(changes.len(), 1);
		match &changes[0] {
			TextModelChange::Delete { range } => assert_eq!(range, &(1..7)),
			_ => panic!("expected delete change"),
		}
	}

	#[tokio::test]
	async fn test_plain_text_preserves_exact_utf8_bytes() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let decomposed = "e\u{301}";
		let composed = "é";
		assert_ne!(decomposed.as_bytes(), composed.as_bytes());

		for text in [decomposed, composed, "e\u{301}|é\r\n中\n😀"] {
			let state = dispatch(
				&storage,
				&mut time,
				RichText::default(),
				InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: text.to_owned() },
			)
			.await;
			assert_eq!(state.plain_text(&storage).await.unwrap().as_bytes(), text.as_bytes());
			assert_eq!(text_model(&storage, &state).await.plain_text().await.unwrap().as_bytes(), text.as_bytes());
		}
	}

	#[test]
	fn test_insertion_point_tags() {
		let position = Position(Cid::default().into(), 3);
		let after = to_cbor(&InsertionPoint::After(position)).unwrap();
		let before = to_cbor(&InsertionPoint::Before(position)).unwrap();
		assert_eq!(after[..3], [0xa1, 0x61, b'a']);
		assert_eq!(before[..3], [0xa1, 0x61, b'b']);
		assert_eq!(after[3..], before[3..]);
		assert_eq!(to_cbor(&InsertionPoint::Start).unwrap(), [0x61, b'l']);
		assert_eq!(to_cbor(&InsertionPoint::End).unwrap(), [0x61, b'r']);
		assert_eq!(from_cbor::<InsertionPoint>(&after).unwrap(), InsertionPoint::After(position));
		assert_eq!(from_cbor::<InsertionPoint>(&before).unwrap(), InsertionPoint::Before(position));
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn test_after_inserts_behind_scalar() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let bold = Attributes::default().with_attribute("bold", true);
		let bold_link: OptionLink<Attributes> = storage.set_value(&bold).await.unwrap().into();
		let plain_link: OptionLink<Attributes> = storage.set_value(&Attributes::default()).await.unwrap().into();
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction {
				at: InsertionPoint::Start,
				attributes: AttributesOperation::Merge(bold),
				text: "abc".to_owned(),
			},
		)
		.await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction {
				at: InsertionPoint::End,
				attributes: AttributesOperation::Replace(Attributes::default()),
				text: "def".to_owned(),
			},
		)
		.await;
		let positions = scalar_positions(&storage, &state).await;
		let model = text_model(&storage, &state).await;

		for (anchor, expected, attributes) in [
			(0, "aXYbcdef", bold_link),
			(1, "abXYcdef", bold_link),
			(2, "abcXYdef", bold_link),
			(3, "abcdXYef", plain_link),
			(5, "abcdefXY", plain_link),
		] {
			let action: RichTextAction = InsertAction {
				at: InsertionPoint::After(positions[anchor]),
				attributes: Default::default(),
				text: "XY".to_owned(),
			}
			.into();
			let index = insert_index(&model, &action).await;
			let mut placed = "abcdef".to_owned();
			placed.insert_str(index, "XY");
			assert_eq!(placed, expected);

			let next = dispatch(&storage, &mut time, state.clone(), action).await;
			assert_eq!(next.plain_text(&storage).await.unwrap(), expected);
			assert_linked(&storage, &next).await;
			let chars = next.chars(storage.clone()).try_collect::<Vec<_>>().await.unwrap();
			assert_eq!(chars[index].2, attributes);

			let next = dispatch(
				&storage,
				&mut time,
				next,
				InsertAction { at: InsertionPoint::End, attributes: Default::default(), text: "!".to_owned() },
			)
			.await;
			assert_eq!(next.plain_text(&storage).await.unwrap(), format!("{expected}!"));
			assert_linked(&storage, &next).await;
		}
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn test_after_keeps_links_through_later_edits() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "abcdef".to_owned() },
		)
		.await;
		let positions = scalar_positions(&storage, &state).await;

		// split the run behind 'b'
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction {
				at: InsertionPoint::After(positions[1]),
				attributes: Default::default(),
				text: "XY".to_owned(),
			},
		)
		.await;
		assert_eq!(state.plain_text(&storage).await.unwrap(), "abXYcdef");
		assert_linked(&storage, &state).await;
		let positions = scalar_positions(&storage, &state).await;

		// surround the new run
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction {
				at: InsertionPoint::Before(positions[4]),
				attributes: Default::default(),
				text: "Z".to_owned(),
			},
		)
		.await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction {
				at: InsertionPoint::After(positions[3]),
				attributes: Default::default(),
				text: "Q".to_owned(),
			},
		)
		.await;
		assert_eq!(state.plain_text(&storage).await.unwrap(), "abXYQZcdef");
		assert_linked(&storage, &state).await;
		let positions = scalar_positions(&storage, &state).await;

		// delete and format across the new runs by original identities
		let state =
			dispatch(&storage, &mut time, state, DeleteAction { at: positions[2], last: Some(positions[5]) }).await;
		assert_eq!(state.plain_text(&storage).await.unwrap(), "abcdef");
		assert_linked(&storage, &state).await;
		let marked = Attributes::default().with_attribute("marked", true);
		let marked_link: OptionLink<Attributes> = storage.set_value(&marked).await.unwrap().into();
		let state = dispatch(
			&storage,
			&mut time,
			state,
			FormatAction { at: positions[0], last: Some(positions[7]), attributes: AttributesOperation::Merge(marked) },
		)
		.await;
		assert_linked(&storage, &state).await;
		let chars = state.chars(storage.clone()).try_collect::<Vec<_>>().await.unwrap();
		assert_eq!(
			chars
				.iter()
				.filter(|(_char, _position, attributes)| *attributes == marked_link)
				.count(),
			4
		);

		// insert after deleted scalars
		let action: RichTextAction = InsertAction {
			at: InsertionPoint::After(positions[2]),
			attributes: Default::default(),
			text: "|".to_owned(),
		}
		.into();
		assert_eq!(insert_index(&text_model(&storage, &state).await, &action).await, 2);
		let state = dispatch(&storage, &mut time, state, action).await;
		assert_eq!(state.plain_text(&storage).await.unwrap(), "ab|cdef");
		assert_linked(&storage, &state).await;
		let action: RichTextAction = InsertAction {
			at: InsertionPoint::After(positions[5]),
			attributes: Default::default(),
			text: "-".to_owned(),
		}
		.into();
		assert_eq!(insert_index(&text_model(&storage, &state).await, &action).await, 3);
		let state = dispatch(&storage, &mut time, state, action).await;
		assert_eq!(state.plain_text(&storage).await.unwrap(), "ab|-cdef");
		assert_linked(&storage, &state).await;
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn test_after_split_relinks_the_outer_successor() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "D".to_owned() },
		)
		.await;
		let d = state.left.unwrap();

		// a multi-byte `Before` insertion links `D` back to the first byte of the new run
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction { at: InsertionPoint::Before(d), attributes: Default::default(), text: "abc".to_owned() },
		)
		.await;
		assert_eq!(state.plain_text(&storage).await.unwrap(), "abcD");
		let positions = scalar_positions(&storage, &state).await;

		// splitting that run behind `a` must leave `D` linked to `c`
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction {
				at: InsertionPoint::After(positions[0]),
				attributes: Default::default(),
				text: "w".to_owned(),
			},
		)
		.await;
		assert_eq!(state.plain_text(&storage).await.unwrap(), "awbcD");
		assert_linked(&storage, &state).await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction { at: InsertionPoint::Before(d), attributes: Default::default(), text: "z".to_owned() },
		)
		.await;
		assert_eq!(state.plain_text(&storage).await.unwrap(), "awbczD");
		assert_linked(&storage, &state).await;
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn test_after_utf8_scalars_and_invalid_anchors() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let text = "Aé中😀B";
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: text.to_owned() },
		)
		.await;
		let id = state.left.unwrap();
		let model = text_model(&storage, &state).await;

		for offset in [0, 1, 3, 6, 10] {
			let action: RichTextAction = InsertAction {
				at: InsertionPoint::After(id.right_by(offset)),
				attributes: Default::default(),
				text: "|".to_owned(),
			}
			.into();
			let index = insert_index(&model, &action).await;
			assert_eq!(index, offset + text[offset..].chars().next().unwrap().len_utf8());
			let mut expected = text.to_owned();
			expected.insert(index, '|');
			let next = dispatch(&storage, &mut time, state.clone(), action).await;
			assert_eq!(next.plain_text(&storage).await.unwrap(), expected);
			assert_linked(&storage, &next).await;
		}

		let unknown = Position(Cid::default().into(), 0);
		for at in [2, 4, 5, 7, 8, 9, 11]
			.map(|offset| id.right_by(offset))
			.into_iter()
			.chain([unknown])
		{
			let action: RichTextAction =
				InsertAction { at: InsertionPoint::After(at), attributes: Default::default(), text: "|".to_owned() }
					.into();
			assert!(try_dispatch(&storage, &mut time, state.clone(), action.clone()).await.is_err(), "{at:?}");
			assert!(model.text_change(&[action]).await.is_err(), "{at:?}");
		}

		// empty text and empty anchors are rejected
		let empty: RichTextAction =
			InsertAction { at: InsertionPoint::After(id), attributes: Default::default(), text: String::new() }.into();
		assert!(try_dispatch(&storage, &mut time, state.clone(), empty).await.is_err());
		let (anchored, anchor) = inherited_empty_anchor(&storage, text, id, 3).await;
		let action: RichTextAction =
			InsertAction { at: InsertionPoint::After(anchor), attributes: Default::default(), text: "|".to_owned() }
				.into();
		assert!(try_dispatch(&storage, &mut time, anchored.clone(), action.clone())
			.await
			.is_err());
		assert!(text_model(&storage, &anchored).await.text_change(&[action]).await.is_err());
		assert_eq!(state.plain_text(&storage).await.unwrap(), text);
	}

	#[tokio::test]
	async fn test_empty_insertions_are_rejected_without_mutation() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let text = "Aé中😀B";
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: text.to_owned() },
		)
		.await;
		let id = state.left.unwrap();
		let model = text_model(&storage, &state).await;
		let empty = RichText::default();
		let bold = AttributesOperation::Merge(Attributes::default().with_attribute("bold", true));

		for at in [InsertionPoint::Start, InsertionPoint::End, InsertionPoint::Before(id), InsertionPoint::After(id)] {
			for attributes in [AttributesOperation::default(), bold.clone(), AttributesOperation::RemoveAll] {
				let action = InsertAction { at: at.clone(), attributes, text: String::new() };
				assert_rejected(&storage, &empty, action.clone()).await;
				assert_rejected(&storage, &state, action.clone()).await;
				assert!(model.text_change(&[action.into()]).await.is_err(), "{at:?}");
			}
		}
		assert_eq!(state.plain_text(&storage).await.unwrap(), text);
		assert!(model.insert(0, String::new(), bold).await.is_err());
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn test_after_deleted_anchor() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction { at: InsertionPoint::Start, attributes: Default::default(), text: "Aé中😀B".to_owned() },
		)
		.await;
		let positions = scalar_positions(&storage, &state).await;
		let state = dispatch(&storage, &mut time, state, DeleteAction { at: positions[2], last: None }).await;
		assert_eq!(state.plain_text(&storage).await.unwrap(), "Aé😀B");
		let model = text_model(&storage, &state).await;

		for (anchor, index, expected) in [(1, 3, "Aé|😀B"), (2, 3, "Aé|😀B"), (3, 7, "Aé😀|B")] {
			let action: RichTextAction = InsertAction {
				at: InsertionPoint::After(positions[anchor]),
				attributes: Default::default(),
				text: "|".to_owned(),
			}
			.into();
			assert_eq!(insert_index(&model, &action).await, index);
			let next = dispatch(&storage, &mut time, state.clone(), action).await;
			assert_eq!(next.plain_text(&storage).await.unwrap(), expected);
			assert_linked(&storage, &next).await;
		}

		// deleted scalars stay addressable once nothing is visible
		let deleted =
			dispatch(&storage, &mut time, state.clone(), DeleteAction { at: positions[0], last: Some(positions[4]) })
				.await;
		assert_eq!(deleted.plain_text(&storage).await.unwrap(), "");
		let action: RichTextAction = InsertAction {
			at: InsertionPoint::After(positions[4]),
			attributes: Default::default(),
			text: "x".to_owned(),
		}
		.into();
		assert_eq!(insert_index(&text_model(&storage, &deleted).await, &action).await, 0);
		let appended = dispatch(&storage, &mut time, deleted, action).await;
		assert_eq!(appended.plain_text(&storage).await.unwrap(), "x");
		assert_linked(&storage, &appended).await;
		let inserted = scalar_positions(&storage, &appended).await;
		let appended = dispatch(
			&storage,
			&mut time,
			appended,
			InsertAction {
				at: InsertionPoint::After(inserted[0]),
				attributes: Default::default(),
				text: "y".to_owned(),
			},
		)
		.await;
		assert_eq!(appended.plain_text(&storage).await.unwrap(), "xy");
		let action: RichTextAction = InsertAction {
			at: InsertionPoint::After(positions[0]),
			attributes: Default::default(),
			text: "0".to_owned(),
		}
		.into();
		assert_eq!(insert_index(&text_model(&storage, &appended).await, &action).await, 0);
		let appended = dispatch(&storage, &mut time, appended, action).await;
		assert_eq!(appended.plain_text(&storage).await.unwrap(), "0xy");
		assert_linked(&storage, &appended).await;
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn test_text_model_insert_authors_relative_points() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let point = |action: RichTextAction| match action {
			RichTextAction::Insert(action) => (action.at, action.attributes),
			_ => panic!("expected insert action"),
		};
		let text = |text: &str| text.to_owned();

		// structurally empty text
		let model = text_model(&storage, &RichText::default()).await;
		let (at, attributes) = point(model.insert(0, text("a"), Default::default()).await.unwrap());
		assert_eq!((at, attributes), (InsertionPoint::Start, AttributesOperation::Replace(Attributes::default())));
		assert!(model.insert(0, String::new(), Default::default()).await.is_err());

		// visible scalars
		let bold = Attributes::default().with_attribute("bold", true);
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction {
				at: InsertionPoint::Start,
				attributes: AttributesOperation::Merge(bold.clone()),
				text: text("aé"),
			},
		)
		.await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction {
				at: InsertionPoint::End,
				attributes: AttributesOperation::Replace(Attributes::default()),
				text: text("中"),
			},
		)
		.await;
		let positions = scalar_positions(&storage, &state).await;
		let model = text_model(&storage, &state).await;
		let italic = Attributes::default().with_attribute("italic", true);
		let bold_italic = bold.clone().with_merge(italic.clone());
		let cases = [
			(0, Default::default(), InsertionPoint::Before(positions[0]), bold.clone()),
			(1, AttributesOperation::Merge(italic.clone()), InsertionPoint::After(positions[0]), bold_italic),
			(
				3,
				AttributesOperation::Remove(["bold".to_owned()].into()),
				InsertionPoint::After(positions[1]),
				Attributes::default(),
			),
			(3, AttributesOperation::RemoveAll, InsertionPoint::After(positions[1]), Attributes::default()),
			(6, AttributesOperation::Merge(italic.clone()), InsertionPoint::After(positions[2]), italic.clone()),
			(6, AttributesOperation::Replace(bold.clone()), InsertionPoint::After(positions[2]), bold.clone()),
		];
		for (index, operation, expected_at, expected_attributes) in cases {
			let (at, attributes) = point(model.insert(index, text("x"), operation).await.unwrap());
			assert_eq!((at, attributes), (expected_at, AttributesOperation::Replace(expected_attributes)), "{index}");
		}
		for index in [2, 4, 5, 7] {
			assert!(model.insert(index, text("x"), Default::default()).await.is_err(), "{index}");
		}

		// the empty string is rejected
		assert!(model
			.insert(3, String::new(), AttributesOperation::Merge(italic.clone()))
			.await
			.is_err());
		assert!(model.insert(6, String::new(), Default::default()).await.is_err());

		// deleted text keeps `End`, resolving the last run's attributes
		let deleted = dispatch(
			&storage,
			&mut time,
			state,
			DeleteAction { at: positions[0], last: Some(positions[2].right_by(2)) },
		)
		.await;
		assert_eq!(deleted.plain_text(&storage).await.unwrap(), "");
		let model = text_model(&storage, &deleted).await;
		let (at, attributes) = point(
			model
				.insert(0, text("x"), AttributesOperation::Merge(italic.clone()))
				.await
				.unwrap(),
		);
		assert_eq!((at, attributes), (InsertionPoint::End, AttributesOperation::Replace(italic)));
		assert!(model.insert(0, String::new(), Default::default()).await.is_err());
		assert!(model.insert(1, text("x"), Default::default()).await.is_err());

		// at a formatting boundary the snapshot follows the scalar at the cursor, not the anchor
		let state = dispatch(
			&storage,
			&mut time,
			RichText::default(),
			InsertAction {
				at: InsertionPoint::Start,
				attributes: AttributesOperation::Merge(bold.clone()),
				text: text("a"),
			},
		)
		.await;
		let state = dispatch(
			&storage,
			&mut time,
			state,
			InsertAction {
				at: InsertionPoint::End,
				attributes: AttributesOperation::Replace(Attributes::default()),
				text: text("b"),
			},
		)
		.await;
		let positions = scalar_positions(&storage, &state).await;
		let model = text_model(&storage, &state).await;
		let italic = Attributes::default().with_attribute("italic", true);
		let cases = [
			(0, Default::default(), InsertionPoint::Before(positions[0]), bold.clone()),
			(1, Default::default(), InsertionPoint::After(positions[0]), Attributes::default()),
			(1, AttributesOperation::Merge(italic.clone()), InsertionPoint::After(positions[0]), italic.clone()),
			(2, Default::default(), InsertionPoint::After(positions[1]), Attributes::default()),
			(
				0,
				AttributesOperation::Merge(italic.clone()),
				InsertionPoint::Before(positions[0]),
				bold.with_merge(italic),
			),
		];
		for (index, operation, expected_at, expected_attributes) in cases {
			let (at, attributes) = point(model.insert(index, text("x"), operation).await.unwrap());
			assert_eq!((at, attributes), (expected_at, AttributesOperation::Replace(expected_attributes)), "{index}");
		}
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn test_disconnected_forward_typing_stays_contiguous() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let a = identity("a");
		let b = identity("b");
		let cases = [
			("ABC", 3, "xy", "uv", ["ABCxyuv", "ABCuvxy"]),
			("ABC", 0, "xy", "uv", ["xyuvABC", "uvxyABC"]),
			("ABC", 1, "xy", "uv", ["AxyuvBC", "AuvxyBC"]),
			("ABC", 3, "xyz", "u", ["ABCxyzu", "ABCuxyz"]),
			("Aé中", 3, "😀ü", "中x", ["Aé😀ü中x中", "Aé中x😀ü中"]),
		];
		for (shared, index, text_a, text_b, expected) in cases {
			let state = merge_typing(&storage, &mut time, shared, index, (&a, text_a), (&b, text_b)).await;
			let text = state.plain_text(&storage).await.unwrap();
			assert!(expected.contains(&text.as_str()), "{shared} at {index}: {text_a} | {text_b} = {text}");
		}

		// one signing identity on both peers
		let state = merge_typing(&storage, &mut time, "ABC", 3, (&a, "xy"), (&a, "uv")).await;
		let text = state.plain_text(&storage).await.unwrap();
		assert!(["ABCxyuv", "ABCuvxy"].contains(&text.as_str()), "{text}");
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn test_merged_sequences_keep_formatting_and_identities() {
		let storage = MemoryBlockStorage::default();
		let mut time = 1;
		let a = identity("a");
		let b = identity("b");
		let bold = Attributes::default().with_attribute("bold", true);
		let bold_link: OptionLink<Attributes> = storage.set_value(&bold).await.unwrap().into();

		// peer a types bold, peer b types plain at the same gap
		let mut log_a = signed_log(Default::default());
		type_text(&storage, &mut log_a, &a, &mut time, 0, "ABC", Default::default()).await;
		let mut log_b = signed_log(log_a.heads().clone());
		type_text(&storage, &mut log_a, &a, &mut time, 3, "xy", AttributesOperation::Merge(bold)).await;
		type_text(&storage, &mut log_b, &b, &mut time, 3, "uv", Default::default()).await;
		let mut merged = log_a.clone();
		assert!(merged.join(&storage, &log_b).await.unwrap());
		let (state, cid) = replay(&storage, &merged).await;
		let text = state.plain_text(&storage).await.unwrap();
		assert!(["ABCxyuv", "ABCuvxy"].contains(&text.as_str()), "{text}");
		let chars = state.chars(storage.clone()).try_collect::<Vec<_>>().await.unwrap();
		for (char, _position, attributes) in &chars {
			assert_eq!(*attributes == bold_link, matches!(char, 'x' | 'y'), "{char}");
		}
		let positions: BTreeMap<char, Position> = chars.iter().map(|(char, position, _)| (*char, *position)).collect();

		// later edits address the original identities from either peer
		let marked = Attributes::default().with_attribute("marked", true);
		let marked_link: OptionLink<Attributes> = storage.set_value(&marked).await.unwrap().into();
		for (identity, payload) in [
			(&b, RichTextAction::from(DeleteAction { at: positions[&'y'], last: None })),
			(
				&a,
				FormatAction {
					at: positions[&'u'],
					last: Some(positions[&'v']),
					attributes: AttributesOperation::Merge(marked),
				}
				.into(),
			),
		] {
			let action =
				ReducerAction { core: "rich-text".to_owned(), from: identity.identity().to_owned(), payload, time };
			time += 1;
			merged.push_event(&storage, identity, &action).await.unwrap();
		}
		let (edited, edited_cid) = replay(&storage, &merged).await;
		assert_ne!(edited_cid, cid);
		let edited_text = edited.plain_text(&storage).await.unwrap();
		assert!(["ABCxuv", "ABCuvx"].contains(&edited_text.as_str()), "{edited_text}");
		for (char, _position, attributes) in edited.chars(storage.clone()).try_collect::<Vec<_>>().await.unwrap() {
			assert_eq!(attributes == marked_link, matches!(char, 'u' | 'v'), "{char}");
			assert_eq!(attributes == bold_link, char == 'x', "{char}");
		}
		assert_linked(&storage, &edited).await;

		// the other peer converges, and typing continues behind the merged text
		let mut merged_b = log_b.clone();
		assert!(merged_b.join(&storage, &merged).await.unwrap());
		assert_eq!(replay(&storage, &merged_b).await.1, edited_cid);
		type_text(&storage, &mut merged_b, &b, &mut time, edited_text.len(), "!", Default::default()).await;
		assert_eq!(replay(&storage, &merged_b).await.0.plain_text(&storage).await.unwrap(), format!("{edited_text}!"));
	}
}
