// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_api::{
	co, BlockStorageExt, CoList, CoMap, CoreBlockStorage, Date, Did, Link, OptionLink, Reducer, ReducerAction, Tags,
	TotalFloat64,
};
use futures::TryStreamExt;

#[co(state)]
pub struct DataSeries {
	/// The data points.
	pub data: CoMap<String, Series>,

	/// Aggregates.
	pub aggregates: CoMap<String, Aggregate>,
}

#[co]
pub struct Series {
	/// Metadata for this series.
	#[serde(default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,

	/// Data points. Sorted on data.time.
	pub data: CoList<Data>,

	/// Pending data points.
	pub pending_data: CoMap<String, Data>,

	/// Only keep series for specified amount of seconds.
	pub time_to_live: Option<u64>,
}

#[co]
pub struct Data {
	/// The data issuer.
	pub did: Did,

	/// The data time.
	/// When an complete tag is used this is the start time.
	pub time: Date,

	/// The value. Defaults to 1 if not used.
	#[serde(default = "default_value", skip_serializing_if = "is_default_value")]
	pub value: i32,

	/// Metadata for this data.
	/// Known Tags:
	/// * `complete: Date` - The date when the data point has been completed (to calulate duration using time).
	#[serde(default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,
}

fn default_value() -> i32 {
	1
}

fn is_default_value(value: &i32) -> bool {
	*value == 1
}

#[co]
pub struct Aggregate {
	series: String,

	/// Group By
	group: Option<AggregateGroup>,
	by: AggregateBy,

	/// Aggregated values. Sorted by date.
	values: CoList<AggregateValue>,
}

#[co]
pub struct AggregateValue {
	time: Date,
	count: u64,
	value: TotalFloat64,
}

#[co(repr)]
#[derive(Default)]
#[repr(u8)]
pub enum AggregateBy {
	/// Sum.
	#[default]
	Sum = 0,

	/// Average.
	Average = 1,
}

#[co(repr)]
#[repr(u8)]
pub enum AggregateGroup {
	/// By Minute.
	TimeMinute = 0,

	/// By Hour.
	TimeHour = 1,

	/// By Day.
	TimeDay = 2,

	/// By Week.
	TimeWeek = 3,

	/// By Month.
	TimeMonth = 4,

	/// By Year.
	TimeYear = 5,
}

/// Create a series.
#[co]
#[derive(Default)]
pub struct CreateSeriesPayload {
	pub series: String,
	pub tags: Tags,
	pub time_to_live: Option<u64>,
}

#[co]
#[derive(Default)]
pub struct DataPayload {
	pub series: String,
	pub pending_id: Option<String>,
	pub tags: Option<Tags>,
	pub time: Option<Date>,
	pub value: Option<i32>,
}

#[co]
#[derive(Default)]
pub struct PendingDataPayload {
	pub series: String,
	pub id: String,
	pub tags: Option<Tags>,
	pub time: Option<Date>,
	pub value: Option<i32>,
}

#[co]
#[derive(Default)]
pub struct CreateAggregatePayload {
	pub aggregate: String,
	pub series: String,
	pub group: Option<AggregateGroup>,
	pub by: AggregateBy,
}

#[co]
pub enum DataSeriesAction {
	/// Create a series.
	CreateSeries(CreateSeriesPayload),

	/// Remove a series.
	RemoveSeries { series: String },

	/// Insert Data.
	Data(DataPayload),

	/// Insert Pending Data.
	PendingData(PendingDataPayload),

	/// Cancel Pending Data.
	PendingCancel { series: String, id: String },

	/// Create Aggregate.
	CreateAggregate(CreateAggregatePayload),

	/// Remove Aggregate.
	RemoveAggregate { aggregate: String, series: String },
}

impl Reducer<DataSeriesAction> for DataSeries {
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<DataSeriesAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let action = storage.get_value(&event).await?;
		let mut result = storage.get_value_or_default(&state).await?;
		match &action.payload {
			DataSeriesAction::CreateSeries(payload) => reduce_create_series(storage, &mut result, payload).await?,
			DataSeriesAction::RemoveSeries { series } => reduce_remove_series(storage, &mut result, series).await?,
			DataSeriesAction::Data(payload) => {
				reduce_data(storage, &action.from, action.time, &mut result, payload).await?
			},
			DataSeriesAction::PendingData(PendingDataPayload { series, id, tags, time, value }) => {
				reduce_pending_data(storage, &action.from, action.time, &mut result, series, id, tags, time, value)
					.await?
			},
			DataSeriesAction::PendingCancel { series, id } => {
				reduce_pending_cancel(storage, &mut result, series, id).await?
			},
			DataSeriesAction::CreateAggregate(CreateAggregatePayload { aggregate, series, group, by }) => {
				reduce_create_aggregate(storage, &mut result, aggregate, series, *group, *by).await?
			},
			DataSeriesAction::RemoveAggregate { aggregate, series } => {
				reduce_remove_aggregate(storage, &mut result, series, aggregate).await?
			},
		}
		Ok(storage.set_value(&result).await?)
	}
}

async fn reduce_create_series(
	storage: &CoreBlockStorage,
	state: &mut DataSeries,
	payload: &CreateSeriesPayload,
) -> Result<(), anyhow::Error> {
	if !state.data.contains(storage, &payload.series).await? {
		let value = Series {
			tags: payload.tags.clone(),
			data: Default::default(),
			pending_data: Default::default(),
			time_to_live: payload.time_to_live,
		};
		state.data.insert(storage, payload.series.clone(), value).await?;
	}
	Ok(())
}

async fn reduce_remove_series(
	storage: &CoreBlockStorage,
	state: &mut DataSeries,
	series: &str,
) -> Result<(), anyhow::Error> {
	state.data.remove(storage, series.to_owned()).await?;

	// remove aggregates referencing this series
	let keys_to_remove: Vec<String> = state
		.aggregates
		.stream(storage)
		.try_filter_map(|(key, aggregate): (String, Aggregate)| async move {
			Ok(if aggregate.series == series { Some(key) } else { None })
		})
		.try_collect()
		.await?;
	for key in keys_to_remove {
		state.aggregates.remove(storage, key).await?;
	}
	Ok(())
}

async fn reduce_data(
	storage: &CoreBlockStorage,
	did: &Did,
	action_time: Date,
	state: &mut DataSeries,
	payload: &DataPayload,
) -> Result<(), anyhow::Error> {
	let Some(mut series) = state.data.get(storage, &payload.series).await? else {
		return Ok(());
	};

	// pending?
	let mut pending = None;
	if let Some(pending_id) = &payload.pending_id {
		pending = series.pending_data.remove(storage, pending_id.clone()).await?;
	}

	// data
	let data = match pending {
		Some(mut pending) => {
			if let Some(tags) = &payload.tags {
				pending.tags.extend(tags.iter().cloned());
			}
			pending.tags.set(co_api::tags!("completed": action_time as i128));
			pending
		},
		None => Data {
			did: did.clone(),
			time: payload.time.unwrap_or(action_time),
			tags: payload.tags.clone().unwrap_or_default(),
			value: payload.value.unwrap_or(1),
		},
	};

	// aggregate
	let aggregate_keys: Vec<(String, Aggregate)> = state
		.aggregates
		.stream(storage)
		.try_filter(|(_, value)| std::future::ready(value.series == payload.series))
		.try_collect()
		.await?;
	for (key, mut agg) in aggregate_keys {
		let mut values = agg.values.vec(storage, None).await?;

		// apply
		aggregate(agg.group, agg.by, &data, &mut values);

		// ttl
		if let Some(time_to_live) = series.time_to_live {
			let expire = action_time - time_to_live;
			values.retain(|item| item.time == 0 || item.time < expire);
		}

		agg.values = CoList::from_iter(storage, values).await?;
		state.aggregates.insert(storage, key, agg).await?;
	}

	// insert data at sorted position
	let mut items = series.data.vec(storage, None).await?;
	match find_next_index(items.iter().map(|item| &item.time), &data.time) {
		Some(index) => items.insert(index, data),
		None => items.push(data),
	}

	// ttl?
	if let Some(time_to_live) = series.time_to_live {
		let expire = action_time - time_to_live;
		items.retain(|item| item.time < expire);
	}

	series.data = CoList::from_iter(storage, items).await?;
	state.data.insert(storage, payload.series.clone(), series).await?;
	Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn reduce_pending_data(
	storage: &CoreBlockStorage,
	did: &Did,
	action_time: Date,
	state: &mut DataSeries,
	series_key: &str,
	id: &str,
	tags: &Option<Tags>,
	time: &Option<Date>,
	value: &Option<i32>,
) -> Result<(), anyhow::Error> {
	let Some(mut series) = state.data.get(storage, &series_key.to_owned()).await? else {
		return Ok(());
	};

	if !series.pending_data.contains(storage, &id.to_owned()).await? {
		let data = Data {
			did: did.clone(),
			tags: tags.clone().unwrap_or_default(),
			time: time.unwrap_or(action_time),
			value: value.unwrap_or(1),
		};
		series.pending_data.insert(storage, id.to_owned(), data).await?;
		state.data.insert(storage, series_key.to_owned(), series).await?;
	}
	Ok(())
}

async fn reduce_pending_cancel(
	storage: &CoreBlockStorage,
	state: &mut DataSeries,
	series_key: &str,
	id: &str,
) -> Result<(), anyhow::Error> {
	let Some(mut series) = state.data.get(storage, &series_key.to_owned()).await? else {
		return Ok(());
	};

	series.pending_data.remove(storage, id.to_owned()).await?;
	state.data.insert(storage, series_key.to_owned(), series).await?;
	Ok(())
}

async fn reduce_create_aggregate(
	storage: &CoreBlockStorage,
	state: &mut DataSeries,
	aggregate_key: &str,
	series_key: &str,
	group: Option<AggregateGroup>,
	by: AggregateBy,
) -> Result<(), anyhow::Error> {
	if state.aggregates.contains(storage, &aggregate_key.to_owned()).await? {
		return Ok(());
	}

	let Some(series) = state.data.get(storage, &series_key.to_owned()).await? else {
		return Ok(());
	};

	// calculate initial aggregates from existing data
	let mut values = Vec::new();
	let data_items = series.data.vec(storage, None).await?;
	for data in &data_items {
		aggregate(group, by, data, &mut values);
	}

	state
		.aggregates
		.insert(
			storage,
			aggregate_key.to_owned(),
			Aggregate { by, group, series: series_key.to_owned(), values: CoList::from_iter(storage, values).await? },
		)
		.await?;
	Ok(())
}

async fn reduce_remove_aggregate(
	storage: &CoreBlockStorage,
	state: &mut DataSeries,
	series_key: &str,
	aggregate_key: &str,
) -> Result<(), anyhow::Error> {
	if let Some(item) = state.aggregates.get(storage, &aggregate_key.to_owned()).await? {
		if item.series == series_key {
			state.aggregates.remove(storage, aggregate_key.to_owned()).await?;
		}
	}
	Ok(())
}

fn aggregate(group: Option<AggregateGroup>, by: AggregateBy, data: &Data, values: &mut Vec<AggregateValue>) {
	// bucket
	let bucket_time = match &group {
		Some(group) => {
			let group_seconds = match group {
				AggregateGroup::TimeMinute => 60,
				AggregateGroup::TimeHour => 60 * 60,
				AggregateGroup::TimeDay => 24 * 60 * 60,
				AggregateGroup::TimeWeek => 7 * 24 * 60 * 60,
				AggregateGroup::TimeMonth => 4 * 7 * 24 * 60 * 60,
				AggregateGroup::TimeYear => 365 * 24 * 60 * 60,
			};
			data.time - (data.time % group_seconds)
		},
		None => 0,
	};

	// get or insert bucket
	let value = match values.iter_mut().find(|item| item.time == bucket_time) {
		Some(value) => value,
		None => {
			let result = AggregateValue { count: 0, time: bucket_time, value: 0f64.into() };
			// insert as position
			let index = match find_next_index(values.iter().map(|item| &item.time), &data.time) {
				Some(index) => {
					values.insert(index, result);
					index
				},
				None => {
					let index = values.len();
					values.push(result);
					index
				},
			};
			values.get_mut(index).unwrap()
		},
	};

	// apply data
	match by {
		AggregateBy::Sum => {
			value.count += 1;
			value.value = (data.value as f64).into();
		},
		AggregateBy::Average => {
			// See: https://math.stackexchange.com/questions/22348/how-to-add-and-subtract-values-from-an-average
			value.count += 1;
			value.value =
				(value.value.value() + ((data.value as f64 - value.value.value()) / value.count as f64)).into();
		},
	}
}

fn find_next_index<T: PartialOrd>(values: impl Iterator<Item = T>, value: T) -> Option<usize> {
	values
		.enumerate()
		.filter(|(_index, item)| *item < value)
		.last()
		.map(|(index, _)| index + 1)
}

#[cfg(test)]
mod tests {
	use crate::find_next_index;

	#[test]
	fn test_find_next_index() {
		assert_eq!(Some(2), find_next_index([10, 20, 30].iter(), &21));
	}
}
