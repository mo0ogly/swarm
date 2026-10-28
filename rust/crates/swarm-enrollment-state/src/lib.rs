//! Native signed enrollment journal. Records are opaque data, not business rights.
//! Pins and the independent Head must come from trusted bootstrap, never this file.
//! SQLite serialization is local; signed replies can be stale before final use.
#![forbid(unsafe_code)]
mod bound;
mod reply;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
pub use bound::{BatchPreview, BoundJournal};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier as _, VerifyingKey};
pub use reply::{decode_active_reply, ActiveReply, ACTIVE_REPLY_SCHEMA};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};
use swarm_enrollment_proof::{sign_response, Request};
use swarm_identity_contract::digest;
use swarm_wire_json as wire;
const PROTOCOL: &str = "swarm-enrollment-change-v1";
#[derive(Debug)]
pub enum Error {
    Context,
    Head,
    Change,
    History(Box<Error>),
    Exists,
    Missing,
    Revoked,
    Reply,
    ReplyVersion,
    Canceled,
    Closed,
    Busy,
    State,
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Proof(swarm_enrollment_proof::Error),
    Close {
        cause: Box<Error>,
        close: rusqlite::Error,
    },
    Rollback {
        cause: Box<Error>,
        rollback: rusqlite::Error,
    },
}
impl Error {
    /// Stable diagnostic category; typed storage/cleanup causes remain available.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Head => "head",
            Self::Change | Self::History(_) => "change",
            Self::Exists => "exists",
            Self::Missing => "missing",
            Self::Revoked => "revoked",
            Self::Reply => "reply",
            Self::ReplyVersion => "reply-version",
            Self::Canceled => "canceled",
            Self::Closed => "closed",
            Self::Busy => "busy",
            Self::State => "state",
            Self::Sqlite(_) => "sqlite",
            Self::Io(_) => "io",
            Self::Proof(e) => e.code(),
            Self::Rollback { .. } => "rollback",
            Self::Close { .. } => "close",
        }
    }
    /// True only for immediate ownership/SQLite contention, never a cleanup failure.
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Busy)
            || matches!(self,Self::Sqlite(rusqlite::Error::SqliteFailure(e,_)) if matches!(e.code,rusqlite::ErrorCode::DatabaseBusy|rusqlite::ErrorCode::DatabaseLocked))
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "enrollment state: SQLite: {e}"),
            Self::Io(e) => write!(f, "enrollment state: I/O: {e}"),
            Self::Proof(e) => write!(f, "{e}"),
            Self::History(cause) => write!(f, "enrollment state: invalid history: {cause}"),
            Self::Rollback { cause, rollback } => write!(f, "{cause}; rollback failed: {rollback}"),
            Self::Close { cause, close } => write!(f, "{cause}; close failed: {close}"),
            e => write!(f, "enrollment state: {}", e.code()),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite(e) => Some(e),
            Self::Io(e) => Some(e),
            Self::Proof(e) => Some(e),
            Self::Rollback { cause, .. } | Self::Close { cause, .. } | Self::History(cause) => {
                Some(cause.as_ref())
            }
            _ => None,
        }
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}
/// Explicit cancellation, with no deadline, polling thread or retry policy.
#[derive(Clone, Default)]
pub struct Context(Arc<AtomicBool>);
impl Context {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst)
    }
    fn check(&self) -> Result<(), Error> {
        if self.0.load(Ordering::SeqCst) {
            Err(Error::Canceled)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    pub sequence: i64,
    pub digest: String,
}
impl Head {
    pub fn valid(&self) -> bool {
        self.sequence == 0 && self.digest.is_empty()
            || self.sequence > 0 && digest(self.digest.as_bytes())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Record {
    pub id: String,
    #[serde(serialize_with = "reply::serialize_body")]
    pub body: Vec<u8>,
    pub enrolled_sequence: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    protocol: String,
    authority_key_id: String,
    project_digest: String,
    sequence: i64,
    previous: String,
    operation: String,
    id: String,
    body: String,
    signature: String,
}
struct Loaded {
    head: Head,
    records: HashMap<String, Entry>,
}
struct Entry {
    record: Record,
    revoked: bool,
}
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
/// Preserve exact SQLite TEXT/BLOB event bytes as Go Scan([]byte) does. Storage
/// class supplies no authority: signatures/canonical history are checked by load.
/// Numeric/null/special representations refuse instead of becoming empty events.
fn event_bytes(value: rusqlite::types::ValueRef<'_>) -> Result<Vec<u8>, Error> {
    match value {
        rusqlite::types::ValueRef::Text(raw) | rusqlite::types::ValueRef::Blob(raw) => Ok(raw.to_vec()),
        _ => Err(Error::Change),
    }
}
fn valid_id(id: &str) -> bool {
    !id.trim().is_empty() && !id.chars().any(char::is_control)
}
fn decode_body(s: &str) -> Result<Vec<u8>, Error> {
    let raw = URL_SAFE_NO_PAD.decode(s).map_err(|_| Error::Change)?;
    if URL_SAFE_NO_PAD.encode(&raw) != s {
        return Err(Error::Change);
    }
    Ok(raw)
}
impl Change {
    fn valid(&self) -> bool {
        self.protocol == PROTOCOL
            && digest(self.authority_key_id.as_bytes())
            && digest(self.project_digest.as_bytes())
            && self.sequence > 0
            && Head {
                sequence: self.sequence - 1,
                digest: self.previous.clone(),
            }
            .valid()
            && valid_id(&self.id)
            && decode_body(&self.body).is_ok()
            && (self.operation == "enroll" || self.operation == "revoke" && self.body.is_empty())
    }
    fn signed_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut output = format!("{PROTOCOL}\0change\0").into_bytes();
        let mut copy = self.clone();
        copy.signature.clear();
        output.extend(wire::encode(&copy).map_err(|_| Error::Change)?);
        Ok(output)
    }
}
/// Sign a proposal only; no storage acceptance or private-key provisioning.
pub fn sign_change(
    key: &SigningKey,
    project: [u8; 32],
    base: &Head,
    operation: &str,
    id: &str,
    body: &[u8],
) -> Result<Vec<u8>, Error> {
    if !base.valid() || base.sequence == i64::MAX {
        return Err(Error::Change);
    }
    let mut c = Change {
        protocol: PROTOCOL.into(),
        authority_key_id: hash(&key.verifying_key().to_bytes()),
        project_digest: hex(&project),
        sequence: base.sequence + 1,
        previous: base.digest.clone(),
        operation: operation.into(),
        id: id.into(),
        body: URL_SAFE_NO_PAD.encode(body),
        signature: String::new(),
    };
    if !c.valid() {
        return Err(Error::Change);
    }
    c.signature = URL_SAFE_NO_PAD.encode(key.sign(&c.signed_bytes()?).to_bytes());
    wire::encode(&c).map_err(|_| Error::Change)
}
/// Connections are operation-local; no process mutex serializes SQLite writers.
/// close waits for active operations and rejects new ones, as a closed Go DB does.
pub struct Journal {
    path: PathBuf,
    key: [u8; 32],
    key_id: String,
    project: String,
    closed: RwLock<bool>,
}
impl Journal {
    pub fn open(
        ctx: &Context,
        path: impl AsRef<Path>,
        key: &[u8],
        project: [u8; 32],
    ) -> Result<Self, Error> {
        if key.len() != 32 || path.as_ref().as_os_str().is_empty() {
            return Err(Error::Context);
        }
        ctx.check()?;
        let path = if path.as_ref().is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            std::env::current_dir().map_err(Error::Io)?.join(path)
        };
        let j = Self {
            path,
            key: key.try_into().map_err(|_| Error::Context)?,
            key_id: hash(key),
            project: hex(&project),
            closed: RwLock::new(false),
        };
        let c = j.connect(ctx)?;
        let mode: String = c.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        if mode != "wal" {
            return Err(Error::Context);
        }
        j.transaction(ctx,&c,|c|{
   c.execute_batch("CREATE TABLE IF NOT EXISTS enrollment_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), schema_version INTEGER NOT NULL, authority_key_id TEXT NOT NULL, project_digest TEXT NOT NULL, sequence INTEGER NOT NULL CHECK(sequence>=0), digest TEXT NOT NULL); CREATE TABLE IF NOT EXISTS enrollment_events (sequence INTEGER PRIMARY KEY CHECK(sequence>0), body BLOB NOT NULL)")?;
   c.execute("INSERT INTO enrollment_meta(singleton,schema_version,authority_key_id,project_digest,sequence,digest) VALUES(1,1,?,?,0,'') ON CONFLICT(singleton) DO NOTHING",params![j.key_id,j.project])?;j.load(ctx,c)?;Ok(())
  })?;
        Ok(j)
    }
    pub fn close(&self) -> Result<(), Error> {
        *self.closed.write().map_err(|_| Error::State)? = true;
        Ok(())
    }
    fn connect(&self, ctx: &Context) -> Result<Connection, Error> {
        ctx.check()?;
        let c = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Self::configure(ctx, &c)?;
        Ok(c)
    }
    /// Install this call's cancellation hook and durable SQLite settings. There
    /// is no waiting/retry policy; callers must explicitly resume after Busy.
    fn configure(ctx: &Context, c: &Connection) -> Result<(), Error> {
        ctx.check()?;
        // Remove rusqlite's 5000 ms default. Go's journal has no busy handler/retry.
        c.busy_handler(None)?;
        let token = ctx.clone();
        c.progress_handler(1, Some(move || token.check().is_err()))?;
        c.execute_batch("PRAGMA synchronous=FULL")?;
        ctx.check()?;
        Ok(())
    }
    fn transaction<T>(
        &self,
        ctx: &Context,
        c: &Connection,
        f: impl FnOnce(&Connection) -> Result<T, Error>,
    ) -> Result<T, Error> {
        ctx.check()?;
        let result = (|| {
            c.execute_batch("BEGIN IMMEDIATE")?;
            let out = f(c)?;
            ctx.check()?;
            c.execute_batch("COMMIT")?;
            Ok(out)
        })();
        // A canceled progress hook must not prevent cleanup; no errors are swallowed.
        c.progress_handler(0, None::<fn() -> bool>)?;
        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                let cause = if ctx.check().is_err() {
                    Error::Canceled
                } else {
                    e
                };
                if !c.is_autocommit() {
                    if let Err(rollback) = c.execute_batch("ROLLBACK") {
                        return Err(Error::Rollback {
                            cause: Box::new(cause),
                            rollback,
                        });
                    }
                }
                Err(cause)
            }
        }
    }
    fn decode(&self, raw: &[u8]) -> Result<Change, Error> {
        let c: Change = wire::decode(raw).map_err(|_| Error::Change)?;
        if wire::encode(&c).map_err(|_| Error::Change)? != raw || !c.valid() {
            return Err(Error::Change);
        }
        if c.authority_key_id != self.key_id || c.project_digest != self.project {
            return Err(Error::Context);
        }
        let sig = decode_body(&c.signature)?;
        let sig = Signature::from_slice(&sig).map_err(|_| Error::Change)?;
        let key = VerifyingKey::from_bytes(&self.key).map_err(|_| Error::Change)?;
        key.verify(&c.signed_bytes()?, &sig)
            .map_err(|_| Error::Change)?;
        Ok(c)
    }
    fn load(&self, ctx: &Context, c: &Connection) -> Result<Loaded, Error> {
        let(version,key,project,sequence,digest):(i64,String,String,i64,String)=c.query_row("SELECT schema_version,authority_key_id,project_digest,sequence,digest FROM enrollment_meta WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        if version != 1 || key != self.key_id || project != self.project {
            return Err(Error::Context);
        }
        let declared = Head { sequence, digest };
        let mut query =
            c.prepare("SELECT sequence,body FROM enrollment_events ORDER BY sequence")?;
        let mut rows = query.query([])?;
        let mut head = Head::default();
        let mut records = HashMap::new();
        while let Some(row) = rows.next()? {
            ctx.check()?;
            let sequence: i64 = row.get(0)?;
            // Go Scan([]byte) accepts both SQLite TEXT and BLOB. Authenticate
            // the unchanged bytes rather than granting meaning to a storage class.
            let raw = event_bytes(row.get_ref(1)?)?;
            let change = self.decode(&raw)?;
            if head.sequence == i64::MAX
                || sequence != head.sequence + 1
                || change.sequence != sequence
                || change.previous != head.digest
            {
                return Err(Error::Change);
            }
            advance(&mut records, &change).map_err(|cause| Error::History(Box::new(cause)))?;
            head = Head {
                sequence,
                digest: hash(&raw),
            };
        }
        if head != declared {
            return Err(Error::Head);
        }
        Ok(Loaded {
            head,
            records,
        })
    }
    /// Apply one signed event with the original exact-last-event replay contract.
    /// Validates pins/chain before storage, then uses one SQLite transaction. An
    /// error after SQLite publication does not prove no change; preserve/replay
    /// the exact signed bytes. No head is learned automatically from storage.
    pub fn apply(&self, ctx: &Context, base: &Head, raw: &[u8]) -> Result<Head, Error> {
        // Preserve single-event refusal precedence: base, signed frame/chain,
        // closed state, then cancellation/storage. Batch cancellation is separate.
        if !base.valid() || base.sequence == i64::MAX { return Err(Error::Head); }
        let change = self.decode(raw)?;
        if change.sequence != base.sequence + 1 || change.previous != base.digest { return Err(Error::Head); }
        let closed = self.closed.read().map_err(|_| Error::State)?;
        if *closed { return Err(Error::Closed); }
        let c = self.connect(ctx)?;
        let next = Head {sequence:change.sequence,digest:hash(raw)};
        self.apply_prepared(ctx, &c, base, &[raw], &[change], &next)
    }
    /// Validate every signed change and its complete chain before installing a
    /// connection callback or touching storage. Empty batches refuse; no implicit
    /// count/size/attempt limit is added. Cancellation is checked between changes.
    fn prepare_batch(&self, ctx: &Context, base: &Head, raws: &[&[u8]]) -> Result<(Vec<Change>, Head), Error> {
        ctx.check()?;
        if !base.valid() || base.sequence == i64::MAX { return Err(Error::Head); }
        if raws.is_empty() { return Err(Error::Change); }
        let mut head = base.clone();
        let mut changes = Vec::new();
        for raw in raws {
            ctx.check()?;
            let change = self.decode(raw)?;
            if head.sequence == i64::MAX || change.sequence != head.sequence + 1 || change.previous != head.digest { return Err(Error::Head); }
            head = Head { sequence: change.sequence, digest: hash(raw) };
            changes.push(change);
        }
        Ok((changes, head))
    }
    /// Commit a previously validated full batch on the actual owned connection.
    /// CAS compares the independently expected base, or accepts only exact full
    /// batch replay at its terminal head. All intermediate events and the final
    /// metadata update share one transaction; a later refusal rolls back earlier
    /// writes. Does not invoke external business effects or invent authority.
    fn apply_prepared(&self, ctx: &Context, c: &Connection, base: &Head, raws: &[&[u8]], changes: &[Change], next: &Head) -> Result<Head, Error> {
        self.transaction(ctx, c, |c| {
            let Loaded { head, mut records, .. } = self.load(ctx, c)?;
            if &head == next {
                for (change, raw) in changes.iter().zip(raws) {
                    let stored = c.query_row("SELECT body FROM enrollment_events WHERE sequence=?", [change.sequence], |r| Ok(event_bytes(r.get_ref(0)?)))??;
                    if stored != *raw { return Err(Error::Change); }
                }
                return Ok(head);
            }
            if &head != base { return Err(Error::Head); }
            for (change, raw) in changes.iter().zip(raws) {
                ctx.check()?;
                advance(&mut records, change)?;
                c.execute("INSERT INTO enrollment_events(sequence,body) VALUES(?,?)", params![change.sequence, *raw])?;
            }
            c.execute("UPDATE enrollment_meta SET sequence=?,digest=? WHERE singleton=1", params![next.sequence, next.digest])?;
            Ok(next.clone())
        })
    }
    /// Keep callbacks local and reversible. No reentry, close or waiting on writers.
    /// Commit can fail after callback execution; this is not an external-action permit.
    pub fn with_active<T>(
        &self,
        ctx: &Context,
        expected: &Head,
        id: &str,
        f: impl FnOnce(Record) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if !expected.valid() {
            return Err(Error::Head);
        }
        let closed = self.closed.read().map_err(|_| Error::State)?;
        if *closed {
            return Err(Error::Closed);
        }
        let c = self.connect(ctx)?;
        self.active_on(ctx, &c, expected, id, f)
    }
    /// Check signed current state while retaining a write reservation through
    /// the callback and COMMIT. Callback effects cannot be undone by rollback.
    fn active_on<T>(
        &self,
        ctx: &Context,
        c: &Connection,
        expected: &Head,
        id: &str,
        f: impl FnOnce(Record) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.transaction(ctx, c, |c| {
            let Loaded {
                head, mut records, ..
            } = self.load(ctx, c)?;
            if &head != expected {
                return Err(Error::Head);
            }
            let current = records.remove(id).ok_or(Error::Missing)?;
            if current.revoked {
                return Err(Error::Revoked);
            }
            ctx.check()?;
            f(current.record)
        })
    }
    pub fn sign_active(
        &self,
        ctx: &Context,
        expected: &Head,
        id: &str,
        request: Request,
        key: &SigningKey,
    ) -> Result<Vec<u8>, Error> {
        if key.verifying_key().to_bytes() != self.key || request.project_digest != self.project {
            return Err(Error::Context);
        }
        self.with_active(ctx, expected, id, |record| {
            let payload = wire::encode(&ActiveReply {
                schema_version: ACTIVE_REPLY_SCHEMA,
                head: expected.clone(),
                record,
            })
            .map_err(|_| Error::Reply)?;
            sign_response(&request, &payload, key).map_err(Error::Proof)
        })
    }
}
fn advance(records: &mut HashMap<String, Entry>, c: &Change) -> Result<(), Error> {
    if c.operation == "enroll" {
        if records.contains_key(&c.id) {
            return Err(Error::Exists);
        }
        records.insert(
            c.id.clone(),
            Entry {
                record: Record {
                    id: c.id.clone(),
                    body: decode_body(&c.body)?,
                    enrolled_sequence: c.sequence,
                },
                revoked: false,
            },
        );
        return Ok(());
    }
    let current = records.get_mut(&c.id).ok_or(Error::Missing)?;
    if current.revoked {
        return Err(Error::Revoked);
    }
    current.revoked = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_connection_has_full_sync_no_busy_timeout_and_sql_cancellation_recovers() {
        let dir = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        let ctx = Context::new();
        let j = Journal::open(
            &ctx,
            dir.path().join("journal.sqlite"),
            &key.verifying_key().to_bytes(),
            [11; 32],
        )
        .unwrap();
        for _ in 0..2 {
            let c = j.connect(&ctx).unwrap();
            let sync: i64 = c.query_row("PRAGMA synchronous", [], |r| r.get(0)).unwrap();
            let busy: i64 = c
                .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
                .unwrap();
            assert_eq!(sync, 2);
            assert_eq!(busy, 0);
        }
        let c = j.connect(&ctx).unwrap();
        j.transaction(&ctx, &c, |c| {
            let n: i64 = c.query_row("SELECT 1", [], |r| r.get(0))?;
            assert_eq!(n, 1);
            Ok(())
        })
        .unwrap();
        let token = ctx.clone();
        let mut steps = 0;
        c.progress_handler(
            1,
            Some(move || {
                steps += 1;
                if steps == 20 {
                    token.cancel()
                }
                token.check().is_err()
            }),
        )
        .unwrap();
        assert_eq!(j.transaction(&ctx,&c,|c|{c.execute_batch("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<1000) SELECT sum(x) FROM n")?;Ok(())}).unwrap_err().code(),"canceled");
        assert!(c.is_autocommit());
        let fresh = Context::new();
        let raw = sign_change(&key, [11; 32], &Head::default(), "enroll", "alice", b"").unwrap();
        j.apply(&fresh, &Head::default(), &raw).unwrap();
    }
}
