//! Engine-side records of the authority mode (phase B): rollback witness of
//! the daemon state (EP-06) and attribution of web writes (B8).
//!
//! Both live in the Root, under the operator UID — not the daemon's — so
//! each process guards the other: the daemon cannot rewrite them and a
//! restored older daemon copy is noticed here.
//!
//! **Witness** (`.swarm/authority-witness.json`): for the daemon's account
//! journal and access audit, the highest `(seq, digest)` seen in a verified
//! assertion and when it was first seen. An assertion is refused with
//! `web-assertion-fork` when it shows the same `seq` with another digest,
//! and with `web-assertion-rollback` when it shows a lower `seq` although it
//! was issued after the newer head was first seen here (an older head issued
//! before that is a request still in flight, accepted). A new head is
//! persisted (temporary file, fsync, rename) **before** the request is
//! accepted; `web-assertion-witness-io` refuses otherwise. An unreadable or
//! invalid witness file refuses every assertion (`web-assertion-witness-
//! invalid`) until the operator resets it (`swarm-rust web-authority-witness
//! reset`, used by `./swarm.sh authority accept-restore` after a deliberate
//! restore). Limit: restoring the daemon state *and* this file together
//! (root) is not detected.
//!
//! **Attribution ledger** (`.swarm/web-attribution.jsonl`, the daemon audit
//! format with its SHA-256 chain): for every assertion-authenticated request
//! that is not GET/HEAD, an `intent` line (account subject, method, path,
//! `reference: assertion:<nonce>`, the same reference as the daemon audit line) is written **before**
//! the route runs — failure refuses the request — and an `outcome` line with
//! the HTTP status after (best effort). Engine records themselves keep the
//! process identity (`host/uid`), which also acts as an authority there.
use serde::{Deserialize, Serialize};
use std::path::Path;
use swarm_authority::assertion::Witness;
use swarm_authority::audit::{Audit, Event};

/// Highest head seen for one daemon stream.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stream {
    seq: i64,
    digest: String,
    first_seen: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    schema_version: i64,
    journal: Stream,
    audit: Stream,
}

/// Lazily loaded witness and ledger of one running service.
#[derive(Default)]
pub(crate) struct AuthorityRecords {
    witness: std::sync::Mutex<Option<Stored>>,
    ledger: std::sync::Mutex<Option<Audit>>,
}

const WITNESS: &str = "authority-witness.json";
const LEDGER: &str = "web-attribution.jsonl";

fn load(root: &Path) -> Result<Stored, &'static str> {
    match std::fs::read(root.join(".swarm").join(WITNESS)) {
        Ok(raw) => match serde_json::from_slice::<Stored>(&raw) {
            Ok(s) if s.schema_version == 1 => Ok(s),
            _ => Err("web-assertion-witness-invalid"),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Stored { schema_version: 1, ..Default::default() }),
        Err(_) => Err("web-assertion-witness-invalid"),
    }
}

fn persist(root: &Path, stored: &Stored) -> Result<(), &'static str> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let dir = root.join(".swarm");
    let tmp = dir.join(format!(".{WITNESS}.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec(stored).map_err(|_| "web-assertion-witness-io")?;
    let result = (|| {
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, dir.join(WITNESS))?;
        std::fs::File::open(&dir)?.sync_all()
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
        return Err("web-assertion-witness-io");
    }
    Ok(())
}

/// Compare one stream; returns whether the stored head advanced.
fn advance(stored: &mut Stream, seq: i64, digest: &str, issued_at: i64, now: i64) -> Result<bool, &'static str> {
    use std::cmp::Ordering::*;
    match seq.cmp(&stored.seq) {
        Greater => {
            *stored = Stream { seq, digest: digest.to_owned(), first_seen: now };
            Ok(true)
        }
        Equal if digest == stored.digest => Ok(false),
        Equal => Err("web-assertion-fork"),
        Less if issued_at > stored.first_seen => Err("web-assertion-rollback"),
        Less => Ok(false),
    }
}

impl AuthorityRecords {
    /// Check `witness` of an assertion issued at `issued_at` against the
    /// Root witness and persist any newer head before returning Ok.
    pub(crate) fn check(&self, root: &Path, witness: &Witness, issued_at: i64, now: i64) -> Result<(), &'static str> {
        let mut cell = self.witness.lock().map_err(|_| "web-assertion-witness-io")?;
        if cell.is_none() {
            *cell = Some(load(root)?);
        }
        let Some(current) = cell.as_ref() else { return Err("web-assertion-witness-io") };
        let mut next = current.clone();
        let audit_seq = i64::try_from(witness.audit_seq).map_err(|_| "web-assertion-witness-invalid")?;
        let a = advance(&mut next.journal, witness.journal_seq, &witness.journal_digest, issued_at, now)?;
        let b = advance(&mut next.audit, audit_seq, &witness.audit_digest, issued_at, now)?;
        if a || b {
            persist(root, &next)?;
            *cell = Some(next);
        }
        Ok(())
    }

    /// Append one ledger line; opens the ledger on first use.
    #[allow(clippy::too_many_arguments)] // one ledger line's fields
    pub(crate) fn record(&self, root: &Path, event: &str, subject: &str, nonce: &str, method: &str, path: &str, status: Option<u16>, now: i64) -> Result<(), &'static str> {
        let mut cell = self.ledger.lock().map_err(|_| "web-attribution-unavailable")?;
        if cell.is_none() {
            *cell = Some(Audit::open_file(&root.join(".swarm").join(LEDGER)).map_err(|_| "web-attribution-unavailable")?);
        }
        let Some(ledger) = cell.as_ref() else { return Err("web-attribution-unavailable") };
        let reference = format!("assertion:{nonce}");
        ledger
            .record(&Event { at: now, event, subject: Some(subject), credential: Some("assertion"), reference: Some(&reference), method: Some(method), path: Some(path), action: None, status })
            .map(|_| ())
            .map_err(|_| "web-attribution-unavailable")
    }
}

/// Operator reset after a deliberate restore of the daemon state: remove
/// the Root witness (the next assertion re-establishes it) and note it in
/// the attribution ledger. Must run while the engine is stopped (a running
/// engine keeps its in-memory witness). Errors: `web-assertion-witness-io`.
pub fn reset_witness(root: &Path, actor: &str, now: i64) -> Result<bool, &'static str> {
    let path = root.join(".swarm").join(WITNESS);
    let existed = match std::fs::remove_file(&path) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err("web-assertion-witness-io"),
    };
    let ledger = Audit::open_file(&root.join(".swarm").join(LEDGER)).map_err(|_| "web-attribution-unavailable")?;
    ledger
        .record(&Event { at: now, event: "witness-reset", subject: Some(actor), ..Default::default() })
        .map_err(|_| "web-attribution-unavailable")?;
    Ok(existed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn witness(j: i64, a: u64) -> Witness {
        Witness { journal_seq: j, journal_digest: format!("j{j}"), audit_seq: a, audit_digest: format!("a{a}") }
    }

    #[test]
    fn advance_accepts_newer_and_in_flight_refuses_fork_and_rollback() {
        let mut s = Stream::default();
        assert_eq!(advance(&mut s, 3, "x", 100, 100), Ok(true));
        assert_eq!((s.seq, s.first_seen), (3, 100));
        assert_eq!(advance(&mut s, 3, "x", 101, 101), Ok(false), "same head");
        assert_eq!(advance(&mut s, 3, "y", 101, 101), Err("web-assertion-fork"));
        assert_eq!(advance(&mut s, 2, "w", 100, 101), Ok(false), "older head issued before the newer was seen: in flight");
        assert_eq!(advance(&mut s, 2, "w", 101, 101), Err("web-assertion-rollback"));
        assert_eq!(s.seq, 3, "refusals change nothing");
    }

    #[test]
    fn check_persists_witness_and_refuses_invalid_until_reset() {
        let dir = { let area = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures"); std::fs::create_dir_all(&area).unwrap(); tempfile::tempdir_in(area) }.unwrap();
        std::fs::create_dir(dir.path().join(".swarm")).unwrap();
        let records = AuthorityRecords::default();
        records.check(dir.path(), &witness(5, 9), 10, 10).unwrap();
        let saved: Stored = serde_json::from_slice(&std::fs::read(dir.path().join(".swarm").join(WITNESS)).unwrap()).unwrap();
        assert_eq!((saved.journal.seq, saved.audit.seq), (5, 9));
        // A fresh service (restart) reads the persisted witness.
        let restarted = AuthorityRecords::default();
        assert_eq!(restarted.check(dir.path(), &witness(4, 9), 11, 11), Err("web-assertion-rollback"));
        assert_eq!(restarted.check(dir.path(), &witness(5, 8), 11, 11), Err("web-assertion-rollback"));
        // Corrupted witness: every assertion refused, then an explicit reset.
        std::fs::write(dir.path().join(".swarm").join(WITNESS), b"{").unwrap();
        assert_eq!(AuthorityRecords::default().check(dir.path(), &witness(5, 9), 12, 12), Err("web-assertion-witness-invalid"));
        assert_eq!(reset_witness(dir.path(), "host/uid:1", 12), Ok(true));
        assert_eq!(AuthorityRecords::default().check(dir.path(), &witness(1, 1), 13, 13), Ok(()));
        let ledger = std::fs::read_to_string(dir.path().join(".swarm").join(LEDGER)).unwrap();
        assert!(ledger.contains("witness-reset") && ledger.contains("host/uid:1"));
    }
}
