//! source_origin.rs — private bounded module for native task validation origin
//! attribution. Rows bind job sources to observation hashes; nothing here
//! grants a lease, capability, or execution permission. Parent writers use the
//! actual borrowed owning handle; the gate separately checks live authority.
use crate::agent_launch::query;
use crate::AgentLaunchError as E;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

const NAME: &str = "native_validation_source_origins";
const DDL: &str = "CREATE TABLE native_validation_source_origins(job_id TEXT PRIMARY KEY REFERENCES native_validation_jobs(id),source_sha256 TEXT NOT NULL,controls_sha256 TEXT NOT NULL,candidate_binding_sha256 TEXT)";
const DOMAIN: &str = "swarm-native-validation-origin-v1";

/// Preserve typed primary/secondary errors under one safe static family.
fn caused(error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::caused("native-validation-source-origin", error)
}

/// Immutable source-origin observation for a validation job. No field confers
/// authority; all hashes are recorded comparisons, never actor authentication.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Origin {
    job: String,
    source_sha256: String,
    controls_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_binding_sha256: Option<String>,
}

/// Validate protocol representation before hashing or touching SQL. All digest
/// fields, including the controls digest, are exact SHA256 hex; this is no
/// operator policy or authority assertion. Invalid data refuses without IO.
fn validate(origin: &Origin) -> Result<(), E> {
    if !swarm_identity_contract::name(origin.job.as_bytes()) {
        return Err(E::Refused("native-validation-source-origin-job-invalid"));
    }
    for hash in [&origin.source_sha256, &origin.controls_sha256]
        .into_iter()
        .chain(origin.candidate_binding_sha256.iter())
    {
        if !crate::provider_configuration::configuration_digest_valid(hash) {
            return Err(E::Refused("native-validation-source-origin-hash-invalid"));
        }
    }
    Ok(())
}

impl Origin {
    /// Construct and fully validate an origin. Inputs: `job` job identifier
    /// validated by the identity contract; `source` a 64-hex SHA-256 of the
    /// prepared source; `controls` any serializable controls document, hashed
    /// over its canonical model encoding; `candidate` optional 64-hex binding
    /// hash (ordinary producer uses `None`; schema3 readers require its full
    /// candidate preimage, while actual S2 execution hookup remains pending). Errors:
    /// `E::Refused` for invalid identifiers or hash shapes, typed serialization
    /// causes preserved otherwise. No defaults are applied.
    pub(super) fn new(
        job: &str,
        source: &str,
        controls: &impl Serialize,
        candidate: Option<&str>,
    ) -> Result<Origin, E> {
        if !swarm_identity_contract::name(job.as_bytes()) {
            return Err(E::Refused("native-validation-source-origin-job-invalid"));
        }
        for hash in std::iter::once(source).chain(candidate) {
            if !crate::provider_configuration::configuration_digest_valid(hash) {
                return Err(E::Refused("native-validation-source-origin-hash-invalid"));
            }
        }
        let encoded = crate::encode_model(controls).map_err(caused)?;
        let controls_sha256 = crate::provider_configuration::configuration_digest(&encoded);
        Ok(Origin {
            job: job.into(),
            source_sha256: source.into(),
            controls_sha256,
            candidate_binding_sha256: candidate.map(|c| c.to_string()),
        })
    }
    /// Read the validated row's optional candidate binding to select the typed
    /// private helper mode. A digest is not a permit or a substitute for its full
    /// preimage; read()/constructor validation remains mandatory before use.
    pub(super) fn candidate_binding(&self) -> Option<&str> { self.candidate_binding_sha256.as_deref() }
    /// Compare current source with the recorded base before adding origin and
    /// mission domains. Invalid shapes/drift refuse, without IO or backfill.
    pub(super) fn verify_base_source(&self, base: &str) -> Result<(), E> {
        validate(self)?;
        if self.source_sha256 != base { return Err(E::Refused("native-validation-source-changed")); }
        Ok(())
    }
    /// Check the exact optional candidate digest expected from a retained
    /// preimage. None means ordinary controls and cannot accept an opaque Some.
    /// Shape/option mismatch refuses without IO. The caller must compute the
    /// expected digest from the actual candidate; claimed hashes grant nothing.
    pub(super) fn verify_candidate_binding(&self, expected: Option<&str>) -> Result<(), E> {
        validate(self)?;
        if self.candidate_binding_sha256.as_deref() != expected {
            return Err(E::Refused("native-validation-origin-candidate-mismatch"));
        }
        Ok(())
    }
    /// Check a controls preimage and outer job identity before pure conversion.
    /// This is allocation/codec work only, with safe typed refusals on a changed
    /// preimage or malformed origin. It establishes no SQL backing, current
    /// source, permission, native process outcome or independent acceptance.
    pub(super) fn verify_controls(&self, job: &str, controls: &impl Serialize) -> Result<(), E> {
        validate(self)?;
        if self.job != job {
            return Err(E::Refused("native-validation-origin-receipt-mismatch"));
        }
        let bytes = crate::encode_model(controls).map_err(caused)?;
        if crate::provider_configuration::configuration_digest(&bytes) != self.controls_sha256 {
            return Err(E::Refused("native-validation-origin-controls-mismatch"));
        }
        Ok(())
    }
}

/// Check exact optional representation without creation. Added triggers or
/// malformed storage refuse; absent table is historical absence, not permission.
fn schema(db: &Connection) -> Result<bool, E> {
    crate::root_store::mission_supervision_store::auxiliary_schema(
        db,
        NAME,
        DDL,
        "native-validation-source-origin-schema-invalid",
    )
    .map_err(caused)
}

/// Domain-separated digest of this origin tuple. Input: the stored origin.
/// Output: SHA-256 over the tagged serialization via `configuration_digest`.
/// Serialization errors preserve their cause. This hash never authenticates
/// actors and is not an execution permission.
pub(super) fn digest(origin: &Origin) -> Result<String, E> {
    validate(origin)?;
    let encoded = crate::encode_model(&(DOMAIN, origin)).map_err(caused)?;
    Ok(crate::provider_configuration::configuration_digest(
        &encoded,
    ))
}

/// Read one origin under the caller's transaction. Absence can only match:
/// missing table or missing row both yield `Ok(None)`. Wrong SQL shape, bad
/// types, malformed stored hashes, or a mismatched job id fail with typed
/// causes; no error is swallowed into None.
pub(super) fn read(db: &Connection, job: &str) -> Result<Option<Origin>, E> {
    if !swarm_identity_contract::name(job.as_bytes()) {
        return Err(E::Refused("native-validation-source-origin-job-invalid"));
    }
    if !schema(db)? {
        return Ok(None);
    }
    query(db, "SELECT source_sha256,controls_sha256,candidate_binding_sha256 FROM native_validation_source_origins WHERE job_id=?", [job], |rows| {
        rows.next()?.map(|r| {
            let source_sha256: String = r.get(0)?;
            let controls_sha256: String = r.get(1)?;
            let candidate_binding_sha256: Option<String> = r.get(2)?;
            let origin = Origin { job: job.into(), source_sha256, controls_sha256, candidate_binding_sha256 };
            validate(&origin)?;
            Ok(origin)
        }).transpose()
    })
}

/// Record an origin inside the caller's actual transaction. No nested
/// transaction, no upsert, no ignored conflict; schema is created with the
/// exact `if absent` DDL when needed. The insert must affect exactly one row
/// and a readback must equal the staged value. A failure requires the caller
/// to roll back: this function does not itself undo staged rows/DDL. Typed causes
/// are preserved; the immutable row is no permission or source authenticity proof.
pub(super) fn stage(db: &Connection, origin: &Origin) -> Result<(), E> {
    validate(origin)?;
    if db.is_autocommit() {
        return Err(E::Refused("native-validation-writer-required"));
    }
    if !schema(db)? {
        db.execute_batch(DDL).map_err(caused)?;
    }
    let n = crate::root_store::mission_supervision_store::execute(
        db,
        "INSERT INTO native_validation_source_origins VALUES(?,?,?,?)",
        params![
            origin.job,
            origin.source_sha256,
            origin.controls_sha256,
            origin.candidate_binding_sha256
        ],
    )
    .map_err(caused)?;
    if n != 1 {
        return Err(E::Refused("native-validation-source-origin-not-recorded"));
    }
    if read(db, &origin.job)?.as_ref() != Some(origin) {
        return Err(E::Refused(
            "native-validation-source-origin-readback-mismatch",
        ));
    }
    Ok(())
}

/// Compare a current ordinary base source against its recorded origin domain.
/// This ordinary permission/review reader requires no managed candidate claim;
/// managed execution needs an explicit future candidate-aware source path.
/// If no origin row exists, return `base` unchanged: legacy manual jobs keep
/// their exact source bytes. If an origin row exists, its stored source hash
/// must equal `base` (otherwise the source changed under the record) and the
/// domain digest of the origin is returned. Errors: typed read/serialization
/// causes; refusal on stored/actual source mismatch. Removing the row cannot
/// recreate the stored job source: new jobs hash via this domain before
/// `mission_authority::digest`, so the separation protects downgrades. No
/// backfill or migration is performed.
pub(super) fn source(db: &Connection, job: &str, base: &str) -> Result<String, E> {
    match read(db, job)? {
        None => Ok(base.into()),
        Some(origin) => {
            origin.verify_candidate_binding(None)?;
            if origin.source_sha256 != base {
                return Err(E::Refused("native-validation-source-changed"));
            }
            digest(&origin)
        }
    }
}

/// Verify an optional receipt body against the actually stored row.
/// Inputs: `job` identifier; `body` the receipt's optional origin claim.
/// Valid only when: both absent (legacy), or both present and exactly equal.
/// A receipt claiming an origin (`Some`) with no stored row is refused — a
/// new-style receipt cannot retroactively attach to a legacy job. Mismatched
/// or malformed stored rows refuse with typed causes. This compares records
/// only; it grants no lease and accepts no external attestation.
pub(super) fn verify_receipt(db: &Connection, job: &str, body: Option<&Origin>) -> Result<(), E> {
    if let Some(body) = body {
        validate(body)?;
        if body.job != job {
            return Err(E::Refused("native-validation-origin-receipt-mismatch"));
        }
    }
    let actual = read(db, job)?;
    match (actual.as_ref(), body) {
        (None, None) => Ok(()),
        (Some(actual), Some(body)) if actual == body => Ok(()),
        (None, Some(_)) => Err(E::Refused("native-validation-origin-receipt-unbacked")),
        _ => Err(E::Refused("native-validation-origin-receipt-mismatch")),
    }
}

/// Verify immutable receipt backing against the actual scalar Job as well as its
/// origin row and exact effective-controls preimage. New jobs must match the
/// controls digest and origin-domain plus recorded mission overlay;
/// historical absent origin retains its original SQL/frame checks in the caller.
/// Reads only, preserves schema/SQL/codec causes and refuses changed source; this
/// does not grant a lease, reconstruct a process or establish current freshness.
pub(super) fn verify_job_receipt<T: Serialize>(
    db: &Connection,
    job: &super::store::Job,
    body: Option<&Origin>,
    controls: Option<&T>,
) -> Result<(), E> {
    verify_bound_job_receipt(db, job, body, controls, None)
}
/// Verify SQL/body/controls and optional full-candidate preimages against the
/// actual Job source domain plus mission overlay. Reads only; no upsert, backfill,
/// current filesystem freshness, execution or authority is established. Absent
/// body with a candidate claim and all mismatches refuse with typed safe causes.
pub(super) fn verify_bound_job_receipt<T: Serialize>(
    db: &Connection,
    job: &super::store::Job,
    body: Option<&Origin>,
    controls: Option<&T>,
    candidate_binding: Option<&str>,
) -> Result<(), E> {
    verify_receipt(db, &job.id, body)?;
    if body.is_none() && candidate_binding.is_some() {
        return Err(E::Refused("native-validation-origin-candidate-mismatch"));
    }
    if body.is_some() != controls.is_some() {
        return Err(E::Refused("native-validation-origin-controls-required"));
    }
    if let (Some(body), Some(controls)) = (body, controls) {
        body.verify_controls(&job.id, controls)?;
        body.verify_candidate_binding(candidate_binding)?;
        let combined = super::mission_authority::digest(
            &digest(body)?,
            super::mission_authority::read(db, &job.id)?.as_ref(),
        )?;
        if combined != job.source_sha256 {
            return Err(E::Refused("native-validation-origin-job-source-mismatch"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Isolated SQL shape fixture only, not a native job/permission. Actual
    /// native producers are exercised separately by the public CLI recipe.
    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE native_validation_jobs(id TEXT PRIMARY KEY); INSERT INTO native_validation_jobs VALUES('job');").unwrap();
        db
    }
    /// An explicit serialized control fixture, never a runtime fallback.
    fn origin() -> Origin {
        Origin::new(
            "job",
            &"a".repeat(64),
            &serde_json::json!({"controls":["actual fixture bytes"]}),
            None,
        )
        .unwrap()
    }
    #[test]
    fn native_source_origin_legacy_absence_preserves_base_without_writes() {
        let db = db();
        let base = "a".repeat(64);
        assert_eq!(source(&db, "job", &base).unwrap(), base);
        verify_receipt(&db, "job", None).unwrap();
        assert!(!schema(&db).unwrap());
        assert_ne!(digest(&origin()).unwrap(), base);
        assert!(verify_receipt(&db, "job", Some(&origin())).is_err());
        assert!(!schema(&db).unwrap());
    }
    #[test]
    fn native_source_origin_binds_control_changes_and_refuses_missing_row_then_recovers() {
        let db = db();
        let tx = db.unchecked_transaction().unwrap();
        let value = origin();
        stage(&tx, &value).unwrap();
        let bound = source(&tx, "job", &value.source_sha256).unwrap();
        assert_eq!(bound, digest(&value).unwrap());
        verify_receipt(&tx, "job", Some(&value)).unwrap();
        assert!(verify_receipt(&tx, "job", None).is_err());
        assert!(source(&tx, "job", &"b".repeat(64)).is_err());
        assert_eq!(source(&tx, "job", &value.source_sha256).unwrap(), bound);
        let changed = Origin::new(
            "job",
            &value.source_sha256,
            &serde_json::json!({"controls":["different"]}),
            None,
        )
        .unwrap();
        assert_ne!(digest(&changed).unwrap(), bound);
        let candidate = Origin::new(
            "job",
            &value.source_sha256,
            &serde_json::json!({"controls":["actual fixture bytes"]}),
            Some(&"c".repeat(64)),
        )
        .unwrap();
        assert_ne!(digest(&candidate).unwrap(), bound);
        tx.execute(
            "DELETE FROM native_validation_source_origins WHERE job_id='job'",
            [],
        )
        .unwrap();
        assert_ne!(source(&tx, "job", &value.source_sha256).unwrap(), bound);
        assert_eq!(
            verify_receipt(&tx, "job", Some(&value))
                .err()
                .unwrap()
                .code(),
            "native-validation-origin-receipt-unbacked"
        );
        stage(&tx, &value).unwrap();
        verify_receipt(&tx, "job", Some(&value)).unwrap();
        assert_eq!(source(&tx, "job", &value.source_sha256).unwrap(), bound);
        tx.rollback().unwrap();
        assert!(!schema(&db).unwrap());
    }
    #[test]
    fn native_source_origin_valid_control_precedes_schema_hash_and_transaction_refusals() {
        let db = db();
        let value = origin();
        {
            let tx = db.unchecked_transaction().unwrap();
            stage(&tx, &value).unwrap();
            assert_eq!(read(&tx, "job").unwrap().as_ref(), Some(&value));
            assert!(stage(&tx, &value).is_err());
            tx.rollback().unwrap();
        }
        assert!(!schema(&db).unwrap());
        assert_eq!(
            stage(&db, &value).err().unwrap().code(),
            "native-validation-writer-required"
        );
        assert!(!schema(&db).unwrap());
        let tx = db.unchecked_transaction().unwrap();
        stage(&tx, &value).unwrap();
        tx.execute("UPDATE native_validation_source_origins SET controls_sha256='invalid' WHERE job_id='job'", []).unwrap();
        assert_eq!(
            read(&tx, "job").err().unwrap().code(),
            "native-validation-source-origin-hash-invalid"
        );
        tx.execute(
            "UPDATE native_validation_source_origins SET controls_sha256=? WHERE job_id='job'",
            [&value.controls_sha256],
        )
        .unwrap();
        assert_eq!(read(&tx, "job").unwrap().as_ref(), Some(&value));
        tx.execute_batch("CREATE TRIGGER origin_fault AFTER INSERT ON native_validation_source_origins BEGIN SELECT 1; END;").unwrap();
        assert!(read(&tx, "job").is_err());
        tx.execute_batch("DROP TRIGGER origin_fault;").unwrap();
        assert_eq!(read(&tx, "job").unwrap().as_ref(), Some(&value));
        tx.rollback().unwrap();
    }
    /// Source equality between row and frame cannot replace the scalar Job hash.
    /// Data fixture only; real source/mission/permission effects are native tests.
    #[test]
    fn native_source_origin_receipt_must_match_registered_job_domain() {
        let db = db();
        let tx = db.unchecked_transaction().unwrap();
        let value = origin();
        stage(&tx, &value).unwrap();
        let mut job = super::super::store::Job {
            id: "job".into(),
            work_id: "work".into(),
            task_id: "task".into(),
            event_id: "event".into(),
            request_sha256: "a".repeat(64),
            source_sha256: digest(&value).unwrap(),
            work_revision: 1,
            settings_revision: 1,
            owner_pid: 1,
            owner_stamp: "1".into(),
            owner_host: "fixture".into(),
            directory_device: "1".into(),
            directory_inode: "1".into(),
            control_count: 1,
            phase: super::super::store::JobPhase::Active,
            receipt_sha256: None,
            published_revision: None,
        };
        verify_job_receipt(
            &tx,
            &job,
            Some(&value),
            Some(&serde_json::json!({"controls":["actual fixture bytes"]})),
        )
        .unwrap();
        assert_eq!(
            verify_job_receipt(
                &tx,
                &job,
                Some(&value),
                Some(&serde_json::json!({"controls":["changed preimage"]}))
            )
            .err()
            .unwrap()
            .code(),
            "native-validation-origin-controls-mismatch"
        );
        job.source_sha256 = value.source_sha256.clone();
        assert_eq!(
            verify_job_receipt(
                &tx,
                &job,
                Some(&value),
                Some(&serde_json::json!({"controls":["actual fixture bytes"]}))
            )
            .err()
            .unwrap()
            .code(),
            "native-validation-origin-job-source-mismatch"
        );
        job.source_sha256 = digest(&value).unwrap();
        verify_job_receipt(
            &tx,
            &job,
            Some(&value),
            Some(&serde_json::json!({"controls":["actual fixture bytes"]})),
        )
        .unwrap();
        tx.rollback().unwrap();
    }
}
