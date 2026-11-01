//! Access audit of the authority daemon (phase B, slice B8 attribution).
//!
//! Records who did what through the cockpit: sign-in (accepted/refused),
//! sign-out, and every request that is not a plain read (relayed engine
//! writes, approvals, administration, daemon administration). A mutating
//! request writes an `intent` line **before** it is executed — if that write
//! fails the request is refused (fail closed: no unattributed effect) — and
//! an `outcome` line with the HTTP status afterwards (best effort: the effect
//! already happened; a failure is reported on stderr).
//!
//! Storage: `audit.jsonl` in the daemon configuration directory (0600,
//! appended, `fdatasync` per line). Each line carries `seq` and `prev`, the
//! SHA-256 of the previous line; [`Audit::read`] reports whether the chain is
//! intact. This makes edits detectable, not impossible: a process running as
//! the daemon account (or root) can rewrite the whole file — protection
//! against that, and against restoring an older copy, needs an OS-protected
//! witness (EP-06, not provided). Never records passwords, tokens or bodies;
//! paths are recorded without query string. Growth is unbounded (one line
//! per human action); rotation is an operator task.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

/// One audit line (also the read view).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub seq: u64,
    pub at: i64,
    /// `sign-in`, `sign-in-refused`, `sign-out`, `intent`, `outcome`.
    pub event: String,
    /// Account subject; `None` when unknown or not a valid account name.
    pub subject: Option<String>,
    /// `session` or `key:<id>` for intent/outcome (B9).
    pub credential: Option<String>,
    /// `assertion:<nonce>` of a relayed request: the same reference is in
    /// the engine's attribution ledger. Absent in older lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    /// Action family (`write`, `approve`, `administer`) for intent/outcome.
    pub action: Option<String>,
    /// HTTP status of the outcome.
    pub status: Option<u16>,
    /// SHA-256 (hex) of the previous line; 64 zeros for the first line.
    pub prev: String,
}

/// What a caller records; `seq`, `prev` are assigned by [`Audit::record`].
#[derive(Clone, Debug, Default)]
pub struct Event<'a> {
    pub at: i64,
    pub event: &'a str,
    pub subject: Option<&'a str>,
    pub credential: Option<&'a str>,
    pub reference: Option<&'a str>,
    pub method: Option<&'a str>,
    pub path: Option<&'a str>,
    pub action: Option<&'a str>,
    pub status: Option<u16>,
}

struct Tail {
    seq: u64,
    prev: String,
}

/// Append-only audit file with an in-memory chain tail.
pub struct Audit {
    path: PathBuf,
    tail: Mutex<Tail>,
}

const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn digest(line: &[u8]) -> String {
    format!("{:x}", Sha256::digest(line))
}

/// Scan `path`: entries, chain validity, tail. Undecodable lines break the
/// chain but do not stop the scan.
fn scan(path: &Path) -> Result<(Vec<Entry>, bool, Tail), &'static str> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((Vec::new(), true, Tail { seq: 0, prev: GENESIS.into() })),
        Err(_) => return Err("authority-audit-io"),
    };
    let (mut entries, mut valid, mut tail) = (Vec::new(), true, Tail { seq: 0, prev: GENESIS.into() });
    for line in std::io::BufReader::new(file).split(b'\n') {
        let line = line.map_err(|_| "authority-audit-io")?;
        match serde_json::from_slice::<Entry>(&line) {
            Ok(e) => {
                valid &= e.prev == tail.prev && e.seq == tail.seq + 1;
                tail.seq = e.seq;
                entries.push(e);
            }
            Err(_) => valid = false,
        }
        tail.prev = digest(&line);
    }
    Ok((entries, valid, tail))
}

impl Audit {
    /// Open `dir/audit.jsonl` (created on first record). A broken chain does
    /// not prevent opening (the daemon keeps serving and reports it through
    /// [`Audit::read`]); new lines chain from the last line present.
    pub fn open(dir: &Path) -> Result<Self, &'static str> {
        Self::open_file(&dir.join("audit.jsonl"))
    }

    /// Open an audit file at an explicit path (the engine's attribution
    /// ledger reuses this format in its Root).
    pub fn open_file(path: &Path) -> Result<Self, &'static str> {
        let (_, _, tail) = scan(path)?;
        Ok(Self { path: path.to_path_buf(), tail: Mutex::new(tail) })
    }

    /// Current chain tail: last `seq` and SHA-256 of the last line (64 zeros
    /// and 0 when empty). Used as the audit part of the rollback witness.
    pub fn tail(&self) -> Result<(u64, String), &'static str> {
        let t = self.tail.lock().map_err(|_| "authority-audit-state")?;
        Ok((t.seq, t.prev.clone()))
    }

    /// Append one event; returns its `seq`. Errors: `authority-audit-io`
    /// (open/write/sync failed; the tail is unchanged so the chain stays
    /// consistent with the file when the write did not land),
    /// `authority-audit-state`.
    pub fn record(&self, ev: &Event<'_>) -> Result<u64, &'static str> {
        use std::os::unix::fs::OpenOptionsExt as _;
        let mut tail = self.tail.lock().map_err(|_| "authority-audit-state")?;
        let entry = Entry {
            seq: tail.seq + 1,
            at: ev.at,
            event: ev.event.into(),
            subject: ev.subject.map(str::to_owned),
            credential: ev.credential.map(str::to_owned),
            reference: ev.reference.map(str::to_owned),
            method: ev.method.map(str::to_owned),
            path: ev.path.map(str::to_owned),
            action: ev.action.map(str::to_owned),
            status: ev.status,
            prev: tail.prev.clone(),
        };
        let line = serde_json::to_vec(&entry).map_err(|_| "authority-audit-state")?;
        let mut f = std::fs::OpenOptions::new().append(true).create(true).mode(0o600).open(&self.path).map_err(|_| "authority-audit-io")?;
        let mut framed = line.clone();
        framed.push(b'\n');
        f.write_all(&framed).and_then(|_| f.sync_data()).map_err(|_| "authority-audit-io")?;
        tail.seq = entry.seq;
        tail.prev = digest(&line);
        Ok(entry.seq)
    }

    /// All entries in order and whether the hash chain is intact.
    pub fn read(&self) -> Result<(Vec<Entry>, bool), &'static str> {
        let _guard = self.tail.lock().map_err(|_| "authority-audit-state")?;
        let (entries, valid, _) = scan(&self.path)?;
        Ok((entries, valid))
    }
}
