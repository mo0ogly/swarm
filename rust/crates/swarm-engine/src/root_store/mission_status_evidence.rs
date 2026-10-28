//! Current task freshness for status, not acceptance or an execution grant.
//!
//! Mirrors score.go acceptedFreshMemo and validation_state.go Fresh/Blockers
//! from HEAD 553593a2ffa3823d16784a6fedc291eebf390837 using current Rust gate/review
//! primitives. Waivers require their real nonempty override and fresh dependencies.
//! Gate, review and dependency observations never become cached launch authority.
//! Infrastructure/configuration/unknown errors refuse the whole read; only known
//! semantic invalidity or an observed missing artifact can become NotCurrent.
//! Original semantic causes remain typed beside the presentation blockers.
//! Unknown native/managed rejection shapes remain Err until explicitly classified;
//! this conservative boundary is not a claim of complete Go status coverage.
//!
//! TaskLifecycleContext was checked: its mutation task/policy limits are not
//! needed by direct gate/review reads. The scoped lifecycle helper additionally
//! enforces requirement prerequisites and accepted-only entry, so it is not used
//! to approximate Go's waived/dependency classification. No extra control-policy
//! requirement is added via check_validation_evidence: ordinary acceptedFresh Go
//! consumes independent review, gate and dependencies; native review verification
//! retains its existing complete original-chain checks in the reused primitive.
//!
//! No defaults, thresholds, search limits, retries, reconciliation, provider/control launch,
//! SQL mutation, reservation debit/refund or task transition is introduced. Caller
//! still owes Root.close and publication/flush handling. Settings reservation and
//! owned Root binding are explicitly released with all cleanup causes retained.
//! Managed review reuses its existing read-only Git observer, which may execute
//! Git inspection processes under its saved policy; this is not a provider grant.
//! Review axes: literals are closed error/protocol classifications and Go blocker
//! text, not operator policy. Functions document actual inputs/effects/limits.
//! Semantic causes remain typed in the map; infrastructure/close failures remain
//! Err through the shared finished wrapper. No acceptance or no-change effect is
//! inferred from Err; these tests do not exercise a real Root-instance lifecycle.
use super::{finished, RootStore, RootStoreError as E};
use crate::mission_status_tasks::{FreshnessObservation, GateObservation};
use crate::{
    EngineSettingsDocument, GateContractError, GateEvaluationError, ReviewEvidenceError, Task, Work,
};
use std::{collections::BTreeMap, error::Error, io, path::Path};

/// One task's actual current observations. Causes are deliberately not Clone,
/// Debug or serialized: they retain typed originals for the trusted consumer's
/// safe error traversal, independently of the Go-facing text in freshness.
/// A map returned successfully is still only a point-in-time status observation.
pub struct TaskFreshnessEvidence {
    /// Current task/dependency proof classification, never an acceptance grant.
    pub freshness: FreshnessObservation,
    /// Actual gate observation, separate from stored task acceptance.
    pub gate: GateObservation,
    /// Original semantic rejection causes for safe traversal; never raw Display.
    pub causes: Vec<E>,
}
/// Internal supplied facts for one traversal. eligible is a result of actual
/// current gate/review/waiver inspection, never inferred from accepted alone.
struct LocalEvidence {
    eligible: bool,
    gate: GateObservation,
    blockers: Vec<String>,
    causes: Vec<E>,
}

/// Recognize only an actual missing-file IO result. Permission, unavailable
/// storage, read/close/allocation failures and unknown causes remain errors.
fn missing(error: &(dyn Error + 'static)) -> bool {
    if let Some(error) = error.downcast_ref::<io::Error>() {
        return error.kind() == io::ErrorKind::NotFound;
    }
    if let Some(crate::LocalPathError::Io { cause, .. }) =
        error.downcast_ref::<crate::LocalPathError>()
    {
        return cause.kind() == io::ErrorKind::NotFound;
    }
    false
}
/// Inspect typed primary/IO/anchored causes without parsing messages or dropping
/// wrappers. Cycles are deduplicated; this projection grants no file access and
/// does not classify cleanup as semantic (the caller rejects cleanup first).
pub(super) fn artifact_cause_matches(error: &(dyn Error + 'static), predicate: impl Fn(&(dyn Error + 'static)) -> bool) -> bool {
    let mut pending = vec![error];
    let mut seen: Vec<*const dyn Error> = vec![];
    while let Some(error) = pending.pop() {
        let pointer = error as *const dyn Error;
        if seen.iter().any(|old| std::ptr::eq(*old, pointer)) { continue; }
        seen.push(pointer);
        if predicate(error) { return true; }
        #[cfg(target_os = "linux")]
        for (_, cause) in crate::native_error_links(error).children { pending.push(cause); }
        #[cfg(not(target_os = "linux"))]
        if let Some(io) = error.downcast_ref::<io::Error>() {
            if let Some(cause) = io.get_ref() { pending.push(cause); }
        } else if let Some(cause) = error.source() { pending.push(cause); }
    }
    false
}
/// Classify the existing artifact parser's typed/closed reason vocabulary,
/// never Display text. A secondary close error always invalidates the read.
/// Unknown caused errors are conservatively returned, not called stale.
pub(super) fn semantic_artifact(error: &GateEvaluationError) -> bool {
    if error.cleanup_cause().is_some() {
        return false;
    }
    match error.reason() {
        "artifact-path" => artifact_cause_matches(error, |e| {
            missing(e)
                || matches!(
                    e.downcast_ref::<crate::LocalPathError>(),
                    Some(crate::LocalPathError::Refused(_))
                )
        }),
        "artifact-open" => artifact_cause_matches(error, missing),
        "artifact-read"
        | "artifact-metadata"
        | "artifact-close"
        | "artifact-buffer"
        | "artifact-byte-limit-exceeded" => false,
        _ => {
            error.source().is_none()
                || artifact_cause_matches(error, |e| e.is::<crate::gate_json::JsonError>())
        }
    }
}
/// Distinguish invalid supplied/current gate evidence from policy/IO failure.
/// Refused/JSON/UTF8 represent evidence rejection; all original causes are retained.
pub(super) fn semantic_gate(error: &GateContractError) -> bool {
    match error {
        GateContractError::Refused(_) | GateContractError::Json(_) | GateContractError::Utf8(_) => {
            true
        }
        GateContractError::Evaluation(error) => semantic_artifact(error),
        GateContractError::Policy(_) => false,
    }
}
/// Review state/digest/contract mismatch is semantic; settings, malformed context,
/// native Root/SQL, Git and unknown refusal causes remain whole-read errors.
/// Context uses its explicit post-read stale discriminator rather than wording.
fn semantic_review(error: &ReviewEvidenceError) -> bool {
    match error {
        ReviewEvidenceError::Refused(code) => matches!(
            *code,
            "reviewer-required"
                | "review-required"
                | "review-attempt-or-contract-stale"
                | "review-artifact-stale"
                | "review-report-stale"
        ),
        ReviewEvidenceError::Context(error) => error.is_stale(),
        ReviewEvidenceError::Artifact(error) => semantic_artifact(error),
        _ => false,
    }
}

/// After a semantic gate rejection, inspect the actual declared artifacts for
/// Go's ordered explanatory blockers. Missing/changed are observations, not IO
/// permission failures. Every semantic read cause remains typed in causes.
/// No absence is guessed from an unreadable path, partial read or failed close.
fn gate_blockers(root: &Path, task: &Task, causes: &mut Vec<E>) -> Result<Vec<String>, E> {
    let mut result = Vec::new();
    for (name, expected) in task
        .gate
        .as_ref()
        .and_then(|g| g.evaluation.artifacts.as_ref())
        .into_iter()
        .flatten()
    {
        match crate::gate_evaluation::artifact_hash(root, name) {
            Ok((_, actual)) if &actual != expected => {
                result.push(format!("Preuve modifiée : {name}"))
            }
            Ok(_) => {}
            Err(error) if semantic_artifact(&error) => {
                result.push(format!("Preuve absente ou inaccessible : {name}"));
                causes.push(E::Gate(GateContractError::Evaluation(error)));
            }
            Err(error) => return Err(E::Gate(GateContractError::Evaluation(error))),
        }
    }
    if result.is_empty() {
        result.push("Gate absente, refusée ou contrôles obligatoires non conformes".into());
    }
    Ok(result)
}
/// Inspect the task's own current gate/review/override before traversing graph.
/// Every configured gate is checked, even for an unaccepted task, as in Go's
/// validation projection; absence on accepted is the explicit missing-gate case.
fn local(
    root: &Path,
    work: &Work,
    task: &Task,
    settings: &EngineSettingsDocument,
) -> Result<LocalEvidence, E> {
    let mut causes = Vec::new();
    let mut blockers = Vec::new();
    let gate = if task.gate.is_none() {
        if task.status == "accepted" {
            blockers.push("Gate manquante".into());
        }
        GateObservation::Absent
    } else {
        let quality = settings.quality_policy_snapshot().map_err(E::Settings)?;
        match crate::check_gate_evidence(task, root, &quality) {
            Ok(_) => GateObservation::Current,
            Err(error) if semantic_gate(&error) => {
                causes.push(E::Gate(error));
                blockers = gate_blockers(root, task, &mut causes)?;
                GateObservation::NotCurrent
            }
            Err(error) => return Err(E::Gate(error)),
        }
    };
    let review = match crate::check_current_review_evidence(root, work, &task.id, settings) {
        Ok(Some(_)) => true,
        Ok(None) if work.planning.is_none() => true,
        Ok(None) => return Err(E::Refused("mission-status-review-observation-missing")),
        Err(error) if semantic_review(&error) => {
            causes.push(E::Review(error));
            false
        }
        Err(error) => return Err(E::Review(error)),
    };
    let eligible = review
        && match task.status.as_str() {
            "accepted" => matches!(gate, GateObservation::Current),
            "waived" => task
                .manual_override
                .as_ref()
                .is_some_and(|v| crate::text::nonempty(&v.reason)),
            _ => false,
        };
    Ok(LocalEvidence {
        eligible,
        gate,
        blockers,
        causes,
    })
}

/// Iterative traversal-local memo, preserving Go's cycle/missing-dependency=false
/// semantics without recursion, a depth ceiling or reuse across filesystem reads.
/// Input coordinates must already be unique; no missing local proof becomes true.
fn current(
    tasks: &BTreeMap<String, &Task>,
    local: &BTreeMap<String, LocalEvidence>,
) -> BTreeMap<String, bool> {
    let mut memo: BTreeMap<String, Option<bool>> = BTreeMap::new();
    for root in tasks.keys() {
        let mut pending = vec![(root.as_str(), false)];
        while let Some((id, exit)) = pending.pop() {
            if exit {
                let good = local.get(id).is_some_and(|l| l.eligible)
                    && tasks.get(id).is_some_and(|t| {
                        t.depends
                            .iter()
                            .flatten()
                            .all(|dep| memo.get(dep).is_some_and(|value| *value == Some(true)))
                    });
                memo.insert(id.into(), Some(good));
                continue;
            }
            if let Some(previous) = memo.get(id) {
                if previous.is_none() {
                    memo.insert(id.into(), Some(false));
                }
                continue;
            }
            if !local.get(id).is_some_and(|l| l.eligible) {
                memo.insert(id.into(), Some(false));
                continue;
            }
            memo.insert(id.into(), None);
            pending.push((id, true));
            if let Some(task) = tasks.get(id) {
                for dep in task.depends.iter().flatten().rev() {
                    pending.push((dep.as_str(), false));
                }
            }
        }
    }
    memo.into_iter()
        .map(|(id, value)| (id, value == Some(true)))
        .collect()
}
/// Complete evidence read from an already selected Work/settings pair. No Root
/// lifecycle is owned here; the public(crate) wrapper handles binding/cleanup.
fn read(
    root: &Path,
    work: &Work,
    settings: &EngineSettingsDocument,
) -> Result<BTreeMap<String, TaskFreshnessEvidence>, E> {
    settings.validate().map_err(E::Settings)?;
    let mut tasks = BTreeMap::new();
    for task in work.tasks.iter().flatten() {
        if tasks.insert(task.id.clone(), task).is_some() {
            return Err(E::Refused("mission-status-task-id-ambiguous"));
        }
    }
    let mut observed = BTreeMap::new();
    for (id, task) in &tasks {
        observed.insert(id.clone(), local(root, work, task, settings)?);
    }
    let fresh = current(&tasks, &observed);
    let mut result = BTreeMap::new();
    for (id, mut evidence) in observed {
        let task = tasks[&id];
        for dep in task.depends.iter().flatten() {
            if fresh.get(dep) != Some(&true) {
                evidence
                    .blockers
                    .push(format!("Dépendance non validée actuellement : {dep}"));
            }
        }
        let freshness = if fresh.get(&id) == Some(&true) {
            FreshnessObservation::Current
        } else {
            FreshnessObservation::NotCurrent {
                blockers: evidence.blockers,
            }
        };
        result.insert(
            id,
            TaskFreshnessEvidence {
                freshness,
                gate: evidence.gate,
                causes: evidence.causes,
            },
        );
    }
    Ok(result)
}

/// Read actual gate/review/dependency freshness for every task of supplied Work.
/// Require the same canonical Work and complete saved settings before/after the
/// read under a shared settings reservation and owned Root-instance witnesses.
/// The actual project pathname's .swarm/database association is also checked
/// before/after material reads; descriptor equality alone cannot establish it.
/// This is not an atomic filesystem snapshot or future permission; report/review
/// presentation, prepared/readiness/execution classification remain separate.
/// Unknown/mismatched Work/settings/instance, infrastructure/policy/unknown evidence
/// failures and every release error refuse the whole map with typed causes. Known
/// semantic stale causes remain in each entry; no provider/control is invoked.
/// Managed evidence may invoke its existing policy-controlled read-only Git observer.
/// No SQL mutation or new task/financial policy is introduced. Caller owes Root.close.
pub(crate) fn read_freshness(
    root: &RootStore,
    work: &Work,
    settings: &EngineSettingsDocument,
) -> Result<BTreeMap<String, TaskFreshnessEvidence>, E> {
    let reservation = root.directory.reserve(false, false).map_err(E::Storage)?;
    let binding = match root.bind_root() {
        Ok(binding) => binding,
        Err(error) => return finished(Err(error), reservation.close().map_err(E::Storage)),
    };
    let result = (|| {
        if root.read_work(&work.id)? != *work {
            return Err(E::Refused("mission-status-evidence-work-changed"));
        }
        if root.read_settings()? != *settings {
            return Err(E::Refused("mission-status-evidence-settings-changed"));
        }
        super::material_path::verify_material_path(root, &binding)?;
        let value = read(&root.root, work, settings)?;
        super::material_path::verify_material_path(root, &binding)?;
        if root.read_work(&work.id)? != *work {
            return Err(E::Refused("mission-status-evidence-work-changed"));
        }
        if root.read_settings()? != *settings {
            return Err(E::Refused("mission-status-evidence-settings-changed"));
        }
        if !binding.matches(root)? {
            return Err(E::Refused("mission-status-evidence-root-changed"));
        }
        Ok(value)
    })();
    finished(
        finished(result, binding.close()),
        reservation.close().map_err(E::Storage),
    )
}

impl RootStore {
    /// Observe current gate/review/dependency freshness for an existing Work using
    /// only this opened Root's canonical Work and complete saved settings. Caller
    /// supplies a Work ID, never injected evidence, policy, time or observations.
    /// Work and settings are first read sequentially; read_freshness then checks
    /// their exact current values before/after evidence reads under a shared
    /// settings reservation and held physical Root-instance witnesses. Before/after
    /// material reads, also verify the actual pathname's datastore association;
    /// a missing/replaced Root is an error, never an absent artifact. A change
    /// refuses rather than mixing revisions. This is not an atomic filesystem
    /// snapshot, process liveness proof, future lease or permission to accept/run.
    ///
    /// Returns every task's actual freshness/gate facts plus retained semantic
    /// causes. Missing/stale evidence is distinct from settings, read, SQL, policy,
    /// unknown native/managed failure and release errors, which return Err with
    /// typed causes and all independent cleanup branches. There is no NotObserved
    /// fallback for failed IO. Native review keeps its original-chain checks;
    /// managed review may use its existing policy-controlled read-only Git observer.
    ///
    /// No SQL/task/financial mutation, provider/control, reconciliation, new default
    /// or retry occurs. Temporary binding/reservation handles are explicitly closed;
    /// caller still owes RootStore::close and safe publication/flush error handling.
    pub fn mission_status_freshness(
        &self,
        work: &str,
    ) -> Result<BTreeMap<String, TaskFreshnessEvidence>, E> {
        let work = self.read_work(work)?;
        let settings = self.settings()?;
        read_freshness(self, &work, &settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Typed nested path/missing errors survive wrappers; permissions and text
    /// resembling ENOENT remain infrastructure errors, never stale evidence.
    #[test]
    fn status_artifact_cause_wrappers_preserve_refusal_and_missing_only() {
        let path = io::Error::other(crate::LocalPathError::Refused(crate::PathRefusal::OutsideProject));
        assert!(artifact_cause_matches(&path, |e| matches!(e.downcast_ref::<crate::LocalPathError>(), Some(crate::LocalPathError::Refused(crate::PathRefusal::OutsideProject)))));
        let absent = io::Error::other(io::Error::from(io::ErrorKind::NotFound));
        assert!(artifact_cause_matches(&absent, missing));
        let denied = io::Error::other(io::Error::from(io::ErrorKind::PermissionDenied));
        assert!(!artifact_cause_matches(&denied, missing));
        assert!(!artifact_cause_matches(&io::Error::other("No such file or directory"), missing));
    }

    /// Pure graph fixture: eligibility is deliberately supplied, not a real proof.
    fn task(id: &str, deps: &[&str]) -> Task {
        Task {
            id: id.into(),
            depends: Some(deps.iter().map(|d| (*d).into()).collect()),
            ..Default::default()
        }
    }
    /// Pure local fact fixture; no artifact, review, Root or process is observed.
    fn local(eligible: bool) -> LocalEvidence {
        LocalEvidence {
            eligible,
            gate: GateObservation::Absent,
            blockers: Vec::new(),
            causes: Vec::new(),
        }
    }
    /// Valid current graph precedes stale/missing/cycle variants. One invalid
    /// dependency invalidates the chain; unrelated current tasks stay current.
    #[test]
    fn status_freshness_graph_current_stale_missing_cycle_and_recovery() {
        let a = task("a", &[]);
        let b = task("b", &["a"]);
        let c = task("c", &[]);
        let tasks = BTreeMap::from([("a".into(), &a), ("b".into(), &b), ("c".into(), &c)]);
        let mut facts = BTreeMap::from([
            ("a".into(), local(true)),
            ("b".into(), local(true)),
            ("c".into(), local(true)),
        ]);
        assert_eq!(current(&tasks, &facts).get("b"), Some(&true));
        facts.insert("a".into(), local(false));
        let invalid = current(&tasks, &facts);
        assert_eq!(invalid.get("b"), Some(&false));
        assert_eq!(invalid.get("c"), Some(&true));
        facts.insert("a".into(), local(true));
        assert_eq!(current(&tasks, &facts).get("b"), Some(&true));
        let absent = task("b", &["missing"]);
        let missing = BTreeMap::from([("b".into(), &absent)]);
        assert_eq!(current(&missing, &facts).get("b"), Some(&false));
        assert_eq!(current(&tasks, &facts).get("b"), Some(&true));
        let cycle_a = task("a", &["b"]);
        let cycle = BTreeMap::from([("a".into(), &cycle_a), ("b".into(), &b)]);
        assert_eq!(current(&cycle, &facts).get("a"), Some(&false));
    }
    /// Exercise the real manual gate/review reader path without creating a Root
    /// or touching artifacts: waived+override is Go's positive witness. Removing
    /// the actual override or a required gate yields observed NotCurrent, not a
    /// guessed acceptance; dependency waits retain Go's explicit blocker text.
    #[test]
    fn status_freshness_manual_waiver_override_missing_gate_and_dependencies() {
        let settings = EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture/uid:1".into(),
            "2026-10-23T00:00:00Z".into(),
        )
        .unwrap();
        let mut parent = task("parent", &[]);
        parent.status = "waived".into();
        parent.manual_override = Some(Box::new(crate::ManualOverride {
            reason: "fixture decision".into(),
            ..Default::default()
        }));
        let mut child = task("child", &["parent"]);
        child.status = "waived".into();
        child.manual_override = parent.manual_override.clone();
        let mut work = Work {
            id: "fixture".into(),
            tasks: Some(vec![parent, child]),
            ..Default::default()
        };
        let observed = read(Path::new("/fixture-unused-no-artifacts"), &work, &settings).unwrap();
        assert!(matches!(
            observed["parent"].freshness,
            FreshnessObservation::Current
        ));
        assert!(matches!(
            observed["child"].freshness,
            FreshnessObservation::Current
        ));
        assert!(matches!(observed["parent"].gate, GateObservation::Absent));
        work.tasks.as_mut().unwrap()[0].manual_override = None;
        let observed = read(Path::new("/fixture-unused-no-artifacts"), &work, &settings).unwrap();
        assert!(matches!(
            observed["parent"].freshness,
            FreshnessObservation::NotCurrent { .. }
        ));
        assert!(
            matches!(&observed["child"].freshness,FreshnessObservation::NotCurrent {blockers} if blockers==&vec!["Dépendance non validée actuellement : parent".to_owned()])
        );
        let original_override = work.tasks.as_ref().unwrap()[1].manual_override.clone();
        work.tasks.as_mut().unwrap()[0].manual_override = original_override;
        assert!(matches!(
            read(Path::new("/fixture-unused-no-artifacts"), &work, &settings).unwrap()["parent"]
                .freshness,
            FreshnessObservation::Current
        ));
        work.tasks.as_mut().unwrap()[0].status = "accepted".into();
        let observed = read(Path::new("/fixture-unused-no-artifacts"), &work, &settings).unwrap();
        assert!(
            matches!(&observed["parent"].freshness,FreshnessObservation::NotCurrent {blockers} if blockers==&vec!["Gate manquante".to_owned()])
        );
    }
    /// Typed absence is semantic; permission/read/close failures are not guessed
    /// as staleness. The original IO value remains available to safe traversal.
    #[test]
    fn status_freshness_error_classes_preserve_infrastructure() {
        assert!(missing(&io::Error::from(io::ErrorKind::NotFound)));
        for kind in [
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::UnexpectedEof,
            io::ErrorKind::OutOfMemory,
        ] {
            assert!(missing(&io::Error::from(io::ErrorKind::NotFound)));
            let error = io::Error::from(kind);
            assert!(!missing(&error));
            assert_eq!(error.kind(), kind);
        }
        assert!(semantic_gate(&GateContractError::Refused(
            "gate-known-stale"
        )));
        assert!(semantic_review(&ReviewEvidenceError::Refused(
            "review-report-stale"
        )));
        assert!(semantic_gate(&GateContractError::Refused(
            "gate-known-stale"
        )));
        assert!(!semantic_review(&ReviewEvidenceError::Refused(
            "review-context-policy-required"
        )));
    }
    /// Valid actual public Root observation precedes substitution. Open SQLite
    /// and .swarm handles remain original while the pathname points elsewhere;
    /// the whole API must refuse, then recover after restoring that exact Root.
    /// No agent/provider is launched and no positive SQL proof is fabricated.
    #[test]
    fn mission_status_freshness_rejects_retargeted_or_missing_project_path() {
        use std::{fs, os::unix::fs::DirBuilderExt};
        let parent =
            std::env::temp_dir().join(crate::generate_identifier("status-path-fixture-").unwrap());
        fs::DirBuilder::new().mode(0o700).create(&parent).unwrap();
        let path = parent.join("project");
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let mut env = crate::SystemEnvironment;
        let mut original = RootStore::initialize(&path, &mut env).unwrap();
        let work = original.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Owned status path fixture","objective":"Observe only original Root","scope":"Isolated public operations","criteria":["No retargeted proof read"]}"#,&mut env).unwrap().result.work;
        assert!(original
            .mission_status_freshness(&work.id)
            .unwrap()
            .is_empty());
        let held = parent.join("original-held");
        fs::rename(&path, &held).unwrap();
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        RootStore::initialize(&path, &mut env)
            .unwrap()
            .close()
            .unwrap();
        let substituted = original.mission_status_freshness(&work.id);
        fs::rename(&path, parent.join("replacement-held")).unwrap();
        fs::rename(&held, &path).unwrap();
        let recovered = original.mission_status_freshness(&work.id);
        fs::rename(&path, &held).unwrap();
        let missing_path = original.mission_status_freshness(&work.id);
        fs::rename(&held, &path).unwrap();
        let restored = original.mission_status_freshness(&work.id);
        original.close().unwrap();
        assert!(
            substituted.is_err(),
            "opened datastore was silently mixed with a replacement project path"
        );
        assert!(recovered.unwrap().is_empty());
        assert!(
            missing_path.is_err(),
            "missing Root was reduced to absent artifact"
        );
        assert!(restored.unwrap().is_empty());
        fs::remove_dir_all(parent).unwrap();
    }
}
