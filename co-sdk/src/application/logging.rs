// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use std::path::PathBuf;

/// The requested log sink, parsed from `CO_LOG` / `--log`.
///
/// Platform-agnostic: consumers map it to their own resolved sink and degrade targets that are not
/// compiled in on the current platform.
#[derive(Debug, Clone, PartialEq)]
pub enum LogSink {
	/// Disable logging (`0` / `false` / `off`).
	Off,
	/// Platform default logging (`1` / `true` / `on`).
	Default,
	/// Log to a file at the default path (`file`).
	File,
	/// Log to stderr (`-` / `stderr`).
	Stderr,
	/// Log to a file at the given path.
	Path(PathBuf),
}

/// Parse a `CO_LOG` / `--log` value with a strict grammar: a keyword (`on`/`off`/`file`/`stderr`)
/// or a clear path. Unknown bare words are rejected so that typos and `RUST_LOG`-style values
/// (e.g. `CO_LOG=debug`) error instead of silently creating files.
pub fn parse_log_sink(s: &str) -> Result<LogSink, String> {
	match s.to_ascii_lowercase().as_str() {
		"0" | "false" | "off" => return Ok(LogSink::Off),
		"1" | "true" | "on" => return Ok(LogSink::Default),
		"file" => return Ok(LogSink::File),
		"-" | "stderr" => return Ok(LogSink::Stderr),
		_ => {},
	}
	if is_path_like(s) {
		Ok(LogSink::Path(PathBuf::from(s)))
	} else {
		Err(format!("unknown CO_LOG value '{s}'; expected on/off/file/stderr or a path"))
	}
}

/// Choose the `EnvFilter` override directives: `filter` (`CO_LOG_FILTER`) if non-empty, otherwise
/// `rust_log` (`RUST_LOG`) if non-empty, otherwise none. The level (applied separately as the
/// default directive) is not included here.
pub fn resolve_filter(filter: Option<&str>, rust_log: Option<&str>) -> Option<String> {
	filter
		.filter(|s| !s.is_empty())
		.or(rust_log.filter(|s| !s.is_empty()))
		.map(str::to_owned)
}

/// A value is treated as a path only if it clearly looks like one (absolute, relative `./`/`../`,
/// home `~`, or containing a `/`). A bare filename like `co.log` must be written `./co.log`.
fn is_path_like(s: &str) -> bool {
	s.starts_with('/') || s.starts_with("./") || s.starts_with("../") || s.starts_with('~') || s.contains('/')
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_keywords() {
		assert_eq!(parse_log_sink("0").unwrap(), LogSink::Off);
		assert_eq!(parse_log_sink("false").unwrap(), LogSink::Off);
		assert_eq!(parse_log_sink("off").unwrap(), LogSink::Off);
		assert_eq!(parse_log_sink("OFF").unwrap(), LogSink::Off);
		assert_eq!(parse_log_sink("1").unwrap(), LogSink::Default);
		assert_eq!(parse_log_sink("true").unwrap(), LogSink::Default);
		assert_eq!(parse_log_sink("on").unwrap(), LogSink::Default);
		assert_eq!(parse_log_sink("file").unwrap(), LogSink::File);
		assert_eq!(parse_log_sink("FILE").unwrap(), LogSink::File);
		assert_eq!(parse_log_sink("-").unwrap(), LogSink::Stderr);
		assert_eq!(parse_log_sink("stderr").unwrap(), LogSink::Stderr);
	}

	#[test]
	fn parse_paths() {
		assert_eq!(parse_log_sink("/var/log/co.log").unwrap(), LogSink::Path(PathBuf::from("/var/log/co.log")));
		assert_eq!(parse_log_sink("./co.log").unwrap(), LogSink::Path(PathBuf::from("./co.log")));
		assert_eq!(parse_log_sink("../x/co.log").unwrap(), LogSink::Path(PathBuf::from("../x/co.log")));
		assert_eq!(parse_log_sink("~/co.log").unwrap(), LogSink::Path(PathBuf::from("~/co.log")));
		assert_eq!(parse_log_sink("logs/co.log").unwrap(), LogSink::Path(PathBuf::from("logs/co.log")));
	}

	#[test]
	fn parse_rejects_bare_words() {
		assert!(parse_log_sink("debug").is_err());
		assert!(parse_log_sink("trace").is_err());
		assert!(parse_log_sink("ture").is_err());
		assert!(parse_log_sink("co.log").is_err());
		assert!(parse_log_sink("").is_err());
	}

	#[test]
	fn resolve_filter_cases() {
		assert_eq!(resolve_filter(None, None), None);
		assert_eq!(resolve_filter(Some("co_sdk=debug"), None), Some("co_sdk=debug".to_owned()));
		assert_eq!(resolve_filter(None, Some("co_sdk=trace")), Some("co_sdk=trace".to_owned()));
		assert_eq!(resolve_filter(Some("a=b"), Some("c=d")), Some("a=b".to_owned()));
		assert_eq!(resolve_filter(Some(""), None), None);
		assert_eq!(resolve_filter(Some(""), Some("c=d")), Some("c=d".to_owned()));
	}
}
