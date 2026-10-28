//! Public material-path regressions for the association finding in201.
//!
//! Uses only public Root initialization/work/task/gate APIs to create positive
//! state. Rename/replacement and artifact writes affect fresh owned inactive test
//! Roots only; no SQL, provider, identity forgery, running mission or gate bypass.
//! Passing gate and ordinary-review controls precede each reader variant under
//! the actual saved installation policy and task-required checks. Before264 the
//! writer refused without effects, while the gate reader accepted replacement
//! material. Preserve both facts; neither proves all audit candidates or path ABA.
//!
//! Review axes: all literals are fixture/protocol inputs drawn from existing
//! root_gate/root_lifecycle controls, not operator thresholds or product defaults.
//! Helpers document ownership, observations and preservation; no production API
//! is changed. Refusal is checked against exact public state and all known-effect
//! wrappers, not Err alone. Original/independent Roots are explicitly closed;
//! failures preserve this fixture for diagnosis. Before-fix execution is owned
//! by the root agent; source/format checks alone do not prove an exploitable write.
//! The SDK example reuses these same cases against the ordinary engine library;
//! cfg(test) only enables the two stable libtest wrapper names below.
use crate::{
    Agent, CostSummary, EngineSettingsDocument, RecordedEvent, RootStore, RootStoreError,
    SystemEnvironment, Work,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fs, os::unix::fs::PermissionsExt, path::PathBuf};

/// Fresh test-owned container; preserves original material and observations
/// on failure. Normal success removes only this regenerable fixture.
struct Fixture(PathBuf);
impl Fixture {
    /// Create a private directory using the existing real identifier producer.
    /// Mode0700 is fixture ownership, not a new product execution policy.
    fn new() -> Self {
        let path = std::env::temp_dir().join(crate::generate_identifier("material-path-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    /// Keep failed fixture bytes for diagnosis; never delete an existing Root or
    /// a sibling's cache. No writer process survives this synchronous test.
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("material-path fixture retained: {}", self.0.display());
        } else {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Encode the actual public event/revision envelope, with no injected producer
/// observation or schema default. Values here are explicit owned test inputs.
fn request(event: &str, revision: i64, mut fields: serde_json::Value) -> Vec<u8> {
    fields["schema_version"] = 1.into();
    fields["event_id"] = event.into();
    fields["expected_revision"] = revision.into();
    serde_json::to_vec(&fields).unwrap()
}
/// Create an ordinary inactive Work/task through current public writes. No SQL
/// seed, launch profile, consent, agent, provider or task acceptance is invented.
fn work(store: &mut RootStore) -> Work {
    let created=store.execute_external("","work.create",&request("material-create",0,serde_json::json!({"title":"Material association fixture","objective":"Gate writer must consume its own Root","scope":"Owned inert fixture","criteria":["Current proof"]})),&mut SystemEnvironment).unwrap().result.work;
    store.execute_external(&created.id,"task.add",&request("material-task",created.revision,serde_json::json!({"id":"task","title":"Gate material","deliverable":"Fixture report","criteria":["Current proof"]})),&mut SystemEnvironment).unwrap().result.work
}
/// Build the same method3 gate fixture used by root_lifecycle/root_gate, using
/// the real created task's required check IDs/phases and the supplied file bytes.
/// PASS declarations exercise manual gate recording, not executable control or
/// independent review provenance; none of those authorities is claimed here.
fn gate_document(task: &crate::Task, proof: &[u8]) -> serde_json::Value {
    let checks:Vec<_>=task.plan_checks.as_ref().expect("public task must carry saved gate checks").iter().map(|(id,phase)|serde_json::json!({"id":id,"gate":phase,"domain":"quality","mandatory":true,"severity":"minor","penalty":1,"max_penalty":100})).collect();
    let results:Vec<_>=checks.iter().map(|c|serde_json::json!({"id":c["id"],"status":"PASS","count":0,"evidence":["proof.txt"]})).collect();
    serde_json::json!({"method_version":"3","scope_id":task.id,"artifacts":{"proof.txt":format!("{:x}",Sha256::digest(proof))},"domains":{"quality":1},"checks":checks,"results":results})
}
/// Observe original datastore state through supported read APIs only. These
/// complete rows/cost groups are compared after restoration, not fetched from
/// the replacement. Gate recording must not alter unrelated settings/agents/usage.
struct Snapshot {
    work: Work,
    events: Vec<RecordedEvent>,
    settings: EngineSettingsDocument,
    agents: Vec<Agent>,
    costs: CostSummary,
}
impl Snapshot {
    /// Capture canonical public facts without deriving any acceptance/liveness.
    fn read(store: &RootStore, id: &str) -> Self {
        Self {
            work: store.read_work(id).unwrap(),
            events: store.events(id).unwrap(),
            settings: store.settings().unwrap(),
            agents: store.list_agents(id).unwrap(),
            costs: store.reported_costs(id).unwrap(),
        }
    }
    /// Compare exact observations without printing full private Work documents.
    fn same(&self, other: &Self) -> bool {
        self.work == other.work
            && self.events == other.events
            && self.settings == other.settings
            && self.agents == other.agents
            && self.costs == other.costs
    }
}
/// Inspect every established native error branch for known recorded effects,
/// including wrapped/secondary errors; source() alone would skip some wrappers.
/// This pure diagnostic never reads or formats private error payloads.
fn recorded(error: &(dyn Error + 'static)) -> bool {
    let mut pending = vec![error];
    while let Some(error) = pending.pop() {
        if let Some(error) = error.downcast_ref::<RootStoreError>() {
            if error.recorded_work().is_some()
                || error.configuration_may_be_recorded()
                || matches!(error, RootStoreError::LifecycleRecorded { .. })
            {
                return true;
            }
        }
        pending.extend(
            crate::native_error_links(error)
                .children
                .into_iter()
                .map(|(_, cause)| cause),
        );
    }
    false
}

/// Verify the writer refuses a gate under a replacement material pathname.
/// The before264 control already refuses without recording; no writer exploit
/// was reproduced. Preserve that existing protection during reader correction.
/// Retarget/restore finish before asserting so the original is never left moved
/// merely because the defect was reproduced. On a correct refusal the identical
/// request succeeds after restoration, proving a live positive recovery path.
/// No whole-mission, reviewer, HTTP, transient-ABA or confinement claim follows.
pub(crate) fn writer_case() {
    let fixture = Fixture::new();
    let project = fixture.0.join("project");
    let retained = fixture.0.join("original-held");
    let replacement = fixture.0.join("replacement");
    for path in [&project, &replacement] {
        fs::create_dir(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let proof = b"owned-current-proof";
    let mut original = RootStore::initialize(&project, &mut SystemEnvironment).unwrap();
    fs::write(project.join("proof.txt"), proof).unwrap();
    let w = work(&mut original);
    let document = gate_document(&w.tasks.as_ref().unwrap()[0], proof);
    let control = request(
        "material-valid-control",
        w.revision,
        serde_json::json!({"task_id":"task","phase":"delivery","name":"Valid original material","document":document}),
    );
    let saved = original
        .execute_external(&w.id, "gate", &control, &mut SystemEnvironment)
        .expect("valid public gate control must pass before retarget");
    assert!(!saved.result.replayed);
    assert_eq!(saved.result.work.revision, w.revision + 1);
    let observed = original.inspect_gate_evidence(&w.id, "task").unwrap();
    assert!(observed.evaluation.ship_allowed);
    let replacement_store = RootStore::initialize(&replacement, &mut SystemEnvironment).unwrap();
    assert!(replacement_store.list_works().unwrap().is_empty());
    replacement_store.close().unwrap();
    fs::write(replacement.join("proof.txt"), proof).unwrap();
    let before = Snapshot::read(&original, &w.id);
    let attacked = request(
        "material-retarget-attempt",
        before.work.revision,
        serde_json::json!({"task_id":"task","phase":"delivery","name":"Retarget material must refuse","document":document}),
    );

    fs::rename(&project, &retained).unwrap();
    fs::rename(&replacement, &project).unwrap();
    // Only the replacement now satisfies the exact gate document. This makes
    // the wrong material consumption discriminating, not merely a renamed path.
    fs::write(retained.join("proof.txt"), b"different-original-proof").unwrap();
    assert_ne!(
        fs::read(retained.join("proof.txt")).unwrap(),
        fs::read(project.join("proof.txt")).unwrap()
    );
    let outcome = original.execute_external(&w.id, "gate", &attacked, &mut SystemEnvironment);
    let was_refused = outcome.is_err();
    let known_recorded = outcome.as_ref().err().is_some_and(|e| recorded(e));
    // Restore both original file bytes and directory association before checking
    // the result or using the live object again. The replacement DB is kept.
    fs::write(retained.join("proof.txt"), proof).unwrap();
    fs::rename(&project, &replacement).unwrap();
    fs::rename(&retained, &project).unwrap();
    let after = Snapshot::read(&original, &w.id);
    let unchanged = before.same(&after);
    let recovered = if was_refused && !known_recorded && unchanged {
        let resumed = original
            .execute_external(&w.id, "gate", &attacked, &mut SystemEnvironment)
            .expect("identical request must recover after restoring original Root");
        !resumed.result.replayed
            && resumed.result.work.revision == before.work.revision + 1
            && original.events(&w.id).unwrap().len() == before.events.len() + 1
            && original
                .inspect_gate_evidence(&w.id, "task")
                .unwrap()
                .evaluation
                .ship_allowed
    } else {
        false
    };
    original.close().unwrap();
    let replacement_store = RootStore::open(&replacement, false).unwrap();
    let replacement_unchanged = replacement_store.list_works().unwrap().is_empty();
    replacement_store.close().unwrap();
    assert!(
        was_refused,
        "writer accepted a gate using material from the retargeted Root"
    );
    assert!(!known_recorded, "refusal retained a known recorded effect");
    assert!(
        unchanged,
        "retarget refusal changed original Work/events/settings/agents/usage"
    );
    assert!(
        replacement_unchanged,
        "writer changed replacement datastore"
    );
    assert!(
        recovered,
        "correct refusal did not recover after restoring original material path"
    );
}

/// Establish actual valid gate/review observations for this manual fixture.
/// Complete serialized observations include policy and observed revisions; they
/// stay in memory for exact recovery comparison and are never printed. Review
/// absence is expected here because no review has been declared or fabricated.
fn reader_control(store: &RootStore, work: &str) -> (serde_json::Value, serde_json::Value) {
    let gate = store
        .inspect_gate_evidence(work, "task")
        .expect("valid gate reader control");
    assert!(gate.evaluation.ship_allowed);
    let review = store
        .inspect_review_evidence(work, "task")
        .expect("valid review reader control");
    assert!(review.review.is_none());
    (
        serde_json::to_value(gate).unwrap(),
        serde_json::to_value(review).unwrap(),
    )
}

/// Retain safe result facts while the fixture path is substituted. Assertions
/// happen only after restoration/close; no private errors or documents are dumped.
struct ReaderAttempt {
    gate_refused: bool,
    review_refused: bool,
    ship_allowed: bool,
    known_recorded: bool,
}
impl ReaderAttempt {
    /// Exercise both public routes independently, retaining all known-effect
    /// error branches even when another route unexpectedly succeeds.
    fn read(store: &RootStore, work: &str) -> Self {
        let gate = store.inspect_gate_evidence(work, "task");
        let review = store.inspect_review_evidence(work, "task");
        Self {
            gate_refused: gate.is_err(),
            review_refused: review.is_err(),
            ship_allowed: gate.as_ref().is_ok_and(|g| g.evaluation.ship_allowed),
            known_recorded: gate.as_ref().err().is_some_and(|e| recorded(e))
                || review.as_ref().err().is_some_and(|e| recorded(e)),
        }
    }
    /// Require rejection without known recorded effects. A successful None review
    /// is still a wrong success under an invalid association, never a refusal.
    fn assert_refused(self, variant: &str) {
        assert!(
            self.gate_refused,
            "gate reader accepted {variant}; ship_allowed={}",
            self.ship_allowed
        );
        assert!(self.review_refused, "review reader accepted {variant}");
        assert!(
            !self.known_recorded,
            "reader refusal carried a recorded effect: {variant}"
        );
    }
}

/// Independent reader regression: unlike a writer, a read need not encounter
/// SQLite's moved-database protection. The retained original Work must not have
/// its gate validated against a replacement Root's matching material. Original
/// artifact bytes and both Root paths are restored before assertions; no SQL or
/// write API is used during attacks. Missing Root and replacement of only the
/// state.db pathname are separate variants. Every variant has a valid gate and
/// ordinary-review control, then exact public-state/evidence recovery. Review
/// None is a valid undeclared result only while the Root association is valid;
/// these cases establish no managed review/Git/receipt or transient-ABA coverage.
pub(crate) fn reader_case() {
    let fixture = Fixture::new();
    let project = fixture.0.join("project");
    let retained = fixture.0.join("original-held");
    let replacement = fixture.0.join("replacement");
    for path in [&project, &replacement] {
        fs::create_dir(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let proof = b"owned-current-proof";
    let mut original = RootStore::initialize(&project, &mut SystemEnvironment).unwrap();
    fs::write(project.join("proof.txt"), proof).unwrap();
    let w = work(&mut original);
    let document = gate_document(&w.tasks.as_ref().unwrap()[0], proof);
    let control = request(
        "material-reader-valid-control",
        w.revision,
        serde_json::json!({"task_id":"task","phase":"delivery","name":"Valid original material","document":document}),
    );
    let saved = original
        .execute_external(&w.id, "gate", &control, &mut SystemEnvironment)
        .expect("valid public gate control must pass before reader retarget");
    assert!(!saved.result.replayed);
    assert_eq!(saved.result.work.revision, w.revision + 1);
    let expected = reader_control(&original, &w.id);
    let replacement_store = RootStore::initialize(&replacement, &mut SystemEnvironment).unwrap();
    assert!(replacement_store.list_works().unwrap().is_empty());
    replacement_store.close().unwrap();
    fs::write(replacement.join("proof.txt"), proof).unwrap();
    let before = Snapshot::read(&original, &w.id);

    fs::rename(&project, &retained).unwrap();
    fs::rename(&replacement, &project).unwrap();
    fs::write(retained.join("proof.txt"), b"different-original-proof").unwrap();
    assert_ne!(
        fs::read(retained.join("proof.txt")).unwrap(),
        fs::read(project.join("proof.txt")).unwrap()
    );
    let retarget = ReaderAttempt::read(&original, &w.id);
    fs::write(retained.join("proof.txt"), proof).unwrap();
    fs::rename(&project, &replacement).unwrap();
    fs::rename(&retained, &project).unwrap();
    let retarget_unchanged = before.same(&Snapshot::read(&original, &w.id));
    let retarget_recovered = reader_control(&original, &w.id) == expected;

    // A valid control immediately precedes each injection. The prior reader
    // result is retained until every pathname is restored and both Roots close.
    assert!(
        reader_control(&original, &w.id) == expected,
        "valid reader control changed before injection"
    );
    let before_missing = Snapshot::read(&original, &w.id);
    fs::rename(&project, &retained).unwrap();
    let missing = ReaderAttempt::read(&original, &w.id);
    fs::rename(&retained, &project).unwrap();
    let missing_unchanged = before_missing.same(&Snapshot::read(&original, &w.id));
    let missing_recovered = reader_control(&original, &w.id) == expected;

    assert!(
        reader_control(&original, &w.id) == expected,
        "valid reader control changed before injection"
    );
    let before_database = Snapshot::read(&original, &w.id);
    let database = project.join(".swarm/state.db");
    let held_database = fixture.0.join("original-state.db-held");
    let donor_database = replacement.join(".swarm/state.db");
    // Both databases were created by public initialization; the donor is closed.
    // Keep the actual .swarm directory and original live connection unchanged.
    // No SQL, database copy, identity edit, write API or journal edit is involved.
    fs::rename(&database, &held_database).unwrap();
    fs::rename(&donor_database, &database).unwrap();
    let database_replaced = ReaderAttempt::read(&original, &w.id);
    fs::rename(&database, &donor_database).unwrap();
    fs::rename(&held_database, &database).unwrap();
    let database_unchanged = before_database.same(&Snapshot::read(&original, &w.id));
    let database_recovered = reader_control(&original, &w.id) == expected;
    let recovery_unchanged = before.same(&Snapshot::read(&original, &w.id));
    original.close().unwrap();
    let replacement_store = RootStore::open(&replacement, false).unwrap();
    let replacement_unchanged = replacement_store.list_works().unwrap().is_empty();
    replacement_store.close().unwrap();

    retarget.assert_refused("replacement Root");
    missing.assert_refused("missing Root");
    database_replaced.assert_refused("replacement state.db in original .swarm");
    assert!(
        retarget_unchanged,
        "retarget reader changed original datastore facts"
    );
    assert!(
        missing_unchanged,
        "missing Root reader changed original datastore facts"
    );
    assert!(
        database_unchanged,
        "replacement DB reader changed original datastore facts"
    );
    assert!(
        replacement_unchanged,
        "reader changed replacement datastore"
    );
    assert!(
        retarget_recovered,
        "readers did not recover after Root retarget"
    );
    assert!(
        missing_recovered,
        "readers did not recover after missing Root"
    );
    assert!(
        database_recovered,
        "readers did not recover after DB replacement"
    );
    assert!(
        recovery_unchanged,
        "reader recovery changed original datastore facts"
    );
}

/// Libtest entry retaining the permanent writer regression name. The case is
/// also used by the SDK example against the ordinary non-cfg(test) engine.
#[cfg(test)]
#[test]
fn public_gate_writer_refuses_retargeted_material_without_recording_and_recovers() {
    writer_case();
}

/// Libtest entry retaining the independent reader regression name. The SDK
/// invokes the same public-API case without compiling engine test-only paths.
#[cfg(test)]
#[test]
fn public_gate_reader_refuses_retargeted_material_and_recovers() {
    reader_case();
}
