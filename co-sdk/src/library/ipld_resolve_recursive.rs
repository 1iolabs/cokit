// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{BlockStorage, BlockStorageExt, MultiCodec};
use cid::Cid;
use ipld_core::ipld::Ipld;

/// Maximum number of CID link resolutions (prevents cycles or excessively deep DAGs).
const MAX_LINK_DEPTH: usize = 256;

/// Maximum total IPLD nodes processed (prevents memory exhaustion on very wide structures).
const MAX_NODES: usize = 1_000_000;

enum Work {
	Resolve(Ipld, usize),
	CollectList(usize),
	CollectMap(Vec<String>),
	WrapLink(Cid),
}

pub async fn ipld_resolve_recursive(
	storage: &impl BlockStorage,
	node: Ipld,
	keep_link: bool,
) -> Result<Ipld, anyhow::Error> {
	let mut work_stack = vec![Work::Resolve(node, 0)];
	let mut result_stack: Vec<Ipld> = Vec::new();
	let mut nodes_processed: usize = 0;

	while let Some(work) = work_stack.pop() {
		match work {
			Work::Resolve(ipld, link_depth) => {
				nodes_processed += 1;
				if nodes_processed > MAX_NODES {
					tracing::warn!(
						"ipld_resolve_recursive: exceeded maximum node count ({MAX_NODES}), returning as-is"
					);
					result_stack.push(ipld);
					continue;
				}

				match ipld {
					Ipld::List(items) => {
						let len = items.len();
						work_stack.push(Work::CollectList(len));
						for item in items.into_iter().rev() {
							work_stack.push(Work::Resolve(item, link_depth));
						}
					},
					Ipld::Map(entries) => {
						let mut keys = Vec::with_capacity(entries.len());
						let mut values = Vec::with_capacity(entries.len());
						for (k, v) in entries {
							keys.push(k);
							values.push(v);
						}
						work_stack.push(Work::CollectMap(keys));
						for value in values.into_iter().rev() {
							work_stack.push(Work::Resolve(value, link_depth));
						}
					},
					Ipld::Link(cid) => {
						if MultiCodec::is_cbor(cid) {
							if link_depth >= MAX_LINK_DEPTH {
								tracing::warn!(%cid, "ipld_resolve_recursive: exceeded maximum link depth ({MAX_LINK_DEPTH}), returning link as-is");
								result_stack.push(Ipld::Link(cid));
								continue;
							}
							match storage.get_deserialized::<Ipld>(&cid).await {
								Ok(resolved) => {
									if keep_link {
										work_stack.push(Work::WrapLink(cid));
									}
									work_stack.push(Work::Resolve(resolved, link_depth + 1));
								},
								Err(err) => {
									tracing::warn!(%err, ?cid, "ipld_resolve_recursive");
									result_stack.push(Ipld::Link(cid));
								},
							}
						} else {
							result_stack.push(Ipld::Link(cid));
						}
					},
					other => result_stack.push(other),
				}
			},
			Work::CollectList(len) => {
				let start = result_stack.len() - len;
				let items = result_stack.drain(start..).collect();
				result_stack.push(Ipld::List(items));
			},
			Work::CollectMap(keys) => {
				let start = result_stack.len() - keys.len();
				let values = result_stack.drain(start..);
				let map = keys.into_iter().zip(values).collect();
				result_stack.push(Ipld::Map(map));
			},
			Work::WrapLink(cid) => {
				let resolved = result_stack.pop().expect("WrapLink requires a resolved value");
				result_stack.push(Ipld::List(vec![Ipld::Link(cid), resolved]));
			},
		}
	}

	Ok(result_stack
		.pop()
		.expect("ipld_resolve_recursive should produce exactly one result"))
}
