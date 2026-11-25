//! Authority application diagnostics. Configured rotations retain complete
//! JSON lines under the declared authority directory.
use crate::config::DiagnosticsConfig;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Stable file name within the installation's declared configuration directory.
pub const LOG_FILE: &str = "authority.log";

/// One log sink. The mutex serializes writes, archiving, reads and policy replacement.
pub struct Diag {
    path: PathBuf,
    state: Mutex<(&'static str, DiagnosticsConfig)>,
}

/// A protected read query. With only a literal search and exact level, clients
/// retain the original latest-batch response. An extended option selects the
/// paginated history response and enables the optional consistency witness.
#[derive(Debug, Default)]
pub struct LogQuery {
    search: String,
    exact: Option<&'static str>,
    minimum: Option<&'static str>,
    history: Option<History>,
}

#[derive(Debug, Default)]
struct History {
    position: Position,
    source: LogSource,
    oldest_first: bool,
    offset: usize,
    limit: Option<usize>,
    window_bytes: Option<u64>,
    snapshot: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum Position {
    Start,
    #[default]
    End,
    All,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum LogSource {
    #[default]
    All,
    Active,
    Archives,
}

/// Metadata for a source observed under the diagnostic writer lock. Archive
/// timestamps are projected as seconds and filenames stay internal.
struct RetainedLog {
    path: PathBuf,
    bytes: u64,
    modified: std::time::Duration,
    rotation_at: Option<u64>,
}

const FILTER_INVALID: &str = "authority-log-filter-invalid";
const DIAG_IO: &str = "authority-diag-io";
/// The caller's previously observed set of retained bytes is no longer current.
pub const SNAPSHOT_STALE: &str = "authority-log-snapshot-stale";

impl LogQuery {
    /// Validate one decoded query. Duplicate or unknown fields, conflicting
    /// level filters, blank numeric values and invalid snapshot syntax refuse.
    /// The effective row and byte limits are checked under the sink lock.
    pub fn parse(query: &[(String, String)]) -> Result<Self, &'static str> {
        let mut parsed = Self::default();
        let mut seen = std::collections::BTreeSet::new();
        for (key, value) in query {
            if !seen.insert(key.as_str()) {
                return Err(FILTER_INVALID);
            }
            match key.as_str() {
                "q" => parsed.search.clone_from(value),
                "level" => parsed.exact = Some(level_static(value).ok_or(FILTER_INVALID)?),
                "minimum_level" => {
                    parsed.minimum = Some(level_static(value).ok_or(FILTER_INVALID)?);
                    parsed.history.get_or_insert_with(History::default);
                }
                _ => {
                    let history = parsed.history.get_or_insert_with(History::default);
                    match key.as_str() {
                        "position" => history.position = match value.as_str() {
                            "start" => Position::Start,
                            "end" => Position::End,
                            "all" => Position::All,
                            _ => return Err(FILTER_INVALID),
                        },
                        "source" => history.source = match value.as_str() {
                            "all" => LogSource::All,
                            "active" => LogSource::Active,
                            "archives" => LogSource::Archives,
                            _ => return Err(FILTER_INVALID),
                        },
                        "order" => history.oldest_first = match value.as_str() {
                            "oldest" => true,
                            "newest" => false,
                            _ => return Err(FILTER_INVALID),
                        },
                        "offset" => history.offset = digits(value)?,
                        "limit" => history.limit = Some(digits(value)?),
                        "window_bytes" => history.window_bytes = Some(digits(value)?),
                        "snapshot" if value.len()==64 && value.bytes().all(|c|c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) => {
                            history.snapshot = Some(value.clone());
                        }
                        _ => return Err(FILTER_INVALID),
                    }
                }
            }
        }
        if parsed.exact.is_some() && parsed.minimum.is_some() {
            return Err(FILTER_INVALID);
        }
        if parsed.history.as_ref().is_some_and(|h| h.position==Position::All && h.window_bytes.is_some()) {
            return Err(FILTER_INVALID);
        }
        Ok(parsed)
    }

    /// Match the literal query and the selected exact or minimum severity.
    /// A line with an invalid level cannot satisfy a severity selection.
    fn matches(&self, line: &str) -> Result<bool, &'static str> {
        if !line.contains(&self.search) {
            return Ok(false);
        }
        let Some(level) = self.exact.or(self.minimum) else {
            return Ok(true);
        };
        let entry: serde_json::Value = serde_json::from_str(line).map_err(|_| DIAG_IO)?;
        let actual = entry.get("level").and_then(serde_json::Value::as_str).and_then(level_static);
        Ok(actual.is_some_and(|value| match self.minimum {
            Some(_) => rank(value) <= rank(level),
            None => value == level,
        }))
    }
}

/// Parse unsigned ASCII decimal values without signs or whitespace.
fn digits<T: std::str::FromStr>(value: &str) -> Result<T, &'static str> {
    if value.is_empty() || !value.bytes().all(|c|c.is_ascii_digit()) {
        return Err(FILTER_INVALID);
    }
    value.parse().map_err(|_| FILTER_INVALID)
}

impl Diag {
    /// Use the declared directory, validated minimum level and effective policy.
    /// Opening the sink never creates or moves an existing log.
    pub fn open(dir: &Path, level: &'static str, policy: DiagnosticsConfig) -> Self {
        Self { path: dir.join(LOG_FILE), state: Mutex::new((level, policy)) }
    }

    /// Apply the new saved policy to later reads and writes. Rotation and archive
    /// trimming happen only before a qualifying append, never during a settings edit.
    pub fn configure(&self, level: &'static str, policy: DiagnosticsConfig) -> Result<(), &'static str> {
        let mut state = self.state.lock().map_err(|_| "authority-diag-io")?;
        *state = (level, policy);
        Ok(())
    }

    /// Append one JSON line when it passes the level. Diagnostics failures
    /// go to stderr and never change the originating request result.
    pub fn log(&self, level: &str, message: &str, fields: serde_json::Value) {
        let result = (|| {
            let state = self.state.lock().map_err(|_| std::io::Error::other("diagnostics lock"))?;
            if rank(level) > rank(state.0) {
                return Ok(());
            }
            let mut obj = serde_json::json!({
                "at": crate::server::now(),
                "level": level,
                "message": message,
            });
            if let (Some(dst), Some(src)) = (obj.as_object_mut(), fields.as_object()) {
                for (key, value) in src {
                    dst.insert(key.clone(), value.clone());
                }
            }
            self.append(&obj.to_string(), state.1)
        })();
        if let Err(e) = result {
            eprintln!("{}", serde_json::json!({"schema_version": 1, "warning": "authority-diag-unavailable", "error": e.to_string()}));
        }
    }

    /// Enumerate only this sink's regular, timestamp-named archives. A malformed
    /// name is outside the retention namespace and remains untouched.
    fn archives(&self) -> std::io::Result<Vec<PathBuf>> {
        let dir = self.path.parent().ok_or_else(|| std::io::Error::other("diagnostics directory"))?;
        let prefix = format!("{LOG_FILE}.");
        let mut archives = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            if name.to_str().and_then(|name| name.strip_prefix(&prefix))
                .is_some_and(|suffix| suffix.len() == 39 && suffix.bytes().all(|b| b.is_ascii_digit()))
                && entry.file_type()?.is_file() {
                archives.push(entry.path());
            }
        }
        archives.sort_unstable();
        Ok(archives)
    }

    /// Rotate the active log by same-directory rename. The old bytes survive a
    /// failed append. Refuse an oversized event before rotating anything.
    fn append(&self, line: &str, policy: DiagnosticsConfig) -> std::io::Result<()> {
        let bytes = line.len() as u64 + 1;
        if bytes > policy.max_bytes {
            return Err(std::io::Error::other("diagnostics entry exceeds configured file size"));
        }
        let size = match std::fs::metadata(&self.path) {
            Ok(meta) => meta.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(error),
        };
        if size > policy.max_bytes - bytes {
            let time = SystemTime::now().duration_since(UNIX_EPOCH)
                .map_err(std::io::Error::other)?.as_nanos();
            // Keep chronological order even if the wall clock has moved backwards.
            let observed = self.archives()?;
            let next = observed.last().and_then(|path| path.extension())
                .and_then(|suffix| suffix.to_str()).and_then(|suffix| suffix.parse::<u128>().ok())
                .map(|last| last.checked_add(1).ok_or_else(|| std::io::Error::other("diagnostics archive sequence")))
                .transpose()?.unwrap_or(time).max(time);
            let archive = self.path.with_file_name(format!("{LOG_FILE}.{next:039}"));
            if archive.try_exists()? {
                return Err(std::io::Error::other("diagnostics archive already exists"));
            }
            std::fs::rename(&self.path, archive)?;
            let archives = self.archives()?;
            for stale in archives.iter().take(archives.len().saturating_sub(policy.archives)) {
                std::fs::remove_file(stale)?;
            }
        }
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()
    }

    /// Return the latest matching entries across retained archives and active
    /// log, oldest first. The buffer holds at most the configured row count.
    /// File, line and lock errors refuse instead of returning partial history.
    pub fn page(&self, search: &str, level: Option<&str>) -> Result<serde_json::Value, &'static str> {
        let state = self.state.lock().map_err(|_| "authority-diag-io")?;
        let mut files = self.archives().map_err(|_| "authority-diag-io")?;
        let archives = files.len();
        files.push(self.path.clone());
        let mut lines = VecDeque::new();
        for file in files {
            let reader = match std::fs::File::open(&file) {
                Ok(file) => BufReader::new(file),
                Err(error) if file == self.path && error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => return Err("authority-diag-io"),
            };
            for line in reader.lines() {
                let line = line.map_err(|_| "authority-diag-io")?;
                if !line.contains(search) {
                    continue;
                }
                if let Some(level) = level {
                    let entry: serde_json::Value = serde_json::from_str(&line).map_err(|_| "authority-diag-io")?;
                    if entry.get("level").and_then(serde_json::Value::as_str) != Some(level) {
                        continue;
                    }
                }
                if lines.len() == state.1.page_rows {
                    lines.pop_front();
                }
                lines.push_back(line);
            }
        }
        Ok(serde_json::json!({"level":state.0,"lines":lines,"max_bytes":state.1.max_bytes,
            "diagnostics":state.1,"archives":archives,"log_levels":crate::config::LOG_LEVELS}))
    }

    /// Read a protected log query. The legacy shape stays compatible. Paged
    /// reads retain only one page of complete lines and refuse stale snapshots.
    pub fn query(&self, query: &LogQuery) -> Result<serde_json::Value, &'static str> {
        let Some(history) = &query.history else {
            let mut response = self.page(&query.search, query.exact)?;
            response["history_version"] = 1.into();
            response["rotation_version"] = 1.into();
            return Ok(response);
        };
        let state = self.state.lock().map_err(|_| DIAG_IO)?;
        let limit = history.limit.unwrap_or(state.1.page_rows);
        let size = history.window_bytes.unwrap_or(state.1.max_bytes);
        if limit == 0 || limit > state.1.page_rows || size == 0 || size > state.1.max_bytes {
            return Err(FILTER_INVALID);
        }
        let end = history.offset.checked_add(limit).ok_or(FILTER_INVALID)?;
        let mut paths = self.archives().map_err(|_| DIAG_IO)?;
        let archives = paths.len();
        paths.push(self.path.clone());
        let mut retained = Vec::new();
        let mut active_bytes = 0u64;
        let mut archived_bytes = 0u64;
        let mut last_rotation_at = None;
        for path in paths {
            let metadata = match std::fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if path==self.path && error.kind()==std::io::ErrorKind::NotFound => continue,
                Err(_) => return Err(DIAG_IO),
            };
            let modified = metadata.modified().map_err(|_| DIAG_IO)?
                .duration_since(UNIX_EPOCH).map_err(|_| DIAG_IO)?;
            let rotation_at = if path == self.path {
                active_bytes = metadata.len();
                None
            } else {
                archived_bytes = archived_bytes.checked_add(metadata.len()).ok_or(DIAG_IO)?;
                let stamp = path.extension().and_then(|value| value.to_str())
                    .and_then(|value| value.parse::<u128>().ok()).ok_or(DIAG_IO)?;
                let time = u64::try_from(stamp / std::time::Duration::from_secs(1).as_nanos()).map_err(|_| DIAG_IO)?;
                last_rotation_at = Some(time);
                Some(time)
            };
            retained.push(RetainedLog { path, bytes: metadata.len(), modified, rotation_at });
        }
        let files: Vec<_> = retained.iter().filter(|file|match history.source {
            LogSource::All => true,
            LogSource::Active => file.rotation_at.is_none(),
            LogSource::Archives => file.rotation_at.is_some(),
        }).collect();
        let mut revision = Sha256::new();
        let mut available_bytes = 0u64;
        let mut newest_at = None;
        for file in &files {
            let name = file.path.file_name().and_then(|name| name.to_str()).ok_or(DIAG_IO)?;
            revision.update((name.len() as u64).to_le_bytes());
            revision.update(name.as_bytes());
            revision.update(file.bytes.to_le_bytes());
            revision.update(file.modified.as_nanos().to_le_bytes());
            available_bytes = available_bytes.checked_add(file.bytes).ok_or(DIAG_IO)?;
            newest_at = Some(newest_at.unwrap_or(0u64).max(file.modified.as_secs()));
        }
        let snapshot = format!("{:x}", revision.finalize());
        if history.snapshot.as_ref().is_some_and(|previous| previous != &snapshot) {
            return Err(SNAPSHOT_STALE);
        }
        let (start, stop) = match history.position {
            Position::Start => (0, available_bytes.min(size)),
            Position::End => (available_bytes.saturating_sub(size), available_bytes),
            Position::All => (0, available_bytes),
        };

        let mut offset = 0_usize;
        let mut total = 0_usize;
        let mut lines = Vec::new();
        let mut origins = Vec::new();
        // First pass computes the count at a stable point under the sink lock.
        // The second pass emits only the requested interval in chronological
        // order, reversing that bounded result for a newest-first request.
        visit_window(&files, start, stop, |line, _| {
            if query.matches(line)? {
                total = total.checked_add(1).ok_or(DIAG_IO)?;
            }
            Ok(())
        })?;
        let (first, last) = if history.oldest_first {
            (history.offset.min(total), end.min(total))
        } else {
            (total.saturating_sub(end), total.saturating_sub(history.offset))
        };
        if first < last {
            visit_window(&files, start, stop, |line, archived_at| {
                if query.matches(line)? {
                    if (first..last).contains(&offset) {
                        lines.push(line.to_owned());
                        origins.push(archived_at);
                    }
                    offset += 1;
                }
                Ok(())
            })?;
        }
        if !history.oldest_first {
            lines.reverse();
            origins.reverse();
        }
        Ok(serde_json::json!({
            "level":state.0,"lines":lines,"max_bytes":state.1.max_bytes,
            "diagnostics":state.1,"archives":archives,"log_levels":crate::config::LOG_LEVELS,
            "history_version":1,"rotation_version":1,"offset":history.offset.min(total),"limit":limit,
            "line_archive_at":origins,"rotation":{"active_bytes":active_bytes,"archived_bytes":archived_bytes,"last_rotation_at":last_rotation_at},
            "total":total,"snapshot":snapshot,"window_bytes":stop-start,"available_bytes":available_bytes,"newest_at":newest_at,
        }))
    }
}

/// Traverse complete UTF-8 lines contained in a byte window over the ordered
/// files. Delimiters count toward the window. Lines crossing either boundary
/// are skipped. Reading or seeking failures refuse instead of returning a
/// partial page. Each pass opens a single file at a time under the sink lock.
fn visit_window(
    files: &[&RetainedLog],
    start: u64,
    stop: u64,
    mut visit: impl FnMut(&str, Option<u64>) -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    let mut at = 0_u64;
    for file in files {
        let next_file = at.checked_add(file.bytes).ok_or(DIAG_IO)?;
        if at >= stop {
            break;
        }
        if next_file <= start {
            at = next_file;
            continue;
        }
        let opened = std::fs::File::open(&file.path).map_err(|_| DIAG_IO)?;
        let metadata = opened.metadata().map_err(|_| DIAG_IO)?;
        if metadata.len()!=file.bytes || metadata.modified().map_err(|_| DIAG_IO)?
            .duration_since(UNIX_EPOCH).map_err(|_| DIAG_IO)?!=file.modified {
            return Err(SNAPSHOT_STALE);
        }
        let mut reader = BufReader::new(opened);
        let mut buffer = String::new();
        while at < stop {
            buffer.clear();
            let count = reader.read_line(&mut buffer).map_err(|_| DIAG_IO)? as u64;
            if count == 0 {
                break;
            }
            let next = at.checked_add(count).ok_or(DIAG_IO)?;
            if at >= start && next <= stop {
                let line = buffer.strip_suffix('\n').unwrap_or(&buffer);
                visit(line.strip_suffix('\r').unwrap_or(line), file.rotation_at)?;
            }
            at = next;
        }
        if at < stop && at != next_file {
            return Err(SNAPSHOT_STALE);
        }
    }
    Ok(())
}

/// Severity rank: higher is more verbose; `critical` is the least verbose level.
fn rank(level: &str) -> u8 {
    match level {
        "critical" => 0,
        "error" => 1,
        "warn" => 2,
        "info" => 3,
        "debug" => 4,
        _ => 3,
    }
}

/// Map the configured level string to its static form (`None` when invalid;
/// the config validation already refuses other values).
pub fn level_static(level: &str) -> Option<&'static str> {
    Some(match level {
        "critical" => "critical",
        "error" => "error",
        "warn" => "warn",
        "info" => "info",
        "debug" => "debug",
        _ => return None,
    })
}


/// Internal engine-to-authority diagnostic header, stripped at the relay.
pub const ENGINE_CODE_HEADER: &str = "x-swarm-diagnostic-code";

/// Fixed security and review codes allowed into the application log. Other
/// engine codes remain in their normal HTTP reply. Never log freeform headers.
pub fn engine_code(code: &str) -> Option<&'static str> {
    Some(match code {
        "blp-access-denied" => "blp-access-denied",
        "blp-dialogue-access-denied" => "blp-dialogue-access-denied",
        "blp-dialogue-authentication-required" => "blp-dialogue-authentication-required",
        "blp-dialogue-policy-required" => "blp-dialogue-policy-required",
        "blp-classification-required" => "blp-classification-required",
        "blp-admission-policy-changed" => "blp-admission-policy-changed",
        "biba-landlock-unavailable" => "biba-landlock-unavailable",
        "biba-workspace-instance-changed" => "biba-workspace-instance-changed",
        "biba-internet-deny" => "biba-internet-deny",
        "task-validation-controls-required" => "task-validation-controls-required",
        "task-validation-controls-failed" => "task-validation-controls-failed",
        "task-validation-gate-identity-or-permission" => "task-validation-gate-identity-or-permission",
        "task-validation-i0-internet-deny" => "task-validation-i0-internet-deny",
        "task-validation-managed-inputs-changed" => "task-validation-managed-inputs-changed",
        "native-scoped-submitted-review-required" => "native-scoped-submitted-review-required",
        "review-delivery-files-exceeded" => "review-delivery-files-exceeded",
        _ => return None,
    })
}

#[cfg(test)]
mod rotation_tests {
    use super::*;

    /// Reuse legacy active bytes, rotate whole lines, filter across archives and
    /// keep only the configured number of archive files.
    #[test]
    fn rotation_keeps_bounded_history_and_searches_archives() {
        let area = std::env::var_os("SWARM_PROTECTED_FIXTURE_AREA")
            .expect("use an admitted fixture area for diagnostics rotation");
        let temp = tempfile::tempdir_in(area).unwrap();
        let policy = DiagnosticsConfig { max_bytes: 80, archives: 2, page_rows: 3 };
        let sink = Diag::open(temp.path(), "debug", policy);
        let legacy = r#"{"level":"info","message":"legacy evidence"}"#;
        std::fs::write(temp.path().join(LOG_FILE), format!("{legacy}\n")).unwrap();
        let event = r#"{"level":"warn","message":"first diagnostic"}"#;
        sink.append(event, policy).unwrap();
        let found = sink.page("legacy", Some("info")).unwrap();
        assert_eq!(found["lines"], serde_json::json!([legacy]));
        assert_eq!(found["archives"], 1);
        let oversized = "x".repeat(policy.max_bytes as usize);
        assert!(sink.append(&oversized, policy).is_err());
        assert_eq!(sink.page("legacy", None).unwrap()["lines"], serde_json::json!([legacy]));
        let second = r#"{"level":"critical","message":"second diagnostic"}"#;
        let third = r#"{"level":"warn","message":"third diagnostic"}"#;
        sink.append(second, policy).unwrap();
        sink.append(third, policy).unwrap();
        let page = sink.page("", None).unwrap();
        assert_eq!(page["archives"], 2);
        assert_eq!(page["lines"], serde_json::json!([event, second, third]));
        assert_eq!(sink.page("diagnostic", Some("warn")).unwrap()["lines"], serde_json::json!([event, third]));
    }
    /// Exercise the retained-window choices, threshold, page ordering and a
    /// rotation invalidating the read witness. The fixture owns every file.
    #[test]
    fn history_reads_windows_and_rejects_stale_snapshots() {
        let area = std::env::var_os("SWARM_PROTECTED_FIXTURE_AREA")
            .expect("use an admitted fixture area for diagnostics history");
        let temp = tempfile::tempdir_in(area).unwrap();
        let first = r#"{"level":"info","message":"old probe"}"#;
        let second = r#"{"level":"warn","message":"mid probe"}"#;
        let third = r#"{"level":"error","message":"new probe"}"#;
        let policy = DiagnosticsConfig {
            max_bytes: 48, archives: 3, page_rows: 2,
        };
        let sink = Diag::open(temp.path(), "debug", policy);
        for line in [first, second, third] {
            sink.append(line, policy).unwrap();
        }
        let parse = |pairs: &[(&str, &str)]| {
            LogQuery::parse(&pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<Vec<_>>())
        };
        assert_eq!(sink.query(&parse(&[]).unwrap()).unwrap()["lines"], serde_json::json!([second, third]));
        let latest = sink.query(&parse(&[("position","all"),("limit","1")]).unwrap()).unwrap();
        assert_eq!(latest["total"], 3);
        assert_eq!(latest["lines"], serde_json::json!([third]));
        let archive_only = sink.query(&parse(&[("position","all"),("source","archives")]).unwrap()).unwrap();
        assert_eq!(archive_only["total"], 2);
        assert_eq!(archive_only["lines"], serde_json::json!([second, first]));
        assert!(archive_only["line_archive_at"][0].is_u64());
        assert_eq!(archive_only["rotation"]["archived_bytes"], first.len()+second.len()+2);
        assert_eq!(archive_only["rotation"]["active_bytes"], third.len()+1);
        let active_only = sink.query(&parse(&[("position","all"),("source","active")]).unwrap()).unwrap();
        assert_eq!(active_only["lines"], serde_json::json!([third]));
        assert!(active_only["line_archive_at"][0].is_null());
        assert!(active_only["rotation"]["last_rotation_at"].is_u64());
        assert!(parse(&[("source","arbitrary")]).is_err());
        let witness = latest["snapshot"].as_str().unwrap();
        let next = sink.query(&parse(&[("position","all"),("limit","1"),("offset","1"),("snapshot",witness)]).unwrap()).unwrap();
        assert_eq!(next["lines"], serde_json::json!([second]));
        let start = sink.query(&parse(&[("position","start"),("window_bytes","48"),("order","oldest")]).unwrap()).unwrap();
        assert_eq!(start["lines"], serde_json::json!([first]));
        let end = sink.query(&parse(&[("position","end"),("window_bytes","48")]).unwrap()).unwrap();
        assert_eq!(end["lines"], serde_json::json!([third]));
        let threshold = sink.query(&parse(&[("position","all"),("q","probe"),("minimum_level","warn")]).unwrap()).unwrap();
        assert_eq!(threshold["lines"], serde_json::json!([third, second]));
        assert_eq!(sink.query(&parse(&[("limit","3")]).unwrap()), Err(FILTER_INVALID));
        assert_eq!(sink.query(&parse(&[("position","start"),("window_bytes","49")]).unwrap()), Err(FILTER_INVALID));
        assert!(parse(&[("level","warn"),("minimum_level","info")]).is_err());
        assert!(parse(&[("offset","+1")]).is_err());
        assert!(parse(&[("position","all"),("window_bytes","1")]).is_err());
        assert!(parse(&[("q","a"),("q","b")]).is_err());
        sink.append(first, policy).unwrap();
        assert_eq!(sink.query(&parse(&[("position","all"),("snapshot",witness)]).unwrap()), Err(SNAPSHOT_STALE));
        assert_eq!(sink.query(&parse(&[("position","all"),("limit","1")]).unwrap()).unwrap()["lines"], serde_json::json!([first]));
        let larger = DiagnosticsConfig { max_bytes: 200, ..policy };
        sink.configure("debug", larger).unwrap();
        let archives = sink.query(&parse(&[("position","all"),("source","archives")]).unwrap()).unwrap();
        let archived_snapshot = archives["snapshot"].as_str().unwrap();
        sink.append(second, larger).unwrap();
        let archives_after_append = sink.query(&parse(&[("position","all"),("source","archives"),("snapshot",archived_snapshot)]).unwrap()).unwrap();
        assert_eq!(archives_after_append["lines"], archives["lines"]);

    }

}
