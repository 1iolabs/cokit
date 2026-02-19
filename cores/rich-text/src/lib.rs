use anyhow::anyhow;
use cid::Cid;
use co_api::{
	async_api::Reducer, co, BlockStorage, BlockStorageExt, CoMap, CoTryStreamExt, CoreBlockStorage, IsDefault,
	LazyTransaction, Link, OptionLink, ReducerAction, TagValue, WeakCid,
};
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	future::ready,
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
	/// The position to delete.
	#[serde(rename = "l")]
	pub at: Position,

	/// The last position to deleted.
	/// If omited only `at` is deleted.
	#[serde(rename = "r", default, skip_serializing_if = "IsDefault::is_default")]
	pub last: Option<Position>,
}

#[co]
pub struct FormatAction {
	/// The position to format.
	#[serde(rename = "l")]
	pub at: Position,

	/// The last position to formatted.
	/// If omited only `at` is formatted.
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
	/// Before position.
	#[serde(rename = "b")]
	Before(Position),
}
impl InsertionPoint {
	pub fn position(&self, state: &RichText) -> Option<Position> {
		match self {
			InsertionPoint::Start => state.left,
			InsertionPoint::End => state.right,
			InsertionPoint::Before(position) => Some(*position),
		}
	}
}

#[co]
#[derive(Default, Copy)]
pub struct Position(WeakCid, usize);
impl Position {
	pub fn left(&self) -> Option<Self> {
		if self.1 > 0 {
			Some(Self(self.0, self.1 - 1))
		} else {
			None
		}
	}

	pub fn right(&self) -> Self {
		Self(self.0, self.1 + 1)
	}

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

	/// Stream characters with position and formatting.
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
						position = position.right();
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
	/// The first character in this run.
	pub fn first(&self) -> Position {
		self.id
	}

	/// The last character in this run.
	/// If the run has only one character this is equal to frist.
	pub fn last(&self) -> Position {
		assert!(!self.text.is_empty());
		self.id.right_by(self.text.len() - 1)
	}

	pub fn range(&self) -> Range<usize> {
		self.id.1..self.id.1 + self.text.len()
	}

	pub fn contains(&self, at: Position) -> bool {
		self.id.0 == at.0 && self.range().contains(&at.1)
	}
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
		RichTextAction::Insert(action) => reduce_text_insert(storage, state, &mut transaction, head, action).await?,
		RichTextAction::Delete(action) => reduce_text_delete(storage, state, &mut transaction, head, action).await?,
		RichTextAction::Format(action) => reduce_text_format(storage, state, &mut transaction, head, action).await?,
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
	let last = action.last.unwrap_or_else(|| action.at.right());

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
		if !is_last {
			if let Some(next_id) = next_id {
				run = transaction.get_run(next_id).await?;
				continue;
			}
		}

		// done
		break;
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

	// attributes
	let attributes = attributes_run(storage, Some(&run), &action.attributes).await?;
	let attributes_link = storage.set_value(&attributes).await?.into();

	// last
	let last = action.last.unwrap_or_else(|| action.at.right());

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
		if !is_last {
			if let Some(next_id) = next_id {
				run = transaction.get_run(next_id).await?;
				continue;
			}
		}

		// done
		break;
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
	if !run.contains(at) {
		return Err(anyhow!("Invalid range"));
	}

	// text
	let text_offset = at.1 - run.id.1;
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

pub struct TextModel<S> {
	storage: S,
	state: OptionLink<RichText>,
}
impl<S> TextModel<S>
where
	S: BlockStorage + Clone + 'static,
{
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

	/// Index for position.
	pub async fn index(&self, at: &Position) -> anyhow::Result<usize> {
		let state = self.storage.get_value_or_default(&self.state).await?;

		// walk runs
		let mut index = 0;
		let runs = state.runs(self.storage.clone());
		pin_mut!(runs);
		while let Some(run) = runs.try_next().await? {
			// done?
			if run.contains(*at) {
				return Ok(if !run.deleted { index + at.1 - run.id.1 } else { index });
			}

			// index
			if !run.deleted {
				index += run.text.len();
			}
		}
		Err(anyhow!("Position not found: {:?}", at))
	}

	/// Range for positions.
	pub async fn range(&self, at: &Position, last: &Option<Position>) -> anyhow::Result<Range<usize>> {
		let state = self.storage.get_value_or_default(&self.state).await?;

		// walk runs
		let mut index = 0;
		let mut start_found = false;
		let mut start = 0;
		let mut start_deleted = false;
		let runs = state.runs(self.storage.clone());
		pin_mut!(runs);
		while let Some(run) = runs.try_next().await? {
			// done?
			if !start_found && run.contains(*at) {
				start = if !run.deleted { index + at.1 - run.id.1 } else { index };
				start_found = true;
				start_deleted = run.deleted;
			}
			if start_found {
				if let Some(last) = last {
					if run.contains(*last) {
						return Ok(Range { start, end: if !run.deleted { index + at.1 - run.id.1 } else { index } });
					}
				} else if run.deleted && start_deleted {
					// return a empty range as the range is fully deleted
					return Ok(Range { start, end: start });
				} else {
					return Ok(Range { start, end: start + 1 });
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

	/// Position for index.
	pub async fn position(&self, index: usize) -> anyhow::Result<Option<Position>> {
		let state = self.storage.get_value_or_default(&self.state).await?;
		let at = state
			.chars(self.storage.clone())
			.skip(index)
			.map_ok(|(_char, position, _attributes)| position)
			.try_first()
			.await?;
		Ok(at)
	}

	/// Positions for range.
	pub async fn position_range(&self, range: &Range<usize>) -> anyhow::Result<(Option<Position>, Option<Position>)> {
		// validate
		if range.is_empty() {
			return Err(anyhow!("Invalid range: {:?}", range));
		}

		// find indicies
		let state = self.storage.get_value_or_default(&self.state).await?;
		let (at, last) = state
			.chars(self.storage.clone())
			.enumerate()
			.skip(range.start)
			.take(range.len())
			.map(|(i, result)| result.map(|(_char, position, _attributes)| (i, position)))
			.try_fold((None, None), |(mut at, mut last), (i, position)| {
				if i == range.start {
					at = Some(position);
				}
				if i == range.end {
					last = Some(position);
				}
				ready(Result::<_, anyhow::Error>::Ok((at, last)))
			})
			.await?;

		// result
		Ok((at, if range.len() == 1 { None } else { last }))
	}

	pub async fn insert(
		&self,
		index: usize,
		text: String,
		attributes: AttributesOperation,
	) -> anyhow::Result<RichTextAction> {
		let at = self
			.position(index)
			.await?
			.map(InsertionPoint::Before)
			.unwrap_or(InsertionPoint::End);
		Ok(InsertAction { at, text, attributes }.into())
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
					let insertion_point = normalize_insertion_point(&state, action.at.clone());
					let index = if let Some(position) = insertion_point.position(&state) {
						self.index(&position).await?
					} else {
						0
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
		RichTextAction, Run,
	};
	use cid::Cid;
	use co_api::{
		async_api::Reducer, BlockStorage, BlockStorageExt, CoTryStreamExt, CoreBlockStorage, Date, ReducerAction,
	};
	use co_storage::MemoryBlockStorage;
	use futures::{StreamExt, TryStreamExt};

	async fn dispatch<S>(storage: &S, time: &mut Date, state: RichText, action: impl Into<RichTextAction>) -> RichText
	where
		S: BlockStorage + Clone + 'static,
	{
		let action = ReducerAction { core: "".to_owned(), from: "".to_owned(), payload: action.into(), time: *time };
		*time += 1;
		let action_link = storage.set_value(&action).await.unwrap();
		let state_link = storage.set_value(&state).await.unwrap();
		let next_state_link =
			RichText::reduce(state_link.into(), action_link, &CoreBlockStorage::new(storage.clone(), true))
				.await
				.unwrap();
		storage.get_value(&next_state_link).await.unwrap()
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
}
