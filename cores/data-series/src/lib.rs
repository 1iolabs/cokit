use co_api::{reduce, Context, DagCollection, DagMap, DagVec, Date, Did, Reducer, ReducerAction, Tags, TotalFloat64};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataSeries {
	/// The data points.
	pub data: DagMap<String, Series>,

	/// Aggragates.
	pub aggregates: DagMap<String, Aggregate>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Series {
	/// Metadata for this series.
	#[serde(default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,

	/// Data points. Sorted on data.time.
	pub data: DagVec<Data>,

	/// Pending data points.
	pub pending_data: DagMap<String, Data>,

	/// Only keep series for specified amount of seconds.
	pub time_to_live: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
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

fn is_default_value(v: &i32) -> bool {
	*v == 1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Aggregate {
	series: String,

	/// Group By
	group: Option<AggregateGroup>,
	by: AggregateBy,

	/// Aggregated values. Sorted by date.
	values: DagVec<AggregateValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregateValue {
	time: Date,
	count: u64,
	value: TotalFloat64,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AggregateBy {
	/// Sum.
	#[default]
	Sum,

	/// Average.
	Average,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AggregateGroup {
	/// By Minute.
	TimeMinute,

	/// By Hour.
	TimeHour,

	/// By Day.
	TimeDay,

	/// By Week.
	TimeWeek,

	/// By Month.
	TimeMonth,

	/// By Year.
	TimeYear,
}

/// Create a series.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateSeriesPayload {
	pub series: String,
	pub tags: Tags,
	pub time_to_live: Option<u64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataPayload {
	pub series: String,
	pub pending_id: Option<String>,
	pub tags: Option<Tags>,
	pub time: Option<Date>,
	pub value: Option<i32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingDataPayload {
	pub series: String,
	pub id: String,
	pub tags: Option<Tags>,
	pub time: Option<Date>,
	pub value: Option<i32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateAggregatePayload {
	pub aggregate: String,
	pub series: String,
	pub group: Option<AggregateGroup>,
	pub by: AggregateBy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl Reducer for DataSeries {
	type Action = DataSeriesAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
		match &event.payload {
			DataSeriesAction::CreateSeries(payload) => reduce_create_series(context, self, payload),
			DataSeriesAction::RemoveSeries { series } => reduce_remove_series(context, self, series),
			DataSeriesAction::Data(payload) => reduce_data(context, &event.from, event.time, self, payload),
			DataSeriesAction::PendingData(PendingDataPayload { series, id, tags, time, value }) => {
				reduce_pending_data(context, &event.from, event.time, self, series, id, tags, time, value)
			},
			DataSeriesAction::PendingCancel { series, id } => reduce_pending_cancel(context, self, series, id),
			DataSeriesAction::CreateAggregate(CreateAggregatePayload { aggregate, series, group, by }) => {
				reduce_create_aggregate(context, self, aggregate, series, *group, *by)
			},
			DataSeriesAction::RemoveAggregate { aggregate, series } => {
				reduce_remove_aggregate(context, self, series, aggregate)
			},
		}
	}
}

fn reduce_create_series(context: &mut dyn Context, mut state: DataSeries, payload: &CreateSeriesPayload) -> DataSeries {
	state.data.update(context, |_context, data| {
		if !data.contains_key(&payload.series) {
			let value = Series {
				tags: payload.tags.clone(),
				data: Default::default(),
				pending_data: Default::default(),
				time_to_live: payload.time_to_live,
			};
			data.insert(payload.series.to_owned(), value);
		}
	});
	state
}

fn reduce_remove_series(context: &mut dyn Context, mut state: DataSeries, series: &str) -> DataSeries {
	state.data.update(context, |_context, data| {
		data.remove(series);
	});
	state.aggregates.update_owned(context, |_context, aggregates| {
		aggregates.into_iter().filter(|(_key, value)| value.series != series).collect()
	});
	state
}

fn reduce_data(
	context: &mut dyn Context,
	did: &Did,
	action_time: Date,
	mut state: DataSeries,
	payload: &DataPayload,
) -> DataSeries {
	state.data.update(context, |context, data| {
		if let Some(series) = data.get_mut(&payload.series) {
			// pending?
			let mut pending = None;
			if let Some(pending_id) = &payload.pending_id {
				series
					.pending_data
					.update(context, |_context, pending_data| pending = pending_data.remove(pending_id));
			}

			// data
			let data_time = payload.time.unwrap_or(action_time);
			let data = match pending {
				Some(mut pending) => {
					if let Some(tags) = &payload.tags {
						pending.tags.extend(tags.iter().cloned());
					}
					pending.tags.set(co_api::tags!("completed": data_time as i128));
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
			state.aggregates.update(context, |context, aggregates| {
				for (_, value) in aggregates.iter_mut() {
					if value.series == payload.series {
						let group = value.group;
						let by = value.by;
						value.values.update_owned(context, |_, mut values| {
							// apply
							aggregate(group, by, &data, &mut values);

							// ttl
							if let Some(time_to_live) = series.time_to_live {
								let expire = action_time - time_to_live as u128;
								values.retain(|item| item.time == 0 || item.time < expire);
							}

							// result
							values
						});
					}
				}
			});

			// insert
			series.data.update_owned(context, |_context, mut items| {
				// insert as position
				match find_next_index(items.iter().map(|item| &item.time), &data.time) {
					Some(index) => {
						items.insert(index, data);
					},
					None => {
						items.push(data);
					},
				}

				// ttl?
				if let Some(time_to_live) = series.time_to_live {
					let expire = action_time - time_to_live as u128;
					items.retain(|item| item.time < expire);
				}

				// result
				items
			});
		}
	});
	state
}

fn reduce_pending_data(
	context: &mut dyn Context,
	did: &Did,
	action_time: Date,
	mut state: DataSeries,
	series_key: &str,
	id: &str,
	tags: &Option<Tags>,
	time: &Option<Date>,
	value: &Option<i32>,
) -> DataSeries {
	state.data.update(context, |context, data| {
		if let Some(series) = data.get_mut(series_key) {
			series.pending_data.update(context, |_context, pending_data| {
				if !pending_data.contains_key(id) {
					let data = Data {
						did: did.clone(),
						tags: tags.clone().unwrap_or_default(),
						time: time.unwrap_or(action_time),
						value: value.unwrap_or(1),
					};
					pending_data.insert(id.to_owned(), data);
				}
			});
		}
	});
	state
}

fn reduce_pending_cancel(context: &mut dyn Context, mut state: DataSeries, series_key: &str, id: &str) -> DataSeries {
	state.data.update(context, |context, data| {
		if let Some(series) = data.get_mut(series_key) {
			series.pending_data.update(context, |_context, pending_data| {
				pending_data.remove(id);
			});
		}
	});
	state
}

fn reduce_create_aggregate(
	context: &mut dyn Context,
	mut state: DataSeries,
	aggregate_key: &str,
	series_key: &str,
	group: Option<AggregateGroup>,
	by: AggregateBy,
) -> DataSeries {
	if !state.aggregates.iter(context.storage()).any(|(key, _)| key == aggregate_key) {
		let item = state.data.iter(context.storage()).find(|(key, _)| key == series_key);
		if let Some((_, series)) = item {
			// calculate
			let mut values = Vec::new();
			for data in series.data.iter(context.storage()) {
				aggregate(group, by, &data, &mut values);
			}

			// insert
			state.aggregates.update(context, |context, aggregates| {
				aggregates.insert(
					aggregate_key.to_owned(),
					Aggregate {
						by,
						group,
						series: series_key.to_owned(),
						values: DagVec::create(context.storage_mut(), values),
					},
				);
			});
		}
	}
	state
}

fn reduce_remove_aggregate(
	context: &mut dyn Context,
	mut state: DataSeries,
	series_key: &str,
	aggregate_key: &str,
) -> DataSeries {
	state.aggregates.update(context, |_context, aggregates| {
		if aggregates.get(aggregate_key).map_or(false, |item| item.series == series_key) {
			aggregates.remove(aggregate_key);
		}
	});
	state
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
			value.value = (value.value.0 + ((data.value as f64 - value.value.0) / value.count as f64)).into();
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

#[no_mangle]
pub extern "C" fn state() {
	reduce::<DataSeries>()
}

#[cfg(test)]
mod tests {
	use crate::find_next_index;

	#[test]
	fn test_find_next_index() {
		assert_eq!(Some(2), find_next_index([10, 20, 30].iter(), &21));
	}
}
