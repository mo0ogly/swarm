//! Scope-only activation projection. Metadata and report text establish no rights.
use crate::{AgentLaunchError as E, PlanningEvent, PlanningScope, Work};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

/// Internal complete projection and delivery description. Never supplied by JSON;
/// no caller may substitute a digest for these observed serialized bytes.
pub(crate) struct Observed {
    pub bytes: Vec<u8>,
    pub delivery: crate::PlanningDelivery,
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::{
        EngineSettingsDocument, EngineSettingsValues, PlanningRequest, SettingCap, SettingCount,
    };
    use sha2::{Digest, Sha256};

    /// Candidate/capability metadata is part of the frozen freshness projection;
    /// absolute private paths never enter the scoped contract. No source IO here.
    #[test]
    fn managed_candidate_changes_the_scoped_contract_without_private_paths() {
        let mut f=Fixture::new();
        f.work.planning.as_mut().unwrap().repository=Some(Box::new(crate::ManagedRepository{
            source:"PRIVATE_SOURCE".into(),storage:"PRIVATE_STORAGE".into(),candidate_commit:"a".repeat(40),project_subdir:"subproject".into(),..Default::default()
        }));
        let value=|w:&Work|serde_json::to_value(contract(w,w.planning.as_ref().unwrap().scopes.as_ref().unwrap().iter().find(|s|s.id=="child").unwrap()).unwrap()).unwrap();
        let before=value(&f.work);
        assert_eq!(before["repository"],serde_json::json!({"candidate_commit":"a".repeat(40),"project_subdir":"subproject","operations":["delegate","task"]}));
        assert!(!serde_json::to_string(&before).unwrap().contains("PRIVATE_SOURCE"));
        assert!(!serde_json::to_string(&before).unwrap().contains("PRIVATE_STORAGE"));
        f.work.planning.as_mut().unwrap().repository.as_mut().unwrap().candidate_commit="b".repeat(40);
        assert_ne!(before["repository"],value(&f.work)["repository"]);
    }

    /// Owned projection-only fixture; no SQL, provider, account or native process.
    struct Fixture {
        root: std::path::PathBuf,
        work: Work,
        settings: EngineSettingsDocument,
    }
    impl Fixture {
        /// Split three explicitly owned requirements into root/child/sibling.
        fn new() -> Self {
            use std::os::unix::fs::PermissionsExt;
            let root =
                std::env::temp_dir().join(crate::generate_identifier("scope-context113-").unwrap());
            std::fs::create_dir(&root).unwrap();
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
            let mut values = EngineSettingsValues::installation_defaults().unwrap();
            values.planning_context_max_events = Some(SettingCap(Some(SettingCount(1))));
            let settings = EngineSettingsDocument::initial(
                values,
                "fixture".into(),
                "2026-10-20T12:00:00Z".into(),
            )
            .unwrap();
            let work = Work {
                schema_version: 1,
                id: "work".into(),
                title: "ROOT_PRIVATE_TITLE".into(),
                objective: "ROOT_PRIVATE_OBJECTIVE".into(),
                criteria: Some(vec![
                    "ROOT_PRIVATE_REQUIREMENT".into(),
                    "Child requirement".into(),
                    "SIBLING_PRIVATE_REQUIREMENT".into(),
                ]),
                ..Default::default()
            };
            let mut work = crate::apply_planning_control(
                &work,
                crate::PlanningControlAction::Enable,
                &PlanningRequest {
                    schema_version: 1,
                    event_id: "root-brief".into(),
                    max_tasks: 3,
                    max_activations: 3,
                    max_decisions: 3,
                    ..Default::default()
                },
                "2026-10-20T12:00:00Z",
                &settings,
            )
            .unwrap();
            let p = work.planning.as_mut().unwrap();
            let scopes = p.scopes.as_mut().unwrap();
            scopes[0].requirements = Some(vec!["req-1".into()]);
            for (id, objective, req) in [
                ("child", "Child objective", "req-2"),
                ("sibling", "SIBLING_PRIVATE_OBJECTIVE", "req-3"),
            ] {
                scopes.push(PlanningScope {
                    id: id.into(),
                    parent: "root".into(),
                    objective: objective.into(),
                    requirements: Some(vec![req.into()]),
                    revision: 1,
                    state: "ready".into(),
                    ..Default::default()
                });
                p.inbox.as_mut().unwrap().push(PlanningEvent {
                    id: format!("{id}-brief"),
                    scope: id.into(),
                    kind: "delegation".into(),
                    message: objective.into(),
                    ..Default::default()
                });
            }
            work.tasks = Some(vec![
                crate::Task {
                    id: "child-task".into(),
                    title: "Own title".into(),
                    status: "ready".into(),
                    scope_id: "child".into(),
                    plan_role: "worker".into(),
                    requirements: Some(vec!["req-2".into()]),
                    next: "Own complete instructions".into(),
                    ..Default::default()
                },
                crate::Task {
                    id: "SIBLING_PRIVATE_TASK".into(),
                    title: "SIBLING_PRIVATE_TITLE".into(),
                    status: "ready".into(),
                    scope_id: "sibling".into(),
                    plan_role: "worker".into(),
                    requirements: Some(vec!["req-3".into()]),
                    ..Default::default()
                },
            ]);
            crate::validate_planning_state(&work).unwrap();
            Self {
                root,
                work,
                settings,
            }
        }
        /// Project the child only; callers still own full state/transaction guards.
        fn observe(&self) -> Result<Observed, E> {
            let scope = self
                .work
                .planning
                .as_ref()
                .unwrap()
                .scopes
                .as_ref()
                .unwrap()
                .iter()
                .find(|s| s.id == "child")
                .unwrap();
            observe(&self.root, &self.work, scope, &self.settings)
        }
    }
    impl Drop for Fixture {
        /// Successful checks remove their own directory; failures retain evidence.
        fn drop(&mut self) {
            if !std::thread::panicking() {
                std::fs::remove_dir_all(&self.root).unwrap();
            }
        }
    }

    /// Shared projection detects current owned changes and leaves unrelated
    /// root/sibling data out. It matches the same fields of admitted context.
    #[test]
    fn scope_context_contract_freshness_matches_rendered_owned_fields() {
        let f = Fixture::new();
        let capture = |w: &Work| {
            let scope = w
                .planning
                .as_ref()
                .unwrap()
                .scopes
                .as_ref()
                .unwrap()
                .iter()
                .find(|s| s.id == "child")
                .unwrap();
            serde_json::to_value(contract(w, scope).unwrap()).unwrap()
        };
        let original = capture(&f.work);
        let rendered: serde_json::Value =
            serde_json::from_slice(&f.observe().unwrap().bytes).unwrap();
        for key in ["work", "scope", "requirements", "tasks", "judge"] {
            assert_eq!(original.get(key), rendered.get(key));
        }
        let text = serde_json::to_string(&original).unwrap();
        assert!(!text.contains("ROOT_PRIVATE") && !text.contains("SIBLING_PRIVATE"));
        for key in ["title", "status", "next", "blocker"] {
            let mut w = f.work.clone();
            let t = w
                .tasks
                .as_mut()
                .unwrap()
                .iter_mut()
                .find(|t| t.scope_id == "child")
                .unwrap();
            match key {
                "title" => t.title = "changed".into(),
                "status" => t.status = "blocked".into(),
                "next" => t.next = "changed".into(),
                "blocker" => t.blocker = "changed".into(),
                _ => unreachable!(),
            }
            assert_ne!(capture(&w), original, "{key}");
            assert_eq!(capture(&f.work), original);
        }
        let mut w = f.work.clone();
        w.criteria.as_mut().unwrap()[1] = "Changed own requirement".into();
        assert_ne!(capture(&w), original);
        // Projection-only fixtures exercise each authoritative field; these do
        // not claim admission of a fabricated lifecycle/review record.
        for field in [
            "attempts_used",
            "attempts_max",
            "instruction_revisions_count",
            "review_coordination",
        ] {
            let mut w = f.work.clone();
            let task = w
                .tasks
                .as_mut()
                .unwrap()
                .iter_mut()
                .find(|t| t.scope_id == "child")
                .unwrap();
            match field {
                "attempts_used" => task.attempts = Some(vec![crate::Attempt::default()]),
                "attempts_max" => task.plan_max_attempts += 1,
                "instruction_revisions_count" => {
                    task.instruction_revisions = Some(vec![crate::InstructionRevision::default()])
                }
                "review_coordination" => task.review_coordination = Some(Box::default()),
                _ => unreachable!(),
            }
            assert_ne!(
                capture(&w),
                original,
                "{field} must invalidate the scoped contract"
            );
            assert_eq!(capture(&f.work), original);
        }
        w = f.work.clone();
        w.criteria.as_mut().unwrap()[2] = "Changed sibling requirement".into();
        w.title = "Changed root title".into();
        w.tasks
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|t| t.scope_id == "sibling")
            .unwrap()
            .next = "Changed sibling instruction".into();
        assert_eq!(capture(&w), original);
    }

    /// Root cycle history is visible only to root and included in its freshness
    /// contract. It never becomes a child instruction or a claim of acceptance.
    #[test]
    fn scope_context_binds_root_judge_without_disclosing_it_to_children() {
        let mut f = Fixture::new();
        let at = "2026-10-22T12:00:00Z";
        let j = f.work.planning.as_mut().unwrap().judge.as_mut().unwrap();
        j.verdict="continue".into();j.reason="ROOT_PRIVATE_JUDGE_REASON".into();j.at=at.into();
        j.history=Some(vec![crate::JudgeCycleRecord { cycle:1,verdict:j.verdict.clone(),reason:j.reason.clone(),at:at.into(),..Default::default() }]);
        crate::validate_planning_state(&f.work).unwrap();
        let child: serde_json::Value = serde_json::from_slice(&f.observe().unwrap().bytes).unwrap();
        assert!(child.get("judge").is_none());assert!(!child.to_string().contains("ROOT_PRIVATE_JUDGE_REASON"));
        let root = f.work.planning.as_ref().unwrap().scopes.as_ref().unwrap().iter().find(|s| s.id=="root").unwrap();
        let before = serde_json::to_value(contract(&f.work,root).unwrap()).unwrap();assert_eq!(before["judge"]["reason"],"ROOT_PRIVATE_JUDGE_REASON");
        let value = observe(&f.root,&f.work,root,&f.settings).unwrap();let projected:serde_json::Value=serde_json::from_slice(&value.bytes).unwrap();assert_eq!(projected["judge"],before["judge"]);assert_eq!(projected["judge_end_available"],false);
        let mut changed=f.work.clone();changed.planning.as_mut().unwrap().judge.as_mut().unwrap().max_cycles=2;
        assert_ne!(serde_json::to_value(contract(&changed,root).unwrap()).unwrap(),before);
        assert_eq!(serde_json::to_value(contract(&f.work,root).unwrap()).unwrap(),before);
    }
    /// Child projection excludes sibling/global text and identifiers, prefers the
    /// newest operator instruction and leaves every input event unacknowledged.
    #[test]
    fn scope_context_isolation_latest_instruction_and_undelivered_events_are_preserved() {
        let mut f = Fixture::new();
        let original = f.work.clone();
        let control = f.observe().unwrap();
        let text = String::from_utf8(control.bytes).unwrap();
        assert!(text.contains("Child requirement") && text.contains("Own complete instructions"));
        assert!(!text.contains("ROOT_PRIVATE") && !text.contains("SIBLING_PRIVATE"));
        assert_eq!(f.work, original);
        let inbox = f.work.planning.as_mut().unwrap().inbox.as_mut().unwrap();
        inbox.push(PlanningEvent {
            id: "operator-latest".into(),
            scope: "child".into(),
            kind: "operator_resume".into(),
            message: "Changed precondition".into(),
            ..Default::default()
        });
        inbox.push(PlanningEvent {
            id: "diagnostic".into(),
            scope: "child".into(),
            kind: "planning_rejected".into(),
            message: "Previous proposal".into(),
            ..Default::default()
        });
        let before = f.work.clone();
        let observed = f.observe().unwrap();
        assert_eq!(
            observed.delivery.events,
            Some(vec!["operator-latest".into()])
        );
        let value: serde_json::Value = serde_json::from_slice(&observed.bytes).unwrap();
        assert_eq!(value["pending_events_total"], 3);
        assert_eq!(f.work, before);
        assert!(f
            .work
            .planning
            .as_ref()
            .unwrap()
            .inbox
            .as_ref()
            .unwrap()
            .iter()
            .all(|e| e.decision.is_empty()));
    }

    /// Healthy report readback precedes digest corruption; cross-scope dependency
    /// and specialized-event faults refuse rather than omitting their evidence.
    #[test]
    fn scope_context_report_change_dependency_and_specialized_inputs_refuse_then_recover() {
        let mut f = Fixture::new();
        f.observe().unwrap();
        let report = b"Complete owned report, not acceptance";
        std::fs::write(f.root.join("report.md"), report).unwrap();
        let event = f
            .work
            .planning
            .as_mut()
            .unwrap()
            .inbox
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e.id == "child-brief")
            .unwrap();
        event.kind = "handoff".into();
        event.handoff = Some(Box::new(crate::ExchangeArtifact {
            path: "report.md".into(),
            sha256: format!("{:x}", Sha256::digest(report)),
        }));
        let healthy = f.work.clone();
        assert!(String::from_utf8(f.observe().unwrap().bytes)
            .unwrap()
            .contains("Complete owned report"));
        std::fs::write(f.root.join("report.md"), b"PRIVATE changed bytes").unwrap();
        let error = f.observe().err().unwrap();
        assert_eq!(error.code(), "planning-context-report-invalid");
        assert!(!error.to_string().contains("PRIVATE"));
        std::fs::write(f.root.join("report.md"), report).unwrap();
        f.observe().unwrap();
        f.work.tasks.as_mut().unwrap()[0].depends = Some(vec!["SIBLING_PRIVATE_TASK".into()]);
        assert_eq!(
            f.observe().err().unwrap().code(),
            "planning-task-dependency-outside-scope"
        );
        f.work = healthy.clone();
        f.observe().unwrap();
        f.work
            .planning
            .as_mut()
            .unwrap()
            .inbox
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e.id == "child-brief")
            .unwrap()
            .kind = "independent_review".into();
        assert_eq!(
            f.observe().err().unwrap().code(),
            "planning-specialized-event-context-unported"
        );
        f.work = healthy;
        f.observe().unwrap();
    }
}

/// Stable scope header excludes mutable lease, previous delivery and workflow.
/// A claim can therefore bind the projected content without recursive receipts.
#[derive(Serialize)]
struct Scope<'a> {
    id: &'a str,
    parent: &'a str,
    objective: &'a str,
    revision: i64,
}

/// Own-task contracts only, without provider argv, root brief or sibling IDs.
/// Fields are complete values: no implicit display truncation authorizes action.
#[derive(Serialize)]
struct Task<'a> {
    id: &'a str,
    title: &'a str,
    status: &'a str,
    requirements: &'a Option<Vec<String>>,
    dependencies: &'a Option<Vec<String>>,
    next: &'a str,
    blocker: &'a str,
    attempts_used: usize,
    attempts_max: i64,
    instruction_revisions_count: usize,
    review_coordination: &'a Option<Box<crate::ReviewCoordinationRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    independent_review: &'a Option<Box<crate::IndependentReview>>,
}

/// Scoped managed metadata; operation labels describe implemented protocol only.
/// No source paths, content, exploration tools or worker launch grant is exposed.
#[derive(Serialize)]
pub(crate) struct Repository<'a> {
    candidate_commit: &'a str,
    project_subdir: &'a str,
    operations: [&'static str;2],
}

/// Canonical scoped context omits unrelated scope details and the full Work.
#[derive(Serialize)]
struct Context<'a> {
    work: &'a str,
    #[serde(skip_serializing_if="Option::is_none")]
    repository: Option<Repository<'a>>,
    scope: Scope<'a>,
    requirements: BTreeMap<String, String>,
    events: Vec<&'a PlanningEvent>,
    historical_only: bool,
    pending_events_total: usize,
    tasks: Vec<Task<'a>>,
    /// Root-only complete cycle data under the same context byte cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    judge: Option<&'a crate::JudgeState>,
    /// Structural hints only; end still needs unported managed evidence.
    judge_available: bool,
    renew_available: bool,
    judge_end_available: bool,
    reports: Vec<crate::PlanningReportContent>,
    #[cfg(target_os = "linux")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    worker_completions: Vec<crate::root_store::WorkerCompletionInput>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    reviews: Vec<crate::IndependentReview>,
    /// Safe attributed current reviewer incident, not a quality verdict.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    review_failure_routing: BTreeMap<String, crate::ReviewFailureRouting>,
    /// Historical wire name: SHA of the protocol/interruption private original
    /// body, not its independently checked SR1
    /// frame; no raw capture or provider text is exposed.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    review_protocol_originals: BTreeMap<String, String>,
    /// Already received current incidents; these never become pending input IDs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unresolved_review_events: Vec<&'a PlanningEvent>,
    /// Structural availability from the exact same producer as delivery; no grant.
    review_retry_unavailable: bool,
    /// Failed calls provide an incident, not an applicable report or verdict.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    unavailable_reports: BTreeMap<String, String>,
}

/// Current scoped context contract, excluding pending-batch selection. Used by
/// initial context observation and later freshness verification so both compare
/// identical rendered task/requirement fields and managed candidate metadata.
/// This is not every Task field;
/// Root additionally carries its complete judge contract; child projections
/// contain none of that history. No provider, policy or ownership grant.
#[derive(Serialize)]
pub(crate) struct ScopeContract<'a> {
    work: &'a str,
    #[serde(skip_serializing_if="Option::is_none")]
    repository: Option<Repository<'a>>,
    scope: Scope<'a>,
    requirements: BTreeMap<String, String>,
    tasks: Vec<Task<'a>>,
    /// Full root-only cycle contract; child contexts expose no root history.
    #[serde(skip_serializing_if = "Option::is_none")]
    judge: Option<&'a crate::JudgeState>,
}
/// Read current requirement texts and owned task projection, including actual
/// attempt/revision counts, the saved attempt policy and review coordination.
/// Counts do not disclose attempt prompts or revision text, and coordination is
/// data only, never review acceptance. Missing historical projection fields are
/// not backfilled: later freshness comparison requires a new claim. Validates
/// referenced requirement positions and dependency containment. Serialization
/// errors belong to the caller. No IO, mutation, cap/truncation or event selection
/// occurs; equal projections establish only the included contract's freshness.
pub(crate) fn contract<'a>(
    work: &'a Work,
    scope: &'a PlanningScope,
) -> Result<ScopeContract<'a>, E> {
    let tasks = work.tasks.as_deref().unwrap_or_default();
    let mut requirements = BTreeMap::new();
    for id in scope.requirements.as_deref().unwrap_or_default() {
        let index = id
            .strip_prefix("req-")
            .and_then(|n| n.parse::<usize>().ok())
            .and_then(|n| n.checked_sub(1))
            .ok_or(E::Refused("planning-requirement-invalid"))?;
        let text = work
            .criteria
            .as_deref()
            .unwrap_or_default()
            .get(index)
            .ok_or(E::Refused("planning-requirement-missing"))?;
        requirements.insert(id.clone(), text.clone());
    }
    for task in tasks.iter().filter(|t| t.scope_id == scope.id) {
        if task
            .depends
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|id| {
                !tasks
                    .iter()
                    .any(|other| other.id == *id && other.scope_id == scope.id)
            })
        {
            return Err(E::Refused("planning-task-dependency-outside-scope"));
        }
    }
    Ok(ScopeContract {
        repository:work.planning.as_deref().and_then(|p|p.repository.as_deref()).map(|r|Repository{candidate_commit:&r.candidate_commit,project_subdir:&r.project_subdir,operations:["delegate","task"]}),
        judge: if scope.id == "root" { work.planning.as_deref().and_then(|p| p.judge.as_deref()) } else { None },
        work: &work.id,
        scope: Scope {
            id: &scope.id,
            parent: &scope.parent,
            objective: &scope.objective,
            revision: scope.revision,
        },
        requirements,
        tasks: tasks
            .iter()
            .filter(|t| t.scope_id == scope.id)
            .map(|t| Task {
                id: &t.id,
                title: &t.title,
                status: &t.status,
                requirements: &t.requirements,
                dependencies: &t.depends,
                next: &t.next,
                blocker: &t.blocker,
                attempts_used: t.attempts.as_deref().unwrap_or_default().len(),
                attempts_max: t.plan_max_attempts,
                instruction_revisions_count: t
                    .instruction_revisions
                    .as_deref()
                    .unwrap_or_default()
                    .len(),
                review_coordination: &t.review_coordination,
                independent_review: &t.independent_review,
            })
            .collect(),
    })
}

/// Recognize only a native certificate reference in the owning task's current
/// or archived review. This is classification, not validation; selected events
/// still require the actual ledger/private/event/certificate proof below.
fn native_review_reference(work: &Work, event: &PlanningEvent) -> bool {
    work.tasks
        .iter()
        .flatten()
        .find(|t| t.id == event.task)
        .is_some_and(|t| {
            t.independent_review
                .iter()
                .map(|v| v.as_ref())
                .chain(t.previous_reviews.iter().flatten())
                .any(|r| {
                    if r.id != event.id {
                        return false;
                    }
                    #[cfg(target_os = "linux")]
                    {
                        crate::root_store::RootStore::native_certificate_reference(r)
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        false
                    }
                })
        })
}
/// Observe complete scoped inputs under persisted event/context/report caps.
/// Current inputs precede historical attempts; the latest operator resumption
/// is delivered first so an older incident cannot hide a changed precondition.
/// Events beyond the configured batch remain pending, never acknowledged here.
/// Specialized exploration/run events and managed/legacy review coordination refuse.
/// Selected native ordinary review events require original publication proofs
/// and complete report text for verdicts. Verified failed calls explicitly mark
/// their report inapplicable instead; none becomes generic unverified narrative.
/// Still-current acknowledged native errors accompany fresh inputs separately;
/// they are never consumed again and count against the same saved context caps. Paths/digests,
/// ownership, encoding and limits preserve safe typed causes. No mutation,
/// acceptance, receipt delivery, semantic redaction or model capacity is proved.
pub(crate) fn observe(
    root: &Path,
    work: &Work,
    scope: &PlanningScope,
    settings: &crate::EngineSettingsDocument,
) -> Result<Observed, E> {
    let policy = settings
        .values
        .planning_action_policy()
        .map_err(|e| E::caused(e.reason(), e))?;
    let planning = work
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    let tasks = work.tasks.as_deref().unwrap_or_default();
    let mut current = Vec::new();
    let mut historical = Vec::new();
    for event in planning.inbox.as_deref().unwrap_or_default() {
        if event.scope != scope.id || !event.decision.is_empty() {
            continue;
        }
        if event.exploration.is_some()
            || event.run_observation.is_some()
            || (event.kind == "independent_review" && !native_review_reference(work, event))
        {
            return Err(E::Refused("planning-specialized-event-context-unported"));
        }
        let stale = if event.task.is_empty() {
            false
        } else {
            match tasks.iter().find(|t| t.id == event.task) {
                None => true,
                Some(task) => {
                    if task.scope_id != scope.id {
                        return Err(E::Refused("planning-event-task-owner-mismatch"));
                    }
                    !event.attempt.is_empty()
                        && task
                            .attempts
                            .as_deref()
                            .unwrap_or_default()
                            .last()
                            .is_none_or(|a| a.id != event.attempt)
                }
            }
        };
        if stale {
            historical.push(event);
        } else {
            current.push(event);
        }
    }
    let pending = current
        .len()
        .checked_add(historical.len())
        .ok_or(E::Refused("planning-context-count-representation"))?;
    let historical_only = current.is_empty() && !historical.is_empty();
    let mut selected = if historical_only { historical } else { current };
    if let Some(i) = selected.iter().rposition(|e| e.kind == "operator_resume") {
        let latest = selected.remove(i);
        selected.insert(0, latest);
    }
    if let Some(cap) = policy.context_max_events {
        selected.truncate(cap);
    }
    if selected.is_empty() {
        return Err(E::Refused(if pending == 0 {
            "planning-no-pending-events"
        } else {
            "planning-event-batch-disabled"
        }));
    }
    let contract = contract(work, scope)?;
    let report_limit = settings
        .values
        .planning_report_limit()
        .map_err(|e| E::caused(e.reason(), e))?;
    let mut reports = Vec::new();
    #[cfg(target_os = "linux")]
    let mut worker_completions = Vec::new();
    let mut references = Vec::new();
    let mut reviews = Vec::new();
    let mut unavailable_reports = BTreeMap::new();
    let mut review_failure_routing = BTreeMap::new();
    let mut review_protocol_originals = BTreeMap::new();
    // Acknowledgement records reception, not resolution. Include only the
    // exact current native error on a fresh activation, under the same caps.
    let unresolved_review_events: Vec<_> = planning.inbox.iter().flatten().filter(|e|
        !historical_only && e.scope == scope.id && !e.decision.is_empty()
        && e.kind == "independent_review" && native_review_reference(work, e)
        && tasks.iter().find(|t| t.id == e.task).is_some_and(|t|
            t.independent_review.as_deref().is_some_and(|r| r.id == e.id && r.state == "error")
            && t.attempts.iter().flatten().last().is_some_and(|a| a.id == e.attempt && a.status == "completed"))
    ).collect();
    if policy.context_max_events.is_some_and(|cap| selected.len().checked_add(unresolved_review_events.len()).is_none_or(|n| n > cap)) {
        return Err(E::Refused("planning-context-event-limit-exceeded"));
    }
    for event in selected.iter().chain(unresolved_review_events.iter()) {
        if event.kind == "attempt_ended" && tasks.iter().find(|t| t.id == event.task)
            .is_some_and(|t| t.attempts.iter().flatten().last().is_some_and(|a| a.id == event.attempt)) {
            #[cfg(target_os = "linux")]
            worker_completions.push(crate::RootStore::worker_completion_for_context(root, work, event)
                .map_err(|e| E::caused("planning-worker-completion-evidence", e))?);
            #[cfg(not(target_os = "linux"))]
            return Err(E::Refused("planning-worker-completion-platform-unported"));
        }
        if event.kind == "independent_review" {
            #[cfg(not(target_os = "linux"))]
            return Err(E::Refused("planning-specialized-event-context-unported"));
            #[cfg(target_os = "linux")]
            {
                let observed =
                    crate::root_store::RootStore::native_review_for_context(root, work, event)
                        .map_err(|e| E::caused("planning-native-review-evidence", e))?;
                // Only current verdict data can support its purpose-specific action.
                // Archives/non-verdicts remain attributed historical observations;
                // active decisions have a separate fresh authoritative fence.
                let current_verdict = observed.verdict_applicable
                    && matches!(observed.review.state.as_str(), "passed" | "changes_requested")
                    && work.tasks.iter().flatten().find(|t| t.id == event.task)
                        .is_some_and(|t| t.independent_review.as_deref()
                            .is_some_and(|r| r.id == observed.review.id)
                            && t.attempts.iter().flatten().last()
                                .is_some_and(|a| a.id == observed.review.attempt));
                if current_verdict {
                    let evidence = if observed.review.state == "changes_requested" {
                        crate::review_evidence::check_current_native_change_request(root, work, &event.task, settings)
                    } else { crate::check_current_review_evidence(root, work, &event.task, settings) };
                    let evidence = evidence.map_err(|e| E::caused("planning-native-review-current-evidence", e))?
                        .ok_or(E::Refused("planning-native-review-current-required"))?;
                    if evidence.review_id != event.id || evidence.attempt_id != event.attempt {
                        return Err(E::Refused("planning-native-review-current-changed"));
                    }
                }
                let current_retry = observed.retry_failure.is_some() && tasks.iter().find(|t| t.id == event.task)
                    .is_some_and(|t| t.independent_review.as_deref().is_some_and(|r| r == &observed.review)
                        && t.attempts.iter().flatten().last().is_some_and(|a| a.id == observed.review.attempt && a.status == "completed"));
                if current_retry {
                    let evidence = crate::review_evidence::check_current_native_reviewer_failure(root, work, &event.task, settings)
                        .map_err(|e| E::caused("planning-native-review-protocol-current", e))?
                        .ok_or(E::Refused("planning-native-review-protocol-required"))?;
                    if evidence.review_id != event.id || evidence.attempt_id != event.attempt { return Err(E::Refused("planning-native-review-protocol-changed")); }
                    review_protocol_originals.insert(event.task.clone(), observed.original_receipt_sha256.clone());
                    review_failure_routing.insert(event.task.clone(), crate::ReviewFailureRouting {
                        result_event: event.id.clone(), review_id: observed.review.id.clone(), attempt: observed.review.attempt.clone(),
                        candidate_commit: String::new(), failure_code: observed.retry_failure.ok_or(E::Refused("planning-review-failure-required"))?.code().into(), owner: observed.retry_failure.ok_or(E::Refused("planning-review-failure-required"))?.owner().into(),
                        operation: "review-retry".into(), producer_correction_required: false,
                    });
                }
                if !observed.verdict_applicable && !current_retry {
                    // This is an original failed/stale non-verdict, not a review of
                    // the current file. Do not invent or silently omit a report.
                    unavailable_reports.insert(
                        event.id.clone(),
                        if observed.review.state == "stale" {
                            "native-review-stale-no-applicable-verdict"
                        } else {
                            "native-review-failed-no-applicable-verdict"
                        }
                        .into(),
                    );
                    references.push(observed.certificate);
                    reviews.push(observed.review);
                    continue;
                }
                let reference = crate::ExchangeArtifact {
                    path: observed.review.report.clone(),
                    sha256: observed.review.sha256.clone(),
                };
                if current_retry && event.artifacts.is_some() || !current_retry && event.artifacts.as_deref() != Some(std::slice::from_ref(&reference)) {
                    return Err(E::Refused("planning-native-review-report-binding"));
                }
                let report =
                    crate::planning_report::read_reference(root, event, &reference, report_limit)
                        .map_err(|e| E::caused("planning-native-review-report", e))?;
                references.push(reference);
                references.push(observed.certificate);
                reports.push(report);
                reviews.push(observed.review);
                continue;
            }
        }
        if let Some(report) = crate::read_planning_report(root, event, report_limit)
            .map_err(|e| E::caused("planning-context-report-invalid", e))?
        {
            references.push(crate::ExchangeArtifact {
                path: report.path.clone(),
                sha256: report.sha256.clone(),
            });
            reports.push(report);
        }
        // Other artifact kinds need their own authoritative validator before a
        // decision can consume them. Do not silently omit unavailable evidence.
        if event
            .artifacts
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|a| {
                !references
                    .iter()
                    .any(|r| r.path == a.path && r.sha256 == a.sha256)
            })
        {
            return Err(E::Refused("planning-context-artifact-kind-unported"));
        }
    }
    let events = selected.iter().map(|e| e.id.clone()).collect();
    let review_retry_unavailable = !settings.values.has_planning_decision_policy()
                || settings.values.planning_decision_policy().map_err(|e| E::caused(e.reason(), e))?.max_operations == Some(0)
                || review_failure_routing.is_empty()
                || planning.reviewer.as_deref().is_none_or(|r| r.calls < 0 || r.calls >= r.max_calls)
                || !tasks.iter().any(|t| review_failure_routing.contains_key(t.id.as_str()) && !t.launch_held && t.status == "submitted" && !t.planning_retry);
    let decision_available = settings.values.has_planning_decision_policy()
        && settings.values.planning_decision_policy().map_err(|e| E::caused(e.reason(), e))?.max_operations != Some(0);
    let (judge_available, renew_available) = if decision_available { crate::planning_judge::availability(work, &scope.id) } else { (false, false) };
    let context = Context {
        work: &work.id,
        repository: contract.repository,
        scope: contract.scope,
        requirements: contract.requirements,
        events: selected,
        historical_only,
        pending_events_total: pending,
        tasks: contract.tasks,
        judge: contract.judge,
        judge_available,
        renew_available,
        judge_end_available: false,
        reports,
        #[cfg(target_os = "linux")]
        worker_completions,
        reviews,
        review_failure_routing: review_failure_routing.clone(),
        review_protocol_originals,
        unresolved_review_events,
        review_retry_unavailable,
        unavailable_reports: unavailable_reports.clone(),
    };
    let bytes =
        crate::encode_model(&context).map_err(|e| E::caused("planning-context-encoding", e))?;
    if policy
        .context_max_bytes
        .is_some_and(|max| bytes.len() > max)
    {
        return Err(E::Refused("planning-context-byte-limit-exceeded"));
    }
    use sha2::{Digest, Sha256};
    Ok(Observed {
        delivery: crate::PlanningDelivery {
            judge_available,
            renew_available,
            // End cannot be inferred from scope closure or a supplied reference.
            judge_end_available: false,
            judge_cycle: if scope.id == "root" { planning.judge.as_deref().map_or(0, |j| j.cycle) } else { 0 },
            context_sha256: format!("{:x}", Sha256::digest(&bytes)),
            events: Some(events),
            reports: Some(references),
            historical_only,
            unavailable_reports: (!unavailable_reports.is_empty()).then_some(unavailable_reports),
            task_budget: planning.max_tasks,
            activation_budget: planning.max_activations,
            // Capability metadata requires adopted decision policy and available
            // operation count. Runtime ownership/budgets are rechecked at decide;
            // this snapshot is neither authenticated authority nor a reservation.
            delegation_unavailable: if settings.values.has_planning_decision_policy() {
                settings
                    .values
                    .planning_decision_policy()
                    .map_err(|e| E::caused(e.reason(), e))?
                    .max_operations
                    == Some(0)
            } else {
                true
            },
            // Pure structural eligibility stays stable during this model call.
            // Original freshness and physical ownership are checked at decision.
            closure_unavailable: !settings.values.has_planning_decision_policy()
                || settings.values.planning_decision_policy().map_err(|e| E::caused(e.reason(), e))?.max_operations == Some(0)
                || match crate::planning_decision::closure_structure(work, &scope.id) {
                    Ok(()) => false,
                    Err(E::Refused("planning-closure-child-open" | "planning-closure-task-not-accepted" | "planning-closure-requirement-uncovered")) => true,
                    Err(error) => return Err(error),
                },
            unresolved_review_sources: (!review_failure_routing.is_empty()).then_some(review_failure_routing),
            review_retry_unavailable,
            // Structural metadata only; current original and canonical decision
            // are verified separately, and this flag never starts a worker.
            production_retry_unavailable: !settings.values.has_planning_decision_policy()
                || settings.values.planning_decision_policy().map_err(|e| E::caused(e.reason(), e))?.max_operations == Some(0)
                || !tasks.iter().any(|t| t.scope_id == scope.id && t.status == "blocked" && !t.launch_held && !t.planning_retry
                    && t.plan_max_attempts > 0 && (t.attempts.as_ref().map_or(0, Vec::len) as u128) < t.plan_max_attempts as u128
                    && (t.independent_review.as_deref().is_some_and(|r| r.state == "changes_requested"
                        && crate::review_evidence::native_receipt_reference(&r.receipt)
                        && t.attempts.iter().flatten().last().is_some_and(|a| a.id == r.attempt))
                        || t.independent_review.is_none() && t.attempts.iter().flatten().last().is_some_and(|a| matches!(a.status.as_str(), "failed" | "interrupted")))) ,
            ..Default::default()
        },
        bytes,
    })
}
