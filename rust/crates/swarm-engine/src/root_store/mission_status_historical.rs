//! Historical status for supported ordinary Work and initial planning trees. Branches lacking real
//! producers refuse explicitly. Go reference: mission_status.go, organization.go,
//! mission_supervision.go and mission_insights.go in HEAD. No writes or grants.
//! Review axes: presentation strings/reduction identities implement Go branches;
//! explicit saved display windows preserve full accounting. Causes survive guards.
use super::{finished, RootStore, RootStoreError as E};
use crate::mission_status_contract::*;
use crate::mission_status_insights as insight;
use crate::mission_status_tasks::*;
use std::time::Duration;

/// Public projection and original private causes, deliberately without Debug or
/// Serialize. Root close and output/flush handling remain the caller's duty.
pub struct MissionStatusObservation {
    pub status: MissionStatus,
    pub causes: Vec<E>,
}
/// Preserve the actual mission reader cause through the existing mutation wrapper;
/// this wrapper does not assert that any mutation happened.
fn mission(cause: crate::MissionSupervisionError) -> E {
    E::Mutation(crate::WorkEngineError::Mission(Box::new(cause)))
}
/// Keep known sidecar failures through later guard/cleanup refusal.
fn retain(mut error: E, causes: Vec<E>) -> E {
    for cause in causes {
        error = E::Cleanup {
            primary: Some(Box::new(error)),
            cleanup: Box::new(cause),
        };
    }
    error
}
/// Go zero strings are intermediate reduction state, filled before publication.
fn understanding() -> MissionUnderstanding {
    MissionUnderstanding {
        what: String::new(),
        next_step: String::new(),
        actor: String::new(),
        actor_kind: String::new(),
        situation: String::new(),
    }
}
/// Overlay observed active provider waits on a structurally ready, unfinished
/// status, as Go missionStatus. Caller supplies the strict reader's sorted active
/// records; this pure projection never clears quotas or changes launch authority.
/// Invalid Unix dates retain their typed time cause rather than inventing a date.
/// Runtime blockers must be applied by the caller afterwards. Task attempt-limit
/// explanations use saved positive limits, never a new retry policy.
fn apply_provider_waits(work: &crate::Work, status: &mut MissionStatus) -> Result<(), E> {
    if !status.organization.ready || status.understanding.situation == "termine" {
        return Ok(());
    }
    let Some(cooldowns) = status.provider_cooldowns.as_deref().filter(|v| !v.is_empty()) else {
        return Ok(());
    };
    let mut messages = Vec::new();
    let mut unknown = false;
    for cooldown in cooldowns {
        let provider = if cooldown.provider.is_empty() { "IA" } else { &cooldown.provider };
        let prefix = format!("Fournisseur {provider} : quota refusé (429). ");
        let message = if cooldown.resets_at > 0 {
            let at = time::OffsetDateTime::from_unix_timestamp(cooldown.resets_at)
                .map_err(|cause| mission(crate::MissionSupervisionError::Cause {
                    code: "mission-status-cooldown-date-invalid", cause: Box::new(cause),
                }))?;
            format!("{prefix}Nouveaux appels suspendus jusqu’au {:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC. Les tentatives et budgets déjà consommés restent conservés.", at.year(), u8::from(at.month()), at.day(), at.hour(), at.minute(), at.second())
        } else {
            unknown = true;
            format!("{prefix}Aucune heure de reprise fournie. Vérifier le compte puis lever explicitement cette attente ; aucune relance automatique.")
        };
        messages.push(message);
    }
    let (mut next, mut actor, mut kind, mut situation) = if unknown {
        ("Vérifier la disponibilité du compte puis lever explicitement l’attente fournisseur sans heure de reprise.".to_string(), "Vous", "user", "decision_humaine")
    } else {
        ("Attendre l’échéance ; les plafonds de tentatives et les autres blocages restent applicables.".to_string(), "Le fournisseur IA", "none", "attente_normale")
    };
    if status.paused {
        next.push_str(" La mission reste en pause ; aucun redémarrage automatique n’est autorisé.");
    }
    let exhausted: Vec<_> = work.tasks.as_deref().unwrap_or(&[]).iter()
        .filter(|task| task.status == "blocked" && task.plan_max_attempts > 0
            && task.attempts.as_deref().unwrap_or(&[]).len() as u128 >= task.plan_max_attempts as u128)
        .map(|task| task.title.as_str()).collect();
    if !exhausted.is_empty() {
        messages.push(format!("Plafond de tentatives atteint : {}. La fin du quota ne débloquera pas ces tâches.", exhausted.join(", ")));
        next.push_str(" Examiner les tâches ayant épuisé leurs tentatives ; aucun nouveau départ n’est autorisé pour elles.");
        actor = "Vous"; kind = "user"; situation = "decision_humaine";
    }
    status.next = next.clone();
    status.understanding = MissionUnderstanding { what: messages.join("\n"), next_step: next,
        actor: actor.into(), actor_kind: kind.into(), situation: situation.into() };
    Ok(())
}
/// Describe structurally validated initial planning metadata, or the ordinary
/// missing-organization explanation. Caller must run supported() on the same
/// Work before use; missing reviewer still refuses defensively. No IO, capacity
/// probe, independent review, permission or acceptance follows from this text.
fn organization(work: &crate::Work) -> Result<Organization, E> {
    let Some(planning) = work.planning.as_deref() else {
        return Ok(Organization {ready:false,label:"Organisation autonome non configurée".into(),issues:Some(vec!["Responsable de mission absent : planification hiérarchique non configurée.".into()]),next:"Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement.".into(),verification:"Politique incomplète ; aucune vérification de mission démontrée.".into()});
    };
    let reviewer = planning.reviewer.as_deref()
        .ok_or(E::Refused("mission-status-reviewer-missing"))?;
    let verification = if planning.human_review_authorized.is_empty() {
        "Contrôles configurés ; aucune revue IA indépendante implicite."
    } else {
        "Revue humaine explicitement autorisée ; les résultats attendront votre acceptation."
    };
    Ok(Organization {
        ready: true,
        label: "Organisation autonome configurée".into(),
        issues: Some(Vec::new()),
        next: "Consulter les responsables et les contrôles autorisés.".into(),
        verification: format!("Vérificateur IA indépendant : {} · {}/{} appels. Avis requis avant acceptation. {}",
            reviewer.provider, reviewer.calls, reviewer.max_calls, verification),
    })
}

/// Read actual SQL rows fully; no ignored decoder/finalize errors or row cutoff.
fn rows<T>(
    db: &rusqlite::Connection,
    sql: &str,
    work: Option<&str>,
    mut decode: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, E> {
    let mut statement = db.prepare(sql).map_err(E::Sql)?;
    let result = (|| {
        let mut query = if let Some(work) = work {
            statement.query([work])
        } else {
            statement.query([])
        }
        .map_err(E::Sql)?;
        let mut out = Vec::new();
        while let Some(row) = query.next().map_err(E::Sql)? {
            out.push(decode(row).map_err(E::Sql)?);
        }
        Ok(out)
    })();
    finished(result, statement.finalize().map_err(E::Sql))
}
/// Complete dispatch context read from its real producers. Missing profiles stop
/// dispatch before these values are consumed, but malformed storage still refuses.
#[derive(PartialEq)]
struct DispatchContext {
    cost: crate::CostSummary,
    budget: Option<crate::WorkBudgetState>,
    occupied: Vec<String>,
    priorities: Vec<(String, i64)>,
}
/// Read global active workspace paths and saved priorities without silent skips.
fn dispatch_context(store: &RootStore, work: &str) -> Result<DispatchContext, E> {
    Ok(DispatchContext {
        cost:store.reported_costs(work)?,budget:store.work_budget_state(work)?,
        occupied:rows(&store.db,"SELECT cwd FROM agents WHERE status IN ('queued','starting','running','stopping') ORDER BY rowid",None,|r|r.get::<_,String>(0))?,
        priorities:rows(&store.db,"SELECT task_id,priority FROM cockpit_tasks WHERE work_id=?1 ORDER BY task_id",Some(work),|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?)))?,
    })
}
/// Observe only ordinary todo readiness branches that terminate before dispatch.
/// Actual dependencies precede readiness. Launch-held and the persisted reported
/// cost guard precede missing profile, pause and manual/assisted mode, as Go.
/// Missing controls remain absence of an automatic mode, not invented saved
/// values. A configured profile is recorded data, never verified provider health.
/// Initial unstarted automatic tasks use saved slots, full Agent observations,
/// lexical workspace overlap and budget/brief assistance. Initial managed tasks
/// use prospective copy paths only; worker history requires separate attribution checks. No copy, probe, SQL
/// write, default, reservation or grant.
/// Typed cost/SQL/policy failures propagate; the exact threshold refusal retains
/// its cause beside the visible intervention. Caller owes final coherence checks.
fn readiness(
    store: &RootStore, work: &crate::Work, task: &crate::Task,
    controls: &super::MissionControlsObservation, settings: &crate::EngineSettingsDocument,
    dependencies: &[DependencyObservation], causes: &mut Vec<E>,
) -> Result<ReadinessObservation, E> {
    if dependencies.iter().any(|d| matches!(d.freshness, DependencyFreshness::NotCurrent)) {
        return Ok(ReadinessObservation::NotObserved);
    }
    let profile = if task.launch_profile.is_some() || work.launch_profile.is_some() {
        ProfileObservation::Configured
    } else { ProfileObservation::Missing };
    let observed = |disposition, reason: &str| ReadinessObservation::Observed {
        disposition, reason: reason.into(), profile,
        assistance: AssistObservation::NotObserved, workspace: WorkspaceObservation::NotObserved,
    };
    if task.launch_held {
        return Ok(observed(DispatchDisposition::Intervention,"Démarrage du plan à autoriser depuis la préparation"));
    }
    match super::mission_cost_guard::admit(&store.db,&work.id,&task.id,settings) {
        Ok(()) => {},
        Err(error @ crate::MissionSupervisionError::Refused("mission-task-reported-cost-threshold")) => {
            causes.push(mission(error));
            return Ok(observed(DispatchDisposition::Intervention,"Seuil de coût de la tâche dépassé : examiner les dépenses avant un nouveau départ"));
        }
        Err(error) => return Err(mission(error)),
    }
    if matches!(profile,ProfileObservation::Missing) {
        return Ok(observed(DispatchDisposition::Intervention,"Aucun profil de lancement : configurer Poursuivre la mission ou le profil de cette tâche"));
    }
    if controls.controls.as_ref().is_some_and(|c| c.paused) {
        return Ok(observed(DispatchDisposition::Waiting,"Mission en pause : reprendre la mission pour autoriser les départs"));
    }
    if controls.controls.as_ref().is_some_and(|c| c.autonomy == super::MissionAutonomy::Automatic) {
        let context = dispatch_context(store, &work.id)?;
        let agents = store.list_agents(&work.id)?;
        let slots = settings.values.mission_dispatch_policy().map_err(E::Settings)?
            .slots(controls.controls.as_ref().map(|c| c.slots)).map_err(E::Settings)?.0;
        return super::mission_status_readiness::observe(super::mission_status_readiness::AutomaticInputs {
            work, task, slots, budget:context.budget.as_ref(),occupied:&context.occupied,
            agents:&agents, root:&store.root,
        });
    }
    Ok(observed(DispatchDisposition::Manual,"Départs automatiques désactivés ; poursuivre la mission ou lancer explicitement"))
}

/// Render the existing Go relative-time vocabulary with saved clock tolerance.
fn relative(at: time::OffsetDateTime, raw: &str, tolerance: i64) -> String {
    let Ok(stamp) = crate::mission_supervision::parse(raw) else {
        return "date illisible".into();
    };
    let age = (at.unix_timestamp_nanos() - stamp.unix_timestamp_nanos())
        .clamp(i128::from(i64::MIN), i128::from(i64::MAX));
    let minute = 60_000_000_000i128;
    let hour = 60 * minute;
    if age < -i128::from(tolerance) * 1_000_000 {
        "horloge incohérente".into()
    } else if age < minute {
        "à l’instant".into()
    } else if age < hour {
        format!("il y a environ {} min", (age + minute / 2) / minute)
    } else {
        format!("il y a environ {} h", (age + hour / 2) / hour)
    }
}
/// Format the validated effective deadline using the lease's persisted cadence.
/// Milliseconds convert to nanoseconds exactly; no timing policy or default is
/// created. Parse failures retain their original cause, and a nonpositive cadence
/// refuses rather than inventing a deadline. Only presentation strings are built.
fn next_relative(at: time::OffsetDateTime, next: &str, poll_ms: i64) -> Result<String, E> {
    if poll_ms < 1 {
        return Err(E::Refused("mission-status-supervision-cadence-invalid"));
    }
    let next = crate::mission_supervision::parse(next).map_err(mission)?;
    let wait = next.unix_timestamp_nanos() - at.unix_timestamp_nanos();
    let second = 1_000_000_000i128;
    if wait > 0 {
        if wait <= i128::from(poll_ms) * 1_000_000 {
            return Ok(if poll_ms % 1000 == 0 {
                format!("dans moins de {} s", poll_ms / 1000)
            } else {
                format!("dans moins de {poll_ms} ms")
            });
        }
        return Ok(format!("dans environ {} s", (wait + second / 2) / second));
    }
    let late = -wait;
    Ok(if late < second {
        "attendue maintenant".into()
    } else {
        format!("en retard d’environ {} s", (late + second / 2) / second)
    })
}
/// Project validated native lease classification and its final durable action.
/// Present/error retain effective deadlines; stale/future/stopped show absent,
/// without inferring process liveness or identity. Legacy-live policy remains
/// explicitly unsupported; stopped legacy history is retained as before. Uses
/// the reader's frozen persisted policy; no mutation, renewal or new grant occurs.
/// Typed read/clock/SQL causes survive, and missing deadline facts refuse.
fn supervision(
    store: &RootStore,
    work: &str,
    now: &str,
    tolerance: i64,
) -> Result<MissionSupervision, E> {
    let lease = store.read_mission_supervision(work, now).map_err(mission)?;
    if lease.as_ref().is_some_and(|lease| lease.stopped_at.is_empty()
        && lease.state == crate::MissionSupervisionState::LegacyUnverified) {
        return Err(E::Refused("mission-status-live-supervision-unported"));
    }
    let at = crate::mission_supervision::parse(now).map_err(mission)?;
    let mut out = MissionSupervision {
        state: "absent".into(),
        source: String::new(),
        observed_at: String::new(),
        last_check_at: String::new(),
        last_check_relative: String::new(),
        reconciled_at: String::new(),
        last_error: String::new(),
        next_check_at: String::new(),
        next_check_relative: String::new(),
        late: false,
        clock_issue: String::new(),
        last_action: None,
    };
    if let Some(lease) = lease {
        out.source = lease.source;
        out.observed_at = lease.observed_at;
        out.last_check_at = lease.last_check_at;
        out.reconciled_at = lease.reconciled_at;
        out.last_error = lease.last_error;
        if !out.last_check_at.is_empty() {
            out.last_check_relative = relative(at, &out.last_check_at, tolerance);
        }
        match lease.state {
            crate::MissionSupervisionState::Present | crate::MissionSupervisionState::CycleError => {
                out.state = if lease.state == crate::MissionSupervisionState::CycleError {
                    "error"
                } else {
                    "active"
                }.into();
                if let Some(next) = lease.next_check_at {
                    let policy = lease.policy.as_ref()
                        .ok_or(E::Refused("mission-status-supervision-policy-missing"))?;
                    out.late = lease.late
                        .ok_or(E::Refused("mission-status-supervision-deadline-unobserved"))?;
                    out.next_check_relative = next_relative(at, &next, policy.poll_interval_ms.0)?;
                    out.next_check_at = next;
                }
            }
            crate::MissionSupervisionState::Future => {
                out.clock_issue = "Horloge modifiée : la présence du conducteur doit être confirmée par une nouvelle vérification.".into();
            }
            crate::MissionSupervisionState::Stopped | crate::MissionSupervisionState::Stale
                | crate::MissionSupervisionState::LegacyUnverified => {}
        }
    }
    let actions=rows(&store.db,"SELECT at,kind,message FROM cockpit_events WHERE work_id=?1 AND (kind='conductor' OR kind IN ('validation-accepted','validation-retained') OR (kind='dispatch' AND message LIKE '% : départ automatique · %')) ORDER BY seq DESC LIMIT 1",Some(work),|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?;
    if let Some((stamp, kind, summary)) = actions.into_iter().next() {
        out.last_action = Some(Box::new(MissionSupervisionAction {
            relative: relative(at, &stamp, tolerance),
            at: stamp,
            kind,
            summary,
            actor: "Conducteur Swarm".into(),
        }));
    }
    Ok(out)
}
/// Borrow the producer's latest-first view without copying private Agent data.
/// None includes all rows and zero includes none. The complete source remains
/// available to ownership/coherence checks and accounting; no row is deleted.
fn recent_agents(agents: &[crate::Agent], limit: Option<usize>) -> &[crate::Agent] {
    &agents[..limit.unwrap_or(agents.len()).min(agents.len())]
}
/// Project newest raw events after the visit, using the explicit saved window
/// before category filtering. None includes every event; zero yields an empty
/// page with more=true when events exist. Inputs/visit/ledger are never mutated.
/// Selected payload codec failures retain their cause; omitted older payloads
/// are not decoded here. The complete retained reader still validates storage.
fn changes(input: &super::MissionStatusInputs, limit: Option<usize>) -> Result<MissionChanges, E> {
    let from = input.visit.as_ref().map_or(0, |v| v.revision);
    let first = input.visit.as_ref().is_none_or(|v| v.at.is_empty());
    let mut out = MissionChanges {
        from,
        to: input.work.revision,
        first,
        more: false,
        items: Some(Vec::new()),
    };
    if first {
        return Ok(out);
    }
    let mut events: Vec<_> = input
        .events
        .iter()
        .filter(|e| e.revision > from && e.revision <= input.work.revision)
        .collect();
    events.sort_by_key(|event| std::cmp::Reverse(event.revision));
    if let Some(limit) = limit {
        out.more = events.len() > limit;
        events.truncate(limit);
    }
    #[derive(serde::Deserialize, Default)]
    struct Payload {
        #[serde(default)]
        id: String,
        #[serde(default)]
        task_id: String,
        #[serde(default)]
        attempt_id: String,
        #[serde(default)]
        status: String,
        #[serde(default)]
        next: String,
        #[serde(default)]
        state: String,
    }
    for event in events {
        let kind = String::from_utf8_lossy(&event.kind);
        let payload: Payload = match event.payload.as_deref() {
            Some(raw) => {
                serde_json::from_slice(raw).map_err(|e| E::Encode(crate::WorkError::Json(e)))?
            }
            None => Payload::default(),
        };
        let category = match kind.as_ref() {
            "task.update" => match payload.status.as_str() {
                "submitted" => Some(("result", "Résultat soumis pour examen")),
                "accepted" => Some(("result", "Acceptation enregistrée")),
                "blocked" => Some(("block", "Blocage enregistré")),
                _ if !payload.next.is_empty() => Some(("decision", "Consigne de reprise modifiée")),
                _ => None,
            },
            "review.result" if payload.state == "passed" => {
                Some(("result", "Avis indépendant enregistré"))
            }
            "review.result" => Some(("block", "Vérification à examiner")),
            "review.failure" | "planning.proof-stale" => {
                Some(("block", "Preuve ou vérification à examiner"))
            }
            "task.auto-validation" if payload.state == "blocked" => {
                Some(("block", "Contrôle en échec"))
            }
            "task.auto-validation" => Some(("result", "Contrôles enregistrés")),
            "task.auto-validation-reviewed" => {
                Some(("result", "Acceptation après contrôles et revue"))
            }
            "decision"
            | "decision.moteur"
            | "planning.decide"
            | "planning.authorize-recovery"
            | "planning.extend-attempt" => Some(("decision", "Décision enregistrée")),
            _ => None,
        };
        let Some((category, label)) = category else {
            continue;
        };
        // Review/attempt identifiers resolve only through persisted ownership,
        // never a coincident task ID. Go uses the last matching task in order.
        let tasks = input.work.tasks.as_deref().unwrap_or(&[]);
        let id = match kind.as_ref() {
            "review.result" => tasks.iter().rfind(|t|
                t.independent_review.as_deref().is_some_and(|r|r.id==payload.id)
                || t.previous_reviews.iter().flatten().any(|r|r.id==payload.id))
                .map_or("",|t|t.id.as_str()),
            "task.auto-validation" => tasks.iter().rfind(|t|
                t.attempts.iter().flatten().any(|attempt|attempt.id==payload.attempt_id))
                .map_or("",|t|t.id.as_str()),
            _ if !payload.id.is_empty() => &payload.id,
            _ => &payload.task_id,
        };
        let task = tasks.iter().find(|task|task.id==id);
        out.items.as_mut().unwrap().push(MissionChange {
            revision: event.revision,
            at: String::from_utf8_lossy(&event.at).into_owned(),
            category: category.into(),
            label: label.into(),
            task: task.map_or(String::new(), |t| t.id.clone()),
            title: task.map_or(String::new(), |t| t.title.clone()),
        });
    }
    Ok(out)
}

impl RootStore {
    /// Recheck original first-worker attribution without requiring its mutable
    /// copy or invoking Git/provider operations. Errors retain launch causes;
    /// caller holds the material read and rechecks Work/Agents/settings. Observe
    /// performs no INSERT or reservation change; its deferred read transaction
    /// and rollback preserve operation/cleanup causes. Sequential checks are not an
    /// atomic filesystem snapshot or protection from same-UID tampering.
    fn status_managed_workers(&self, work: &crate::Work, agents: &[crate::Agent], settings: &crate::EngineSettingsDocument) -> Result<(), E> {
        // Original native provenance includes the FIFO journal, whose coherent
        // history reader requires an explicit read transaction. Close it before
        // the enclosing status performs other independent read transactions.
        super::mission_supervision_store::read_transaction(&self.db, || {
            for agent in agents {
                crate::managed_repository::verify_worker(&self.root, &self.db, work, agent, settings,
                    crate::managed_repository::WorkerCheck::Observe).map_err(|e|
                        crate::MissionSupervisionError::Root(Box::new(E::Launch(Box::new(e)))))?;
            }
            Ok(())
        }).map_err(mission)
    }
    /// Read the Go 21-key historical aggregate for supported ordinary Work and
    /// ready/waiting planning trees, including unstarted managed tasks. Initial
    /// tasks consume real saved
    /// readiness/slots/budget; abandoned no-Agent decisions stay nonvalidated.
    /// Actual queued/active/failed/interrupted/completed attempts are observed
    /// without launching or reconciling them. Frozen process identity/heartbeat
    /// distinguishes observed live from uncertain; SQL status alone is not live.
    /// Submitted reports and current nonmanaged review opinions are presented
    /// from actual attempt/producer/contract/artifact observations. Passed is no
    /// acceptance. Accepted tasks independently recheck native originals, current
    /// gate, report and dependencies; stale evidence never becomes validated.
    /// Display windows affect presentation only, not full-history accounting or
    /// eligibility. Hidden running/blocked Agents refuse rather than invent state.
    /// Review/validation events resolve only persisted review/attempt ownership.
    /// Managed copy paths are prospective metadata, never physical readiness.
    /// Previous review/retry/recovery, managed submitted/accepted results and closed scopes remain
    /// explicit port boundaries. Missing policy/evidence and unknown IO refuse,
    /// preserving typed semantic/cleanup causes beyond the material guard.
    /// Work/settings/controls/Agents and selected reports are compared before and
    /// after. These sequential observations establish neither an atomic snapshot
    /// nor ABA protection. No visit, provider, SQL, Git or ledger mutation occurs.
    /// Caller must close Root and handle output/flush; observation grants nothing.
    pub fn mission_status_historical(
        &self,
        work: &str,
        operator: &str,
    ) -> Result<MissionStatusObservation, E> {
        let mut causes = Vec::new();
        let mut baseline = None;
        let result = super::material_path::with_material_read(self, || {
            let input = self.mission_status_inputs(work, operator)?;
            let settings = self.read_settings()?;
            let controls = self.read_mission_controls(work).map_err(mission)?;
            let now = crate::system_timestamp().map_err(E::Runtime)?;
            let result = (|| {
                let managed = input.work.planning.as_deref().is_some_and(|p| p.repository.is_some());
                if managed { self.status_managed_workers(&input.work, &input.agents, &settings)?; }
                if managed {
                    super::mission_status_reports::supported_observed(&input.work, &input.agents, true)?;
                } else {
                    super::mission_status_reports::supported(&input.work, &input.agents)?;
                }
                let (agent_limit, event_limit) = settings.values.mission_status_view_limits().map_err(E::Settings)?;
                let agents = recent_agents(&input.agents, agent_limit);
                super::mission_status_reports::supported_view(&input.work, agents)?;
                // Submitted results and no-Agent abandoned decisions bypass launch
                // readiness. Submitted results retain unknown process identity;
                // abandonment preserves its existing not_started process fact.
                // Other missing-agent branches still require real readiness and
                // preserve refusal rather than inventing a missing profile.
                // Full-history eligibility and display prerequisites are distinct:
                // hidden completed rows keep unknown identity; hidden active or
                // blocked rows refuse rather than inventing launch readiness.
                let cooldown_at = crate::mission_supervision::parse(&now).map_err(mission)?.unix_timestamp();
                let cooldowns = super::mission_status_reports::active_cooldowns(&self.root, &input.work, cooldown_at)?;
                let diagnostics =
                    super::mission_status_reports::diagnostics(agents)?;
                let states = super::mission_status_activity::observe(agents, &now, &mut causes)?;
                let mut agent_observations =
                    super::mission_status_reports::agent_observations(agents, &diagnostics)?;
                for (observation, state) in agent_observations.iter_mut().zip(&states) {
                    observation.observed_state = state;
                }
                let tasks = input.work.tasks.as_deref().unwrap_or(&[]);
                let policy = settings
                    .values
                    .mission_supervision_policy()
                    .map_err(E::Settings)?;
                let tolerance = policy.future_tolerance_ms.0;
                let observed = crate::mission_status_runtime::observe(&self.root, &settings, &now)?;
                causes.extend(observed.causes);
                // These Go dispatch inputs are genuinely read even though missing
                // profiles terminate dispatch before capacity or cost decisions.
                let context = dispatch_context(self, work)?;
                let authorized = controls.consent.as_ref().is_some_and(|c| c.enabled)
                    && controls
                        .controls
                        .as_ref()
                        .is_some_and(|c| c.autonomy == super::MissionAutonomy::Automatic);
                let paused = controls.controls.as_ref().is_some_and(|c| c.paused);
                let supervision = supervision(self, work, &now, tolerance)?;
                baseline = Some((context, supervision.clone(), tolerance));
                let mut evidence =
                    super::mission_status_evidence::read_freshness(self, &input.work, &settings)?;
                for fact in evidence.values_mut() {
                    causes.append(&mut fact.causes);
                }
                // Keep all current dependency facts before per-task entries are
                // consumed; task order must not turn an accepted parent stale.
                let dependency_current: std::collections::BTreeMap<_, _> = evidence.iter()
                    .map(|(id, fact)| (id.clone(), matches!(fact.freshness, FreshnessObservation::Current)))
                    .collect();
                let mut report_snapshots = Vec::new();
                let runtime_policy = settings
                    .values
                    .runtime_limit_policy()
                    .map_err(E::Settings)?;
                let observations:Vec<_>=tasks.iter().map(|t| {
                    let fact=evidence.remove(&t.id).ok_or(E::Refused("mission-status-freshness-missing"))?;
                    let agent=agents.iter().find(|a|a.task_id==t.id);
                    let dependencies:Vec<_>=t.depends.as_deref().unwrap_or(&[]).iter().map(|id|DependencyObservation {task_id:id.clone(),freshness:if dependency_current.get(id)==Some(&true) {DependencyFreshness::Current} else {DependencyFreshness::NotCurrent}}).collect();
                    let result=super::mission_status_reports::observed_result_inputs(&self.root,&input.work,t,agent,&settings,&mut causes,fact)?;
                    if let ReportListing::Observed(paths)=&result.reports {report_snapshots.push((t.id.clone(),paths.clone()));}
                    let execution=if t.status=="blocked" && t.plan_max_attempts>0 && t.attempts.as_ref().map_or(0,Vec::len) as u128>=t.plan_max_attempts as u128 {
                        let admin=crate::run_limits::read_configured_runtime_limits(&self.db,&runtime_policy,work,"worker",&t.id)?;
                        if admin.observation_mode==1 {ExecutionObservation::Observed} else {ExecutionObservation::NotPresent}
                    } else {ExecutionObservation::NotObserved};
                    let readiness=if agent.is_some() || matches!(t.status.as_str(),"submitted"|"abandoned"|"accepted") {ReadinessObservation::NotObserved} else {readiness(self,&input.work,t,&controls,&settings,&dependencies,&mut causes)?};
                    Ok(TaskObservation {task_id:t.id.clone(),result,dependencies,readiness,prepared_launch:PreparedLaunchObservation::Absent,execution})
                }).collect::<Result<_,E>>()?;
                let mut status=MissionStatus {changes:changes(&input,event_limit)?,spending:crate::mission_status_spending::mission_spending(&input,&now)?,guidance:MissionGuidance {what:String::new(),next:String::new(),actor:String::new(),primary:MissionPrimaryAction {kind:String::new(),label:String::new(),effect:String::new(),task:String::new(),tone:String::new()}},runtime:observed.runtime,provider_cooldowns:(!cooldowns.is_empty()).then(|| cooldowns.clone()),evidence_stage:insight::mission_evidence_stage(&input.work,0,0),organization:organization(&input.work)?,authorized,enabled:false,paused,summary:String::new(),next:String::new(),validated:0,review:0,running:0,active_agents:0,uncertain_agents:0,total:i64::try_from(tasks.len()).map_err(|_|E::Refused("mission-status-count-representation"))?,supervision,understanding:understanding(),coordination:None,tasks:Some(Vec::new())};
                for (_, state) in agents.iter().zip(&states).filter(|(agent, _)| agent.is_active()) {
                    let count = if matches!(state.as_str(), "queued" | "starting" | "running" | "stopping") {
                        &mut status.active_agents
                    } else { &mut status.uncertain_agents };
                    *count = count.checked_add(1).ok_or(E::Refused("mission-status-count-representation"))?;
                }
                // Go enabled precedes organization override; task explanations use it.
                status.enabled = authorized
                    && !paused
                    && status.supervision.state == "active"
                    && status.runtime.launch_allowed;
                let projected =
                    project_tasks(&input.work, &status, &observations, &agent_observations)?;
                status.validated = projected.validated;
                status.review = projected.review;
                status.running = projected.running;
                status.tasks = Some(projected.tasks);
                status.summary=format!("{}/{} résultats validés · {} à examiner · {} tâche(s) en cours · {} agent(s) actif(s) confirmé(s) · {} état(s) agent incertain(s)",status.validated,status.total,status.review,status.running,status.active_agents,status.uncertain_agents);
                status.understanding = insight::mission_understanding(&status);
                insight::apply_planning_understanding(&input.work, &mut status);
                if !status.organization.ready {
                    status.enabled = false;
                    status.next = status.organization.next.clone();
                    status.understanding = MissionUnderstanding {
                        what: status.organization.label.clone(),
                        next_step: status.organization.next.clone(),
                        actor: "Vous".into(),
                        actor_kind: "user".into(),
                        situation: "decision_humaine".into(),
                    };
                }
                apply_provider_waits(&input.work, &mut status)?;
                if status.runtime.state == "blocked" {
                    status.understanding = insight::mission_understanding(&status);
                    status.next = status.runtime.next.clone();
                }
                let at = crate::mission_supervision::parse(&now).map_err(mission)?;
                status.coordination = Some(insight::mission_coordination_phases(
                    &input.work,
                    &status,
                    &input.exchanges,
                    at,
                    Duration::from_millis(
                        u64::try_from(tolerance)
                            .map_err(|_| E::Refused("mission-status-tolerance-representation"))?,
                    ),
                ));
                status.guidance = insight::mission_guidance(&input.work, &status);
                for (task, paths) in report_snapshots {
                    if super::mission_status_reports::task_reports(&self.root, &task)? != paths {
                        return Err(E::Refused("mission-status-reports-changed"));
                    }
                }
                if super::mission_status_reports::active_cooldowns(&self.root, &input.work, cooldown_at)? != cooldowns {
                    return Err(E::Refused("mission-status-provider-cooldowns-changed"));
                }
                if managed { self.status_managed_workers(&input.work, &input.agents, &settings)?; }
                Ok(status)
            })();
            let work_check = self.read_work(work).and_then(|w| {
                if w == input.work {
                    Ok(())
                } else {
                    Err(E::Refused("mission-status-work-changed"))
                }
            });
            let settings_check = self.read_settings().and_then(|s| {
                if s == settings {
                    Ok(())
                } else {
                    Err(E::Refused("mission-status-settings-changed"))
                }
            });
            let agents_check = self.mission_status_inputs(work, operator).and_then(|v| {
                if v.agents == input.agents
                    && v.work == input.work
                    && v.events == input.events
                    && v.visit == input.visit
                    && v.planning_calls == input.planning_calls
                    && v.exchanges == input.exchanges
                {
                    Ok(())
                } else {
                    Err(E::Refused("mission-status-inputs-changed"))
                }
            });
            let controls_check = self
                .read_mission_controls(work)
                .map_err(mission)
                .and_then(|c| {
                    if c == controls {
                        Ok(())
                    } else {
                        Err(E::Refused("mission-status-controls-changed"))
                    }
                });
            let context_check = if let Some((context, lease, tolerance)) = baseline.take() {
                let dispatch = dispatch_context(self, work).and_then(|current| {
                    if current == context {
                        Ok(())
                    } else {
                        Err(E::Refused("mission-status-dispatch-context-changed"))
                    }
                });
                let lease = supervision(self, work, &now, tolerance).and_then(|current| {
                    if current == lease {
                        Ok(())
                    } else {
                        Err(E::Refused("mission-status-supervision-changed"))
                    }
                });
                finished(dispatch, lease)
            } else {
                Ok(())
            };
            finished(
                result,
                finished(
                    finished(work_check, settings_check),
                    finished(finished(agents_check, controls_check), context_check),
                ),
            )
        });
        match result {
            Ok(status) => Ok(MissionStatusObservation { status, causes }),
            Err(error) => Err(retain(error, causes)),
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::SystemEnvironment;
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
    /// Own a fresh public Root fixture and preserve its state on a failed test.
    struct Fixture(PathBuf);
    impl Fixture {
        /// Generate a unique fixture directory; mode0700 attributes fixture data.
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(crate::generate_identifier("status-historical-").unwrap());
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        /// Remove only this test's data following success, retaining failures.
        fn drop(&mut self) {
            if std::thread::panicking() {
                eprintln!("historical status fixture retained: {}", self.0.display());
            } else {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }
    /// Form the actual public mutation envelope with explicit fixture content.
    fn request(event: &str, revision: i64, mut body: serde_json::Value) -> Vec<u8> {
        body["schema_version"] = 1.into();
        body["event_id"] = event.into();
        body["expected_revision"] = revision.into();
        serde_json::to_vec(&body).unwrap()
    }
    /// Event review/receipt identities resolve only through persisted task
    /// ownership, even when an unrelated task id collides. Pure observation
    /// inputs prove mapping only, not canonical Root creation or permission.
    #[test]
    fn mission_status_changes_resolves_review_and_attempt_owners() {
        let (mut work,_)=submitted_planning_fixture();
        work.id="owned-work".into();work.revision=10;
        work.tasks=Some(vec![serde_json::from_value(serde_json::json!({"id":"review-id","title":"Wrong collision"})).unwrap(),serde_json::from_value(serde_json::json!({"id":"actual-task","title":"Actual owner","attempts":[{"id":"attempt-id"}],"independent_review":{"id":"review-id"},"previous_reviews":[{"id":"prior-review"}]})).unwrap()]);
        let event=|revision,kind:&str,payload:serde_json::Value| crate::RecordedEvent{id:format!("event-{revision}").into_bytes(),work_id:b"owned-work".to_vec(),revision,kind:kind.as_bytes().to_vec(),at:b"recorded-event".to_vec(),payload:Some(serde_json::to_vec(&payload).unwrap())};
        let mut input=super::super::MissionStatusInputs{work,agents:vec![],events:vec![event(1,"decision",serde_json::json!({"id":"actual-task"}))],visit:Some(super::super::MissionStatusVisit{revision:0,at:"recorded-visit".into()}),planning_calls:vec![],exchanges:vec![]};
        let control=changes(&input,None).unwrap();assert_eq!(control.items.unwrap()[0].task,"actual-task");
        input.events=vec![event(2,"review.result",serde_json::json!({"id":"review-id","state":"passed"})),event(3,"review.result",serde_json::json!({"id":"prior-review","state":"passed"})),event(4,"task.auto-validation",serde_json::json!({"attempt_id":"attempt-id"})),event(5,"review.result",serde_json::json!({"id":"missing","state":"passed"})),event(6,"task.auto-validation",serde_json::json!({"attempt_id":"review-id"}))];
        let observed=changes(&input,None).unwrap();let rows=observed.items.unwrap();assert_eq!(rows.len(),5);
        for row in rows {
            if (2..=4).contains(&row.revision) {assert_eq!(row.task,"actual-task","revision {}",row.revision);assert_eq!(row.title,"Actual owner");}
            else {assert!(row.task.is_empty() && row.title.is_empty());}
        }
    }
    /// Valid small page followed by 201 retained raw events; reads never mark a
    /// visit, alter ledger inputs or silently include an older visible event.
    #[test]
    fn mission_status_changes_recent_window_preserves_complete_inputs() {
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
        let work = store.execute_external("", "work.create", &request("create-window", 0,
            serde_json::json!({"title":"Window", "objective":"Retain all history", "scope":"Fixture", "criteria":["Exact page"]})),
            &mut SystemEnvironment).unwrap().result.work;
        let mut input = store.mission_status_inputs(&work.id, "window-operator").unwrap();
        input.visit = Some(super::super::mission_status_read::MissionStatusVisit {revision:0, at:"recorded-visit".into()});
        input.work.revision = 201;
        input.events = (1..=2).map(|revision| crate::RecordedEvent {
            id: format!("event-{revision}").into_bytes(), work_id:work.id.as_bytes().to_vec(),
            revision, kind:b"decision".to_vec(), at:b"recorded-event".to_vec(), payload:Some(b"{}".to_vec()),
        }).collect();
        let control = changes(&input, Some(200)).unwrap();
        assert_eq!(control.items.as_ref().unwrap().len(), 2);
        assert!(!control.more);
        let template = input.events[0].clone();
        input.events = (1..=201).map(|revision| {let mut row=template.clone(); row.revision=revision; row.id=format!("event-{revision}").into_bytes();row}).collect();
        let page = changes(&input, Some(200)).unwrap();
        assert!(page.more);
        let items = page.items.as_ref().unwrap();
        assert_eq!(items.len(), 200);
        assert_eq!(items[0].revision, 201);
        assert_eq!(items[199].revision, 2);
        assert_eq!(input.events.len(), 201);
        let unlimited = changes(&input, None).unwrap();
        assert!(!unlimited.more);
        assert_eq!(unlimited.items.as_ref().unwrap().len(), 201);
        let empty = changes(&input, Some(0)).unwrap();
        assert!(empty.more && empty.items.as_ref().unwrap().is_empty());
        // The raw page limit is consumed before visibility/category filtering.
        input.events[200].kind=b"not-visible".to_vec();
        let filtered=changes(&input, Some(200)).unwrap();
        assert!(filtered.more);
        assert_eq!(filtered.items.as_ref().unwrap().len(), 199);
        input.events[200].kind=b"decision".to_vec();
        input.events[200].payload=Some(b"invalid-json".to_vec());
        assert!(changes(&input, Some(200)).is_err());
        input.events[200].payload=Some(b"{}".to_vec());
        assert_eq!(changes(&input, Some(200)).unwrap().items.as_ref().unwrap().len(), 200);
        // A bounded borrowed view never becomes the accounting source.
        input.agents=(0..201).map(|index| {
            crate::Agent { id: format!("agent-{index}"), work_id: input.work.id.clone(), task_id:"task".into(), attempt_id:format!("attempt-{index}"), role:"worker".into(), mode:"pipe".into(), status:"completed".into(), ..crate::Agent::default() }
        }).collect();
        let agents=recent_agents(&input.agents, Some(200));
        assert_eq!(agents.len(),200);
        assert_eq!(agents[0].id,"agent-0");
        assert_eq!(agents[199].id,"agent-199");
        assert_eq!(recent_agents(&input.agents,None).len(),201);
        assert!(recent_agents(&input.agents,Some(0)).is_empty());
        input.agents[200].usage=Some(Box::new(serde_json::from_value(serde_json::json!({"input_tokens":7,"output_tokens":9,"provider_reported_cost_usd":0.25,"source":"fixture","scope":"attempt","at":"recorded"})).unwrap()));
        let spending=crate::mission_status_spending::mission_spending(&input,"observation").unwrap();
        assert_eq!(spending.calls.as_ref().unwrap().len(),201);
        assert_eq!(spending.attempts.as_ref().unwrap().len(),201);
        assert_eq!(spending.rows.as_ref().unwrap()[0].calls,201);
        assert_eq!(spending.rows.as_ref().unwrap()[0].input,7);
        assert_eq!(spending.rows.as_ref().unwrap()[0].output,9);
        assert_eq!(spending.rows.as_ref().unwrap()[0].cost.reported_usd.0,0.25);
        assert_eq!(input.agents.len(),201);
        input.visit=None;
        assert!(!changes(&input, Some(0)).unwrap().more);
        assert_eq!(input.events.len(), 201);
        store.close().unwrap();
    }
    /// Representation boundaries and custom persisted cadence, never a sleep.
    #[test]
    fn mission_status_deadline_units_rounding_and_invalid_recovery() {
        let at = crate::mission_supervision::parse("2026-10-23T00:00:00Z").unwrap();
        assert_eq!(next_relative(at, "2026-10-23T00:00:01.5Z", 1500).unwrap(), "dans moins de 1500 ms");
        assert_eq!(next_relative(at, "2026-10-23T00:00:02Z", 2000).unwrap(), "dans moins de 2 s");
        assert_eq!(next_relative(at, "2026-10-23T00:00:03.5Z", 2000).unwrap(), "dans environ 4 s");
        assert_eq!(next_relative(at, "2026-10-22T23:59:59.999999999Z", 2000).unwrap(), "attendue maintenant");
        assert_eq!(next_relative(at, "2026-10-22T23:59:58.5Z", 2000).unwrap(), "en retard d’environ 2 s");
        assert!(next_relative(at, "2026-10-23T00:00:01Z", 0).is_err());
        assert!(next_relative(at, "PRIVATE-MALFORMED-TIME", 2000).is_err());
        assert_eq!(next_relative(at, "2026-10-23T00:00:02Z", 2000).unwrap(), "dans moins de 2 s");
    }
    /// A real public native claim must be observable without starting a worker.
    /// Reads use frozen policy boundaries, no sleeps, SQL seeding or acceptance.
    #[test]
    fn mission_status_native_lease_active_stale_future_and_recovery() {
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
        let work = store.execute_external("", "work.create", &request("lease-status-create", 0,
            serde_json::json!({"title":"Lease status", "objective":"Observe real claim", "scope":"Owned fixture", "criteria":["No worker"]})),
            &mut SystemEnvironment).unwrap().result.work;
        let now = crate::system_timestamp().unwrap();
        let at = crate::mission_supervision::parse(&now).unwrap();
        let tolerance = store.settings().unwrap().values.mission_supervision_policy().unwrap().future_tolerance_ms.0;
        assert_eq!(supervision(&store, &work.id, &now, tolerance).unwrap().state, "absent");
        let lease = store.claim_mission_supervision(&work.id, "owned-status-fixture", &now).unwrap();
        let policy = lease.policy().clone();
        let stamp = |delta: i128| (at + time::Duration::nanoseconds_i128(delta))
            .format(&time::format_description::well_known::Rfc3339).unwrap();
        let active = supervision(&store, &work.id, &now, tolerance).unwrap();
        assert_eq!(active.state, "active");
        assert!(!active.next_check_at.is_empty());
        assert!(!active.late);
        assert_eq!(supervision(&store, &work.id,
            &stamp(i128::from(policy.stale_after_ms.0) * 1_000_000), tolerance).unwrap().state, "active");
        let stale = supervision(&store, &work.id,
            &stamp(i128::from(policy.stale_after_ms.0) * 1_000_000 + 1), tolerance).unwrap();
        assert_eq!(stale.state, "absent");
        assert!(stale.next_check_at.is_empty());
        assert_eq!(supervision(&store, &work.id,
            &stamp(-i128::from(policy.future_tolerance_ms.0) * 1_000_000), tolerance).unwrap().state, "active");
        let future = supervision(&store, &work.id,
            &stamp(-i128::from(policy.future_tolerance_ms.0) * 1_000_000 - 1), tolerance).unwrap();
        assert_eq!(future.state, "absent");
        assert!(!future.clock_issue.is_empty());
        assert!(future.next_check_at.is_empty());
        store.record_mission_cycle(&lease, &now, Some("owned fixture cycle failure")).unwrap();
        assert_eq!(supervision(&store, &work.id, &now, tolerance).unwrap().state, "error");
        store.record_mission_cycle(&lease, &now, None).unwrap();
        assert_eq!(supervision(&store, &work.id, &now, tolerance).unwrap().state, "active");
        let public = store.mission_status_historical(&work.id, "operator-a").unwrap();
        assert_eq!(public.status.supervision.state, "active");
        assert_eq!(public.status.active_agents, 0);
        store.update_mission_supervision(&lease, &now, true).unwrap();
        let stopped = supervision(&store, &work.id, &now, tolerance).unwrap();
        assert_eq!(stopped.state, "absent");
        assert!(stopped.next_check_at.is_empty());
        lease.close().unwrap();
        store.close().unwrap();
    }
    /// Valid initial and claimed metadata precede invalid shape/counter probes.
    /// No supplied reviewer declaration is persisted or treated as executed review.
    /// Missing structural data retains the organization error in its source chain.
    #[test]
    fn mission_status_initial_planning_boundaries_and_organization() {
        let work: crate::Work = serde_json::from_value(serde_json::json!({
            "planning": {"version":1,"provider":"fixture","max_tasks":10,
                "max_activations":3,"max_decisions":10,
                "reviewer":{"provider":"fixture","authorized":"fixture","max_calls":2},
                "scopes":[{"id":"root","revision":1,"state":"ready"}]}
        })).unwrap();
        let supported = super::super::mission_status_reports::supported;
        supported(&work, &[]).unwrap();
        let observation = organization(&work).unwrap();
        assert!(observation.ready);
        assert!(observation.issues.unwrap().is_empty());
        assert!(observation.verification.contains("0/2 appels"));
        assert!(observation.verification.contains("aucune revue IA indépendante implicite"));
        for variant in 0..7 {
            supported(&work, &[]).unwrap();
            let mut invalid = work.clone();
            let p = invalid.planning.as_deref_mut().unwrap();
            match variant {
                0 => p.activations = -1,
                1 => p.decisions = -1,
                2 => p.scopes.as_mut().unwrap()[0].holder = "fixture-holder".into(),
                3 => p.scopes.as_mut().unwrap()[0].lease_until = "future".into(),
                4 => p.scopes.as_mut().unwrap()[0].state = "closed".into(),
                5 => p.scopes.as_mut().unwrap()[0].activations = -1,
                _ => invalid.tasks = Some(vec![crate::Task::default()]),
            }
            assert!(supported(&invalid, &[]).is_err());
        }
        let mut claimed = work.clone();
        let planning = claimed.planning.as_deref_mut().unwrap();
        planning.activations = planning.max_activations;
        planning.decisions = 1;
        let scope = &mut planning.scopes.as_mut().unwrap()[0];
        scope.activations = 1;
        scope.holder = "fixture-holder".into();
        scope.lease_until = "2026-10-23T12:00:00Z".into();
        supported(&claimed, &[]).unwrap(); // A consumed cap stays observable.
        claimed.planning.as_deref_mut().unwrap().scopes.as_mut().unwrap()[0].lease_until = "invalid".into();
        let error = supported(&claimed, &[]).unwrap_err();
        assert_eq!(error.code(), "mission-status-planning-lease-invalid");
        assert!(std::error::Error::source(&error).is_some());
        assert!(supported(&work, &[crate::Agent::default()]).is_err());
        let mut invalid = work.clone();
        invalid.planning.as_deref_mut().unwrap().reviewer = None;
        let error = supported(&invalid, &[]).unwrap_err();
        assert_eq!(error.code(), "mission-status-organization-invalid");
        assert!(std::error::Error::source(&error).is_some());
        assert!(organization(&invalid).is_err());
        let mut human = work;
        human.planning.as_deref_mut().unwrap().human_review_authorized = "fixture".into();
        assert!(organization(&human).unwrap().verification.contains("Revue humaine explicitement autorisée"));
    }
    /// Pure shape witnesses only; real public execution/submission is verified
    /// separately. Human validation metadata here is supplied fixture data.
    fn submitted_planning_fixture() -> (crate::Work, crate::Agent) {
        let work = serde_json::from_value(serde_json::json!({
            "criteria":["Owned requirement"],
            "planning":{"version":1,"provider":"fixture","max_tasks":10,
                "max_activations":3,"max_decisions":10,"human_review_authorized":"fixture",
                "reviewer":{"provider":"fixture","authorized":"fixture","max_calls":2},
                "scopes":[{"id":"root","revision":1,"state":"ready","requirements":["req-1"]}]},
            "tasks":[{"id":"t","status":"submitted","scope_id":"root","plan_role":"worker","requirements":["req-1"],
                "validation_policy":{"mode":"human","authorized_at":"fixture"}}]
        })).unwrap();
        (work,crate::Agent {id:"a".into(),task_id:"t".into(),status:"completed".into(),..Default::default()})
    }
    /// Require full completed history before projection; reject unknown/active pairings.
    #[test]
    fn mission_status_planned_submitted_requires_full_completed_history() {
        let (work,agent) = submitted_planning_fixture();
        let supported = super::super::mission_status_reports::supported;
        supported(&work,std::slice::from_ref(&agent)).unwrap();
        assert!(supported(&work,&[]).is_err());
        for state in ["running","starting","queued","stopping","failed","interrupted","unknown"] {
            let mut invalid=agent.clone();invalid.status=state.into();
            assert!(supported(&work,&[invalid]).is_err(),"{state}");
        }
        for state in ["todo","abandoned"] {
            let mut invalid=work.clone();invalid.tasks.as_mut().unwrap()[0].status=state.into();
            assert!(supported(&invalid,std::slice::from_ref(&agent)).is_err(),"{state}");
        }
        let mut blocked=work.clone();blocked.tasks.as_mut().unwrap()[0].status="blocked".into();
        supported(&blocked,std::slice::from_ref(&agent)).unwrap();
        blocked.tasks.as_mut().unwrap()[0].blocker="Livraison incomplète : evidence missing".into();
        assert!(supported(&blocked,std::slice::from_ref(&agent)).is_err());
        let mut invalid=agent.clone();invalid.task_id="other".into();
        assert!(supported(&work,&[invalid]).is_err());
        supported(&work,&[agent]).unwrap();
    }
    /// Keep independent-review, recovery, managed and invalid-tree boundaries closed.
    #[test]
    fn mission_status_planned_submitted_keeps_review_recovery_and_tree_refusals() {
        let (work,agent) = submitted_planning_fixture();
        let supported=super::super::mission_status_reports::supported;
        supported(&work,std::slice::from_ref(&agent)).unwrap();
        for kind in 0..7 {
            let mut invalid=work.clone();let t=&mut invalid.tasks.as_mut().unwrap()[0];
            match kind {
                0=>t.independent_review=Some(Default::default()),
                1=>t.previous_reviews=Some(vec![Default::default()]),
                2=>t.corrective_recovery=Some(Default::default()),
                3=>t.planning_retry=true,
                4=>t.review_coordination=Some(Default::default()),
                5=>t.batch_review_resume=Some(Default::default()),
                _=>t.recovered_result=Some(Default::default()),
            }
            assert!(supported(&invalid,std::slice::from_ref(&agent)).is_err());
        }
        let mut invalid=work.clone();invalid.planning.as_mut().unwrap().repository=Some(Default::default());
        assert!(supported(&invalid,std::slice::from_ref(&agent)).is_err());
        let mut invalid=work.clone();invalid.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].state="closed".into();
        assert!(supported(&invalid,std::slice::from_ref(&agent)).is_err());
        let mut invalid=work.clone();invalid.tasks.as_mut().unwrap()[0].validation_policy=None;
        let error=supported(&invalid,std::slice::from_ref(&agent)).unwrap_err();
        assert_eq!(error.code(),"mission-status-organization-invalid");
        assert!(std::error::Error::source(&error).is_some());
        supported(&work,&[agent]).unwrap();
    }
    /// Accepted planning is a shape observation; actual freshness is supplied
    /// by the Root reader, never established by a passed row in this pure test.
    /// Other opinions and all earlier review/recovery branches still refuse.
    #[test]
    fn mission_status_planned_passed_opinion_and_accepted_shape() {
        let (mut work,agent)=submitted_planning_fixture();
        let supported=super::super::mission_status_reports::supported;
        let task=&mut work.tasks.as_mut().unwrap()[0];
        task.independent_review=Some(Box::new(crate::IndependentReview{state:"passed".into(),..Default::default()}));
        supported(&work,std::slice::from_ref(&agent)).unwrap();
        work.tasks.as_mut().unwrap()[0].status="accepted".into();
        supported(&work,std::slice::from_ref(&agent)).unwrap();
        for state in ["running","error","stale","changes_requested",""] {
            work.tasks.as_mut().unwrap()[0].independent_review.as_mut().unwrap().state=state.into();
            assert!(supported(&work,std::slice::from_ref(&agent)).is_err(),"{state}");
        }
    }
    /// Preserve historical metadata-only reads; a managed submitted attempt
    /// still refuses until its actual repository producers are connected.
    #[test]
    fn mission_status_planned_initial_managed_metadata_keeps_prior_read_boundary() {
        let (mut work,agent)=submitted_planning_fixture();
        let supported=super::super::mission_status_reports::supported;
        work.tasks.as_mut().unwrap()[0].status="todo".into();
        supported(&work,&[]).unwrap();
        work.planning.as_mut().unwrap().repository=Some(Default::default());
        supported(&work,&[]).unwrap();
        work.tasks.as_mut().unwrap()[0].status="submitted".into();
        assert!(supported(&work,&[agent]).is_err());
    }
    /// A recorded queued attempt can be described without claiming a live process.
    /// Other lifecycle pairs remain explicit, never guessed from missing Agents.
    #[test]
    fn mission_status_planned_queued_pairing_and_original_boundaries() {
        let (mut work,mut agent)=submitted_planning_fixture();
        let supported=super::super::mission_status_reports::supported;
        let view=super::super::mission_status_reports::supported_view;
        work.tasks.as_mut().unwrap()[0].status="running".into();
        work.tasks.as_mut().unwrap()[0].attempts=Some(vec![crate::Attempt {id:"x".into(),status:"queued".into(),..Default::default()}]);
        agent.status="queued".into();
        supported(&work,std::slice::from_ref(&agent)).unwrap();
        view(&work,std::slice::from_ref(&agent)).unwrap();
        assert_eq!(view(&work,&[]).unwrap_err().code(),"mission-status-task-history-unported");
        assert!(supported(&work,&[]).is_err());
        for state in ["starting","running","stopping"] {
            let mut observed=agent.clone();observed.status=state.into();
            supported(&work,std::slice::from_ref(&observed)).unwrap();
            view(&work,std::slice::from_ref(&observed)).unwrap();
        }
        for state in ["failed","interrupted","completed","unknown"] {
            let mut other=agent.clone();other.status=state.into();
            assert!(supported(&work,&[other]).is_err(),"{state}");
        }
        for state in ["todo","submitted","accepted","blocked","abandoned"] {
            let mut other=work.clone();other.tasks.as_mut().unwrap()[0].status=state.into();
            assert!(supported(&other,std::slice::from_ref(&agent)).is_err(),"{state}");
        }
        let mut invalid=work.clone();invalid.planning.as_mut().unwrap().repository=Some(Default::default());
        assert!(supported(&invalid,std::slice::from_ref(&agent)).is_err());
        supported(&work,&[agent]).unwrap();
    }
    /// Failed/interrupted/completed blocked rows need an Agent in view; completed
    /// shape alone does not supply attribution or establish validation.
    /// Pairing and review/recovery gates preserve refusal before any projection.
    #[test]
    fn mission_status_planned_terminal_pairs_preserve_view_and_review_refusals() {
        let (mut work,mut agent)=submitted_planning_fixture();
        let supported=super::super::mission_status_reports::supported;
        let view=super::super::mission_status_reports::supported_view;
        work.tasks.as_mut().unwrap()[0].status="blocked".into();
        for status in ["failed","interrupted","completed"] {
            agent.status=status.into();
            supported(&work,std::slice::from_ref(&agent)).unwrap();
            view(&work,std::slice::from_ref(&agent)).unwrap();
            assert!(view(&work,&[]).is_err());
            let mut invalid=work.clone();invalid.tasks.as_mut().unwrap()[0].planning_retry=true;
            assert!(supported(&invalid,std::slice::from_ref(&agent)).is_err());
            let mut invalid=work.clone();invalid.tasks.as_mut().unwrap()[0].independent_review=Some(Default::default());
            assert!(supported(&invalid,std::slice::from_ref(&agent)).is_err());
        }
        for status in ["queued","starting","running","stopping","unknown"] {
            agent.status=status.into();assert!(supported(&work,std::slice::from_ref(&agent)).is_err());
        }
        assert!(supported(&work,&[]).is_err());
    }
    /// Full history remains authoritative; only unavailable per-task view facts refuse.
    /// Preserve hidden completed/accepted and initial metadata behavior explicitly.
    #[test]
    fn mission_status_view_requires_active_or_blocked_agent_without_rejecting_terminal_metadata() {
        let (mut work,mut agent)=submitted_planning_fixture();work.planning=None;
        let supported=super::super::mission_status_reports::supported;
        let view=super::super::mission_status_reports::supported_view;
        for (task_state,agent_state) in [("running","queued"),("running","running"),("blocked","failed"),("blocked","interrupted")] {
            work.tasks.as_mut().unwrap()[0].status=task_state.into();agent.status=agent_state.into();
            supported(&work,std::slice::from_ref(&agent)).unwrap();
            view(&work,std::slice::from_ref(&agent)).unwrap();
            assert_eq!(view(&work,&[]).unwrap_err().code(),"mission-status-task-history-unported");
        }
        for state in ["submitted","accepted","abandoned","todo"] {
            work.tasks.as_mut().unwrap()[0].status=state.into();view(&work,&[]).unwrap();
        }
    }
    /// Pure schema witnesses for all-scope lease guards and structural causes.
    /// A valid tree precedes every child/root fault; no supplied Work becomes
    /// persisted authority, process liveness, permission or acceptance.
    #[test]
    fn mission_status_initial_planning_tree_boundaries() {
        let work: crate::Work = serde_json::from_value(serde_json::json!({
            "planning":{"version":1,"provider":"fixture","max_tasks":10,
                "max_activations":3,"max_decisions":10,
                "reviewer":{"provider":"fixture","authorized":"fixture","max_calls":2},
                "scopes":[{"id":"root","revision":1,"state":"waiting"},
                    {"id":"child","parent":"root","revision":1,"state":"ready"}]}
        })).unwrap();
        let supported = super::super::mission_status_reports::supported;
        supported(&work, &[]).unwrap();
        for index in 0..2 {
            for variant in 0..4 {
                let mut valid = work.clone();
                let scope = &mut valid.planning.as_deref_mut().unwrap().scopes.as_mut().unwrap()[index];
                scope.holder = "fixture-holder".into();
                scope.lease_until = "2026-10-23T12:00:00Z".into();
                supported(&valid, &[]).unwrap();
                let scope = &mut valid.planning.as_deref_mut().unwrap().scopes.as_mut().unwrap()[index];
                match variant {
                    0 => scope.lease_until.clear(),
                    1 => scope.holder.clear(),
                    2 => scope.holder = "../outside".into(),
                    _ => scope.lease_until = "invalid".into(),
                }
                let error = supported(&valid, &[]).unwrap_err();
                assert_eq!(error.code(), if variant == 3 {"mission-status-planning-lease-invalid"}
                    else {"mission-status-planning-lease-shape-invalid"});
                if variant == 3 { assert!(std::error::Error::source(&error).is_some()); }
            }
        }
        for variant in 0..4 {
            supported(&work, &[]).unwrap();
            let mut invalid = work.clone();
            let scopes = invalid.planning.as_deref_mut().unwrap().scopes.as_mut().unwrap();
            match variant {
                0 => scopes[1].parent = "missing".into(),
                1 => scopes[0].id = "other".into(),
                2 => scopes[1].state = "closed".into(),
                _ => scopes[0].state = "unknown".into(),
            }
            let error = supported(&invalid, &[]).unwrap_err();
            assert_eq!(error.code(), if variant < 2 {"mission-status-organization-invalid"}
                else {"mission-status-planning-unported"});
            if variant < 2 { assert!(std::error::Error::source(&error).is_some()); }
        }
    }
    /// Public Work/task writes are the positive witness; no SQL authority seeds.
    #[test]
    fn mission_status_historical_empty_todo_unknown_recovery_no_write() {
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
        let work=store.execute_external("","work.create",&request("create",0,serde_json::json!({"title":"Historical status","objective":"Read observations","scope":"Owned fixture","criteria":["No mutation"]})),&mut SystemEnvironment).unwrap().result.work;
        let settings = store.settings().unwrap();
        let events = store.events(&work.id).unwrap();
        let empty = store
            .mission_status_historical(&work.id, "operator-a")
            .unwrap();
        let json = serde_json::to_value(&empty.status).unwrap();
        assert_eq!(json.as_object().unwrap().len(), 21);
        assert_eq!(json["total"], 0);
        assert_eq!(json["tasks"], serde_json::json!([]));
        assert_eq!(json["changes_since_visit"]["first_visit"], true);
        assert_eq!(json["organization"]["ready"], false);
        assert!(!empty.status.authorized && !empty.status.enabled && !empty.status.paused);
        assert!(json.get("schema_version").is_none());
        assert!(json.get("provider_cooldowns").is_none());
        assert_eq!(json["supervision"]["state"], "absent");
        assert!(store
            .mission_status_historical("unknown", "operator-a")
            .is_err());
        assert_eq!(
            store
                .mission_status_historical(&work.id, "operator-a")
                .unwrap()
                .status
                .total,
            0
        );
        assert!(store.read_work(&work.id).unwrap() == work);
        assert!(store.settings().unwrap() == settings);
        assert!(store.events(&work.id).unwrap() == events);
        let work=store.execute_external(&work.id,"task.add",&request("task",work.revision,serde_json::json!({"id":"task","title":"Observe todo","deliverable":"Report","criteria":["No acceptance"]})),&mut SystemEnvironment).unwrap().result.work;
        let events = store.events(&work.id).unwrap();
        let observation = store
            .mission_status_historical(&work.id, "operator-a")
            .unwrap();
        assert_eq!(observation.status.total, 1);
        let task = &observation.status.tasks.as_ref().unwrap()[0];
        assert_eq!(task.id, "task");
        assert_eq!(task.state, "configure");
        assert_eq!(task.action, "configure");
        assert_eq!(task.result.state, "not_started");
        assert!(task.waits.as_ref().unwrap().is_empty());
        assert_eq!(observation.status.validated, 0);
        assert_eq!(observation.status.active_agents, 0);
        assert!(store.read_work(&work.id).unwrap() == work);
        assert!(store.events(&work.id).unwrap() == events);
        assert!(store.settings().unwrap() == settings);
        assert!(store.list_agents(&work.id).unwrap().is_empty());
        store.close().unwrap();
    }
    /// Actual owned Root/settings supply the valid missing-profile control.
    /// Supplied profile/control variants are classification probes, not persisted
    /// permissions. Pause/manual modes consume no assist/workspace grant; an
    /// automatic ordinary mode observes readiness, while missing cost policy and
    /// contradictory metadata stay refused.
    #[test]
    fn mission_status_profile_manual_pause_automatic_and_policy_refusals() {
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0,&mut SystemEnvironment).unwrap();
        let mut work=store.execute_external("","work.create",&request("create-profile",0,serde_json::json!({"title":"Profile observation","objective":"Read saved data","scope":"Owned fixture","criteria":["No grant"]})),&mut SystemEnvironment).unwrap().result.work;
        work=store.execute_external(&work.id,"task.add",&request("add-profile",work.revision,serde_json::json!({"id":"task","title":"Observe profile","deliverable":"Report","criteria":["No mutation"]})),&mut SystemEnvironment).unwrap().result.work;
        let mut settings=store.settings().unwrap();
        let mut controls=store.read_mission_controls(&work.id).unwrap();
        let mut causes=Vec::new();
        let read=|work:&crate::Work,controls:&super::super::MissionControlsObservation,settings:&crate::EngineSettingsDocument,dependencies:&[DependencyObservation],causes:&mut Vec<E>|readiness(&store,work,&work.tasks.as_ref().unwrap()[0],controls,settings,dependencies,causes);
        assert!(matches!(read(&work,&controls,&settings,&[],&mut causes).unwrap(),ReadinessObservation::Observed {profile:ProfileObservation::Missing,..}));
        work.launch_profile=Some(Box::default()); // Stored-presence probe, not validated provider health.
        super::super::mission_status_reports::supported(&work,&[]).unwrap();
        assert!(matches!(read(&work,&controls,&settings,&[],&mut causes).unwrap(),ReadinessObservation::Observed {disposition:DispatchDisposition::Manual,profile:ProfileObservation::Configured,..}));
        controls.controls=Some(super::super::MissionControlSettings {autonomy:super::super::MissionAutonomy::Automatic,paused:true,slots:1});
        assert!(matches!(read(&work,&controls,&settings,&[],&mut causes).unwrap(),ReadinessObservation::Observed {disposition:DispatchDisposition::Waiting,..}));
        controls.controls.as_mut().unwrap().paused=false;
        assert!(matches!(read(&work,&controls,&settings,&[],&mut causes).unwrap(),ReadinessObservation::Observed {disposition:DispatchDisposition::Ready,assistance:AssistObservation::Allowed,..}));
        controls.controls.as_mut().unwrap().autonomy=super::super::MissionAutonomy::Assisted;
        assert!(matches!(read(&work,&controls,&settings,&[],&mut causes).unwrap(),ReadinessObservation::Observed {disposition:DispatchDisposition::Manual,..}));
        settings.values.mission_dispatch_cost_reserve_percent=None;
        assert!(read(&work,&controls,&settings,&[],&mut causes).is_err());
        let dependencies=[DependencyObservation {task_id:"parent".into(),freshness:DependencyFreshness::NotCurrent}];
        assert!(matches!(read(&work,&controls,&settings,&dependencies,&mut causes).unwrap(),ReadinessObservation::NotObserved));
        work.tasks.as_mut().unwrap()[0].planning_retry=true;
        assert!(super::super::mission_status_reports::supported(&work,&[]).is_err());
        assert!(causes.is_empty());
        store.close().unwrap();
    }
    /// Public decisions must stay visible without inventing execution or acceptance.
    /// A valid todo control precedes abandonment; every read preserves Work,
    /// settings, events and the empty Agent history. Public reopening recovers
    /// configure, while pure guard probes retain unsupported-state refusals.
    #[test]
    fn mission_status_historical_abandoned_no_agent_and_reopen() {
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
        let work = store.execute_external("", "work.create", &request("create", 0,
            serde_json::json!({"title":"Decision history", "objective":"Observe decisions", "scope":"Owned fixture", "criteria":["No inferred acceptance"]})), &mut SystemEnvironment).unwrap().result.work;
        let work = store.execute_external(&work.id, "task.add", &request("task", work.revision,
            serde_json::json!({"id":"task", "title":"Explicit decision", "deliverable":"Report", "criteria":["No mutation"]})), &mut SystemEnvironment).unwrap().result.work;
        let control = store.mission_status_historical(&work.id, "operator-a").unwrap();
        assert_eq!(control.status.tasks.as_ref().unwrap()[0].state, "configure");
        let settings = store.settings().unwrap();
        let abandoned = store.execute_external(&work.id, "task.update", &request("abandon", work.revision,
            serde_json::json!({"id":"task", "status":"abandoned", "next":"Explicitly retained decision"})), &mut SystemEnvironment).unwrap().result.work;
        let events = store.events(&work.id).unwrap();
        let observed = store.mission_status_historical(&work.id, "operator-a").unwrap();
        let task = &observed.status.tasks.as_ref().unwrap()[0];
        assert_eq!(task.state, "abandoned");
        assert_eq!(task.action, "inspect");
        assert_eq!(task.result.state, "abandoned");
        assert_eq!(task.result.validation_state, "abandoned");
        assert_eq!(task.result.process_state, "not_started");
        assert_eq!(observed.status.validated, 0);
        assert_eq!(observed.status.review, 0);
        assert_eq!(observed.status.running, 0);
        assert_eq!(observed.status.active_agents, 0);
        assert!(store.read_work(&work.id).unwrap() == abandoned);
        assert!(store.settings().unwrap() == settings);
        assert!(store.events(&work.id).unwrap() == events);
        assert!(store.list_agents(&work.id).unwrap().is_empty());
        // Structural refusals are unit probes, not persisted authority fixtures.
        // Accepted is now supported, but its real freshness must still be read.
        for unsupported in ["waived", "blocked"] {
            let mut candidate = abandoned.clone();
            candidate.tasks.as_mut().unwrap()[0].status = unsupported.into();
            assert!(super::super::mission_status_reports::supported(&candidate, &[]).is_err());
        }
        for state in ["completed", "failed", "interrupted", "running"] {
            let agent = crate::Agent { task_id: "task".into(), status: state.into(), ..Default::default() };
            let observed = super::super::mission_status_reports::supported(&abandoned, &[agent]);
            if state == "running" { assert!(observed.is_err()); } else { observed.unwrap(); }
        }
        let mut candidate = abandoned.clone();
        candidate.tasks.as_mut().unwrap()[0].planning_retry = true;
        assert!(super::super::mission_status_reports::supported(&candidate, &[]).is_err());
        assert!(super::super::mission_status_reports::supported(&abandoned, &[]).is_ok());
        let reopened = store.execute_external(&work.id, "task.update", &request("reopen", abandoned.revision,
            serde_json::json!({"id":"task", "status":"todo"})), &mut SystemEnvironment).unwrap().result.work;
        let events = store.events(&work.id).unwrap();
        let recovered = store.mission_status_historical(&work.id, "operator-a").unwrap();
        let task = &recovered.status.tasks.as_ref().unwrap()[0];
        assert_eq!(task.state, "configure");
        assert_eq!(task.result.state, "not_started");
        assert_eq!(recovered.status.validated, 0);
        assert!(store.read_work(&work.id).unwrap() == reopened);
        assert!(store.settings().unwrap() == settings);
        assert!(store.events(&work.id).unwrap() == events);
        assert!(store.list_agents(&work.id).unwrap().is_empty());
        store.close().unwrap();
    }

}
