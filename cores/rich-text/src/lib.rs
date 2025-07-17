use anyhow::anyhow;
use cid::Cid;
use co_api::{
	async_api::Reducer, co, BlockStorage, BlockStorageExt, CoMap, IsDefault, LazyTransaction, Link, OptionLink,
	ReducerAction, TagValue, WeakCid,
};
use futures::{Stream, TryStreamExt};
use std::{collections::BTreeMap, ops::Range};

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
	pub at: InsertionPoint,

	/// The text to insert.
	pub text: String,

	/// The attributes.
	pub attributes: AttributesOperation,
}

#[co]
pub struct DeleteAction {
	/// The position to delete.
	pub at: Position,

	/// The last position to deleted.
	/// If omited only `at` is deleted.
	pub last: Option<Position>,
}

#[co]
pub struct FormatAction {
	/// The position to format.
	pub at: Position,

	/// The last position to formatted.
	/// If omited only `at` is formatted.
	pub last: Option<Position>,

	/// The attributes.
	pub attributes: AttributesOperation,
}

#[co]
pub enum AttributesOperation {
	Merge(Attributes),
	Replace(Attributes),
	Remove,
}
impl Default for AttributesOperation {
	fn default() -> Self {
		AttributesOperation::Merge(Default::default())
	}
}

#[co]
pub enum InsertionPoint {
	/// First position.
	Start,
	/// Last position.
	End,
	/// Before position.
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
impl<S> Reducer<RichTextAction, S> for RichText
where
	S: BlockStorage + Clone + 'static,
{
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<RichTextAction>>,
		storage: &S,
	) -> Result<Link<Self>, anyhow::Error> {
		let event = storage.get_value(&event_link).await?;
		let mut state = storage.get_value_or_default(&state_link).await?;
		// TODO: replace event_link with actual head cid event_link has the risk of duplucates
		reduce(storage, &mut state, event_link.into(), event.payload).await?;
		Ok(storage.set_value(&state).await?)
	}
}
impl RichText {
	/// Stream characters with position and formatting.
	pub fn stream<S>(
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
		Ok(self
			.stream(storage.clone())
			.map_ok(|(char, _, _)| char)
			.try_collect::<String>()
			.await?)
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
		assert!(self.text.len() > 0);
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
	let mut transaction = Transaction { runs: state.runs.open_lazy(storage).await? };
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
		Ok(self
			.find_run(at)
			.await?
			.ok_or_else(|| anyhow!("InsertionPoint not found: {:?}", at))?)
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
	let attributes = match &action.attributes {
		AttributesOperation::Merge(attributes) => {
			if let Some(position) = insertion_point.position(state) {
				let run = transaction.get_run(position).await?;
				let mut run_attributes = storage.get_value_or_default(&run.attributes).await?;
				run_attributes.values.extend(attributes.values.clone());
				storage.set_value(&run_attributes).await?.into()
			} else {
				storage.set_value(attributes).await?.into()
			}
		},
		AttributesOperation::Replace(attributes) => storage.set_value(attributes).await?.into(),
		AttributesOperation::Remove => OptionLink::none(),
	};

	// create
	let create_run = Run { id, text: action.text, attributes, left: None, right: None, deleted: false };

	// find
	match insertion_point {
		InsertionPoint::Start => {
			// is empty?
			match state.left {
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
	let attributes = match action.attributes {
		AttributesOperation::Merge(attributes) => {
			let mut run_attributes = storage.get_value_or_default(&run.attributes).await?;
			run_attributes.values.extend(attributes.values.clone());
			run_attributes
		},
		AttributesOperation::Replace(attributes) => attributes,
		AttributesOperation::Remove => Attributes::default(),
	};
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
/// ```
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
/// ```
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

#[cfg(test)]
mod tests {
	use crate::{
		Attributes, AttributesOperation, DeleteAction, FormatAction, InsertAction, InsertionPoint, Position, RichText,
		RichTextAction, Run,
	};
	use cid::Cid;
	use co_api::{async_api::Reducer, BlockStorage, BlockStorageExt, CoTryStreamExt, Date, ReducerAction};
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
		let next_state_link = RichText::reduce(state_link.into(), action_link, storage).await.unwrap();
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
		assert_eq!(run.contains(Position(head, 0)), true);
		assert_eq!(run.contains(Position(head, 1)), true);
		assert_eq!(run.contains(Position(head, 2)), true);
		assert_eq!(run.contains(Position(head, 3)), true);
		assert_eq!(run.contains(Position(head, 4)), true);
		assert_eq!(run.contains(Position(head, 5)), false);
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
		let (_char, position, _attributes) = state.stream(storage.clone()).skip(5).try_first().await.unwrap().unwrap();
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
		let (_char, position, _attributes) = state.stream(storage.clone()).skip(5).try_first().await.unwrap().unwrap();
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
			.stream(storage.clone())
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
			.stream(storage.clone())
			.map_ok(|(char, _position, attributes)| (char, attributes))
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
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
			.stream(storage.clone())
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
