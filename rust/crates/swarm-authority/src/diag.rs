//! Bounded diagnostics file of the daemon (ADM-09).
//!
//! One JSON-lines file (`authority.log` in the configuration directory):
//! every line is `{"at", "level", "message", …fields}`. The operator picks
//! the minimum level in the Admin (`log_level` in [`crate::config`], applied
//! live); the file is size-bounded by truncation-from-the-front — when it
//! passes `MAX_BYTES`, the oldest half is dropped, so its cost stays known
//! and the newest lines survive. Writes are best effort: a diagnostics
//! failure never changes a request's outcome (a warning on stderr instead).
//!
//! Reading back for the Admin (`read_tail`) returns the last `max` lines —
//! no shell, no external log tool.
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

/// Soft size cap of the diagnostics file (8 MiB): on overflow the oldest
/// half is dropped, keeping the file's disk cost bounded and predictable.
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// One diagnostics sink. `file` is recreated lazily on each write so the
/// Admin-visible level applies without reopening anything.
pub struct Diag {
    path: PathBuf,
    level: Mutex<&'static str>,
}

impl Diag {
    /// Sink writing `dir/authority.log`, starting at `level`.
    pub fn open(dir: &std::path::Path, level: &'static str) -> Self {
        Self { path: dir.join("authority.log"), level: Mutex::new(level) }
    }

    /// Current minimum level (`debug`…`error`).
    pub fn level(&self) -> &'static str {
        self.level.lock().map(|l| *l).unwrap_or("info")
    }

    /// Apply a new minimum level (validated by the caller, `log_level`).
    pub fn set_level(&self, level: &'static str) {
        if let Ok(mut l) = self.level.lock() {
            *l = level;
        }
    }

    /// Append one line at `level` when the level passes. `fields` are extra
    /// JSON values merged into the object. Best effort: errors are reported
    /// on stderr, never propagated.
    pub fn log(&self, level: &str, message: &str, fields: serde_json::Value) {
        if rank(level) > rank(self.level()) {
            return;
        }
        let mut obj = serde_json::json!({
            "at": crate::server::now(),
            "level": level,
            "message": message,
        });
        if let (Some(dst), Some(src)) = (obj.as_object_mut(), fields.as_object()) {
            for (k, v) in src {
                dst.insert(k.clone(), v.clone());
            }
        }
        let line = obj.to_string();
        if let Err(e) = self.append(&line) {
            eprintln!("{}", serde_json::json!({"schema_version": 1, "warning": "authority-diag-unavailable", "error": e.to_string()}));
        }
    }

    /// Append with size-bounded truncation: over [`MAX_BYTES`], the oldest
    /// half of the file is dropped before writing.
    fn append(&self, line: &str) -> std::io::Result<()> {
        if let Ok(meta) = std::fs::metadata(&self.path) {
            if meta.len() + line.len() as u64 > MAX_BYTES {
                let kept = std::fs::read(&self.path)?;
                // Split at a line boundary past the middle: the newest half.
                let mid = kept.len() / 2;
                let cut = kept[mid..].iter().position(|b| *b == b'\n').map(|p| mid + p + 1).unwrap_or(mid);
                let tmp = self.path.with_extension("log.tmp");
                {
                    let mut f = std::fs::File::create(&tmp)?;
                    f.write_all(&kept[cut..])?;
                    f.sync_all()?;
                }
                std::fs::rename(&tmp, &self.path)?;
            }
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&self.path)?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
        f.flush()
    }

    /// Last `max` lines for the Admin view (oldest first). A missing file
    /// means an empty log (nothing logged yet); read errors surface as
    /// `authority-diag-io`.
    pub fn read_tail(&self, max: usize) -> Result<Vec<String>, &'static str> {
        let file = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err("authority-diag-io"),
        };
        let lines: Vec<String> = BufReader::new(file).lines().map_while(Result::ok).collect();
        Ok(lines.into_iter().rev().take(max).collect::<Vec<_>>().into_iter().rev().collect())
    }
}

/// Severity rank: higher is more verbose; below `error` nothing is logged.
fn rank(level: &str) -> u8 {
    match level {
        "error" => 0,
        "warn" => 1,
        "info" => 2,
        "debug" => 3,
        _ => 2,
    }
}

/// Map the configured level string to its static form (`None` when invalid;
/// the config validation already refuses other values).
pub fn level_static(level: &str) -> Option<&'static str> {
    Some(match level {
        "error" => "error",
        "warn" => "warn",
        "info" => "info",
        "debug" => "debug",
        _ => return None,
    })
}
