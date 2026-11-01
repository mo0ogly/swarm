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
//! per human action, plus page consultations and policy refusals, written
//! best effort); an administrator purges lines older than a chosen number of
//! days ([`Audit::purge`]), the chain staying verifiable from a
//! `purge-anchor` line. [`sessions`] and [`dashboard`] aggregate the lines
//! for the Admin (connections per session, activity per day and account).
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

/// Event of the first line written by [`Audit::purge`].
pub const PURGE_ANCHOR: &str = "purge-anchor";

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
            // A purge anchor (first line only) restarts the chain: the next
            // line links to the digest of the last purged line it records.
            Ok(e) if e.event == PURGE_ANCHOR && entries.is_empty() && tail.seq == 0 => {
                tail.seq = e.seq;
                tail.prev = e.reference.clone().unwrap_or_default();
                entries.push(e);
                continue;
            }
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

    /// Remove every line older than `before` (Unix seconds), keeping the
    /// chain verifiable: the file is rewritten (temporary file, fsync,
    /// rename) as a `purge-anchor` line — recording the actor, the number of
    /// removed lines, the last removed `seq` and the digest of the last
    /// removed line in `reference` — followed by the kept lines unchanged.
    /// The tail (and so the EP-06 witness) does not change. Refuses on a
    /// broken chain (`authority-audit-chain-broken`: purging would hide the
    /// break). Returns the number of removed lines (0: file untouched).
    pub fn purge(&self, before: i64, actor: &str, now: i64) -> Result<usize, &'static str> {
        use std::os::unix::fs::OpenOptionsExt as _;
        let tail = self.tail.lock().map_err(|_| "authority-audit-state")?;
        let raw = match std::fs::read(&self.path) {
            Ok(r) => r,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(_) => return Err("authority-audit-io"),
        };
        let (_, valid, _) = scan(&self.path)?;
        if !valid {
            return Err("authority-audit-chain-broken");
        }
        let lines: Vec<&[u8]> = raw.split(|b| *b == b'\n').filter(|l| !l.is_empty()).collect();
        let mut removed = 0usize;
        let mut last: Option<(u64, String)> = None;
        let mut previous_anchor = 0usize;
        for line in &lines {
            let e: Entry = serde_json::from_slice(line).map_err(|_| "authority-audit-chain-broken")?;
            // A previous anchor is always replaced (its own date is the
            // purge time, not the age of the lines it introduces).
            if e.event == PURGE_ANCHOR {
                previous_anchor += 1;
                continue;
            }
            if e.at >= before {
                break;
            }
            removed += 1;
            last = Some((e.seq, digest(line)));
        }
        let Some((last_seq, last_digest)) = last else { return Ok(0) };
        if removed == 0 {
            return Ok(0);
        }
        let anchor = Entry {
            seq: last_seq,
            at: now,
            event: PURGE_ANCHOR.into(),
            subject: Some(actor.into()),
            credential: None,
            reference: Some(last_digest),
            method: None,
            path: None,
            action: Some(format!("removed:{removed}")),
            status: None,
            prev: String::new(),
        };
        let mut out = serde_json::to_vec(&anchor).map_err(|_| "authority-audit-state")?;
        out.push(b'\n');
        for line in lines.iter().skip(removed + previous_anchor) {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
        let tmp = self.path.with_extension("jsonl.purge.tmp");
        let result = (|| {
            use std::io::Write as _;
            let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
            f.write_all(&out)?;
            f.sync_all()?;
            std::fs::rename(&tmp, &self.path)?;
            if let Some(dir) = self.path.parent() {
                std::fs::File::open(dir)?.sync_all()?;
            }
            Ok::<_, std::io::Error>(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
            return Err("authority-audit-io");
        }
        drop(tail);
        Ok(removed)
    }

    /// All entries in order and whether the hash chain is intact.
    pub fn read(&self) -> Result<(Vec<Entry>, bool), &'static str> {
        let _guard = self.tail.lock().map_err(|_| "authority-audit-state")?;
        let (entries, valid, _) = scan(&self.path)?;
        Ok((entries, valid))
    }
}

/// One sign-in session (or API key) as seen in the audit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SessionSummary {
    /// `session:<id>` or `key:<id>`.
    pub credential: String,
    pub subject: Option<String>,
    pub first_at: i64,
    pub last_at: i64,
    pub consultations: usize,
    /// Mutating requests (intent lines).
    pub actions: usize,
    pub refusals: usize,
    pub signed_out: bool,
}

/// Group `entries` with `at` in `[from, to)` (and `subject` when given) by
/// credential, most recent first. Lines without credential (refused
/// sign-ins, anchors) are not sessions.
pub fn sessions(entries: &[Entry], from: i64, to: i64, subject: Option<&str>) -> Vec<SessionSummary> {
    let mut by: std::collections::BTreeMap<String, SessionSummary> = Default::default();
    for e in entries.iter().filter(|e| e.at >= from && e.at < to && subject.is_none_or(|s| e.subject.as_deref() == Some(s))) {
        let Some(c) = &e.credential else { continue };
        if !(c.starts_with("session:") || c.starts_with("key:")) {
            continue;
        }
        let s = by.entry(c.clone()).or_insert_with(|| SessionSummary {
            credential: c.clone(), subject: e.subject.clone(), first_at: e.at, last_at: e.at,
            consultations: 0, actions: 0, refusals: 0, signed_out: false,
        });
        s.first_at = s.first_at.min(e.at);
        s.last_at = s.last_at.max(e.at);
        match e.event.as_str() {
            "consultation" => s.consultations += 1,
            "intent" => s.actions += 1,
            "refused" => s.refusals += 1,
            "sign-out" => s.signed_out = true,
            _ => {}
        }
    }
    let mut out: Vec<SessionSummary> = by.into_values().collect();
    out.sort_by_key(|s| std::cmp::Reverse(s.first_at));
    out
}

/// Activity of one UTC day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Day {
    /// `YYYY-MM-DD` (UTC).
    pub day: String,
    pub sign_ins: usize,
    pub refused_sign_ins: usize,
    pub accounts: usize,
    pub consultations: usize,
    pub actions: usize,
    pub refusals: usize,
}

/// Activity of one account over the period.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AccountActivity {
    pub subject: String,
    pub sessions: usize,
    pub consultations: usize,
    pub actions: usize,
    pub refusals: usize,
    pub last_at: i64,
}

/// `YYYY-MM-DD` of a Unix time (UTC, proleptic Gregorian).
pub fn utc_day(at: i64) -> String {
    let days = at.div_euclid(86_400);
    // Civil-from-days (H. Hinnant), valid for the whole i64 day range used here.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Per-day and per-account activity for `[from, to)`, days ascending,
/// accounts by number of actions then consultations, descending.
pub fn dashboard(entries: &[Entry], from: i64, to: i64) -> (Vec<Day>, Vec<AccountActivity>) {
    use std::collections::{BTreeMap, BTreeSet};
    let mut days: BTreeMap<String, (Day, BTreeSet<String>)> = BTreeMap::new();
    let mut accounts: BTreeMap<String, (AccountActivity, BTreeSet<String>)> = BTreeMap::new();
    for e in entries.iter().filter(|e| e.at >= from && e.at < to && e.event != PURGE_ANCHOR) {
        let key = utc_day(e.at);
        let (d, who) = days.entry(key.clone()).or_insert_with(|| {
            (Day { day: key, sign_ins: 0, refused_sign_ins: 0, accounts: 0, consultations: 0, actions: 0, refusals: 0 }, BTreeSet::new())
        });
        match e.event.as_str() {
            "sign-in" => d.sign_ins += 1,
            "sign-in-refused" => d.refused_sign_ins += 1,
            "consultation" => d.consultations += 1,
            "intent" => d.actions += 1,
            "refused" => d.refusals += 1,
            _ => {}
        }
        let Some(subject) = &e.subject else { continue };
        if e.event == "sign-in-refused" {
            continue;
        }
        who.insert(subject.clone());
        let (a, creds) = accounts.entry(subject.clone()).or_insert_with(|| {
            (AccountActivity { subject: subject.clone(), sessions: 0, consultations: 0, actions: 0, refusals: 0, last_at: e.at }, BTreeSet::new())
        });
        a.last_at = a.last_at.max(e.at);
        if let Some(c) = &e.credential {
            creds.insert(c.clone());
        }
        match e.event.as_str() {
            "consultation" => a.consultations += 1,
            "intent" => a.actions += 1,
            "refused" => a.refusals += 1,
            _ => {}
        }
    }
    let days = days.into_values().map(|(mut d, who)| { d.accounts = who.len(); d }).collect();
    let mut accounts: Vec<AccountActivity> = accounts.into_values().map(|(mut a, c)| { a.sessions = c.len(); a }).collect();
    accounts.sort_by_key(|a| std::cmp::Reverse((a.actions, a.consultations)));
    (days, accounts)
}
