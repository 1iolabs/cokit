// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use std::path::PathBuf;

/// A single sink kind within `CO_LOG`.
#[derive(Debug, Clone, PartialEq)]
pub enum LogSink {
	/// stderr (`stderr` / `-`).
	Stderr,
	/// A file: `File(None)` = default path (`file`); `File(Some(p))` = explicit path.
	File(Option<PathBuf>),
	/// Apple unified logging (`oslog` / `os`).
	Oslog,
}

/// One `CO_LOG` entry: a sink plus its optional inline `EnvFilter` directives.
#[derive(Debug, Clone, PartialEq)]
pub struct SinkSpec {
	pub sink: LogSink,
	pub filter: Option<String>,
}

/// The parsed `CO_LOG` value.
#[derive(Debug, Clone, PartialEq)]
pub enum LogConfig {
	/// `off` / `0` / `false` — no logging.
	Off,
	/// `on` / `1` / `true` — platform default sinks.
	Default,
	/// One or more explicit sink entries.
	Sinks(Vec<SinkSpec>),
}

/// Parse `CO_LOG`: a `;`-separated list of `sink[:envfilter]` entries, or the whole-value keywords
/// `off`/`on`. Each entry splits on the first `:` into a sink and its inline `EnvFilter` directives.
/// Unknown bare sink tokens (and empty entries) are rejected.
pub fn parse_log(s: &str) -> Result<LogConfig, String> {
	match s.trim().to_ascii_lowercase().as_str() {
		"0" | "false" | "off" => return Ok(LogConfig::Off),
		"1" | "true" | "on" => return Ok(LogConfig::Default),
		_ => {},
	}
	let mut sinks = Vec::new();
	for entry in s.split(';') {
		let entry = entry.trim();
		if entry.is_empty() {
			return Err(format!("empty CO_LOG entry in '{s}'"));
		}
		let (sink_tok, filter) = match entry.split_once(':') {
			Some((sink, f)) => {
				let f = f.trim();
				(sink.trim(), if f.is_empty() { None } else { Some(f.to_owned()) })
			},
			None => (entry, None),
		};
		sinks.push(SinkSpec { sink: parse_sink(sink_tok)?, filter });
	}
	Ok(LogConfig::Sinks(sinks))
}

fn parse_sink(s: &str) -> Result<LogSink, String> {
	match s.to_ascii_lowercase().as_str() {
		"stderr" | "-" => return Ok(LogSink::Stderr),
		"file" => return Ok(LogSink::File(None)),
		"oslog" | "os" => return Ok(LogSink::Oslog),
		_ => {},
	}
	if is_path_like(s) {
		Ok(LogSink::File(Some(PathBuf::from(s))))
	} else {
		Err(format!("unknown CO_LOG sink '{s}'; expected stderr, file, oslog, or a path"))
	}
}

/// Choose a sink's effective directives: its inline filter if non-empty, else `RUST_LOG` if
/// non-empty, else none (the builder then defaults to `info`).
pub fn resolve_filter(inline: Option<&str>, rust_log: Option<&str>) -> Option<String> {
	inline
		.filter(|s| !s.is_empty())
		.or(rust_log.filter(|s| !s.is_empty()))
		.map(str::to_owned)
}

/// A value is a path only if it clearly looks like one (absolute, `./`/`../`, `~`, or contains `/`).
fn is_path_like(s: &str) -> bool {
	s.starts_with('/') || s.starts_with("./") || s.starts_with("../") || s.starts_with('~') || s.contains('/')
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn whole_value_keywords() {
		assert_eq!(parse_log("off").unwrap(), LogConfig::Off);
		assert_eq!(parse_log("0").unwrap(), LogConfig::Off);
		assert_eq!(parse_log("false").unwrap(), LogConfig::Off);
		assert_eq!(parse_log("on").unwrap(), LogConfig::Default);
		assert_eq!(parse_log("1").unwrap(), LogConfig::Default);
		assert_eq!(parse_log("TRUE").unwrap(), LogConfig::Default);
	}

	#[test]
	fn single_sinks() {
		assert_eq!(
			parse_log("file").unwrap(),
			LogConfig::Sinks(vec![SinkSpec { sink: LogSink::File(None), filter: None }])
		);
		assert_eq!(
			parse_log("stderr").unwrap(),
			LogConfig::Sinks(vec![SinkSpec { sink: LogSink::Stderr, filter: None }])
		);
		assert_eq!(parse_log("-").unwrap(), LogConfig::Sinks(vec![SinkSpec { sink: LogSink::Stderr, filter: None }]));
		assert_eq!(
			parse_log("/var/log/co.log").unwrap(),
			LogConfig::Sinks(vec![SinkSpec {
				sink: LogSink::File(Some(PathBuf::from("/var/log/co.log"))),
				filter: None
			}])
		);
	}

	#[test]
	fn sink_with_inline_filter() {
		assert_eq!(
			parse_log("stderr:error").unwrap(),
			LogConfig::Sinks(vec![SinkSpec { sink: LogSink::Stderr, filter: Some("error".into()) }])
		);
		assert_eq!(
			parse_log("/var/log/co.log:debug").unwrap(),
			LogConfig::Sinks(vec![SinkSpec {
				sink: LogSink::File(Some(PathBuf::from("/var/log/co.log"))),
				filter: Some("debug".into())
			}])
		);
	}

	#[test]
	fn multi_sink_with_filters() {
		assert_eq!(
			parse_log("file:info,co_sdk=trace;stderr:error").unwrap(),
			LogConfig::Sinks(vec![
				SinkSpec { sink: LogSink::File(None), filter: Some("info,co_sdk=trace".into()) },
				SinkSpec { sink: LogSink::Stderr, filter: Some("error".into()) },
			])
		);
	}

	#[test]
	fn trailing_colon_is_no_filter() {
		assert_eq!(
			parse_log("file:").unwrap(),
			LogConfig::Sinks(vec![SinkSpec { sink: LogSink::File(None), filter: None }])
		);
	}

	#[test]
	fn rejects_unknown_and_empty() {
		assert!(parse_log("debug").is_err());
		assert!(parse_log("co.log").is_err());
		assert!(parse_log("").is_err());
		assert!(parse_log("file;;stderr").is_err());
	}

	#[test]
	fn parse_oslog() {
		assert_eq!(
			parse_log("oslog").unwrap(),
			LogConfig::Sinks(vec![SinkSpec { sink: LogSink::Oslog, filter: None }])
		);
		assert_eq!(
			parse_log("os:debug").unwrap(),
			LogConfig::Sinks(vec![SinkSpec { sink: LogSink::Oslog, filter: Some("debug".into()) }])
		);
	}

	#[test]
	fn resolve_filter_cases() {
		assert_eq!(resolve_filter(None, None), None);
		assert_eq!(resolve_filter(Some("a=b"), None), Some("a=b".into()));
		assert_eq!(resolve_filter(None, Some("c=d")), Some("c=d".into()));
		assert_eq!(resolve_filter(Some("a=b"), Some("c=d")), Some("a=b".into()));
		assert_eq!(resolve_filter(Some(""), Some("c=d")), Some("c=d".into()));
	}
}
