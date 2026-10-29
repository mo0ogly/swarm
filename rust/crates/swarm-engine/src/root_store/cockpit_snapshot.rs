//! Cockpit snapshot user slice: the first public `agent list` projection.
//!
//! Deliberately narrow slice of the legacy Go `Store.cockpitSnapshot`
//! document in `internal/engine/console.go`, restricted to one ordinary
//! existing Work that has no planning state and no retained Agent, and whose
//! tasks all sit in the supported initial todo shape: status `todo`, empty
//! attempt history and no gate, independent-review,
//! previous-review or revalidation record. This is not the full agent list:
//! planning scopes, retained agent views, later task lifecycle states,
//! review/recovery history and every capability beyond that refuse with an
//! explicit unported code instead of emitting invented rows.
//!
//! The document keeps exactly the 14 legacy keys `post_delivery`,
//! `independent_reviews`, `pilotage`, `work`, `validation`, `agents`,
//! `paused`, `autonomy`, `autonomy_label`, `slots`, `priority`,
//! `task_actions`, `cost` and `cost_text`. Nested map entries are inserted
//! in stable map order; this is JSON value compatibility, not a claim of
//! byte-identical formatting for nested structs or float spellings.
//!
//! Read-only business observation: no Work, settings, event or agent mutation,
//! launch, consent inference or acceptance. Reservation/descriptor bookkeeping
//! follows the existing material reader. Empty `agents`/`health` reflect actual
//! no-agent observations; `edges` mirrors the validated TODO dependencies;
//! `priority` holds only actually persisted rows; per-task `ready`, reasons
//! and action lists are preliminary UI mirrors of store admission predicates,
//! never authority; unsupported task states refuse explicitly.

use std::collections::BTreeMap;
use std::os::unix::ffi::OsStrExt;

use sha2::{Digest, Sha256};

use super::cockpit_actions::{Facts, Observation, project};
use super::{finished, material_path, RootStore, RootStoreError};

/// Wrap a mission producer cause for this Root projection.
/// `MissionSupervisionError` is never handed to a generic Root error helper
/// directly; it travels through the existing mutation-domain error wrapper so its typed
/// history stays inspectable at the CLI boundary.
fn mission_cause(cause: crate::MissionSupervisionError) -> RootStoreError {
    RootStoreError::Mutation(crate::WorkEngineError::Mission(Box::new(cause)))
}

/// Encode an already decoded and validated model into the document.
/// Serialization failures retain the original typed cause through the existing
/// runtime error wrapper; no failed value becomes an empty successful field.
fn model_value<T: serde::Serialize>(model: &T) -> Result<serde_json::Value, RootStoreError> {
    serde_json::to_value(model).map_err(|cause| {
        RootStoreError::Runtime(crate::RuntimeError::new(
            crate::RuntimeOperation::Platform,
            cause,
        ))
    })
}

/// Exact Go `autonomyLabel` vocabulary for the observed saved token. The
/// token is observed data from the closed settings/controls vocabulary,
/// never a permission level; unknown tokens are echoed unchanged like Go.
fn autonomy_label(token: &str) -> &str {
    match token {
        "manuel" => "Manuel — départs et relais humains",
        "assiste" => "Assisté — relais automatique du handoff prouvé",
        "autonome" => "Autonome — départs automatiques dans les créneaux",
        other => other,
    }
}

/// SHA-256 of the opened Root path's raw Unix OsStr bytes, the direct
/// analogue of the Go `hash([]byte(s.root))` project view key. This binds
/// the view to the observed Root pathname only; no authority follows.
fn project_view_key(root: &RootStore) -> String {
    format!("{:x}", Sha256::digest(root.root.as_os_str().as_bytes()))
}

impl RootStore {
    /// Return the legacy fourteen-key cockpit JSON for an existing ordinary
    /// Work with no planning state, no retained agents and every task in the
    /// supported initial todo shape: status `todo`, empty attempts, validated TODO
    /// dependencies and no gate, independent-review, previous-review or
    /// revalidation record. This slice is not the complete agent list. Unknown
    /// Work retains its original typed cause; unsupported task shapes,
    /// planning and retained agents refuse with their explicit
    /// `cockpit-*-projection-unported` code rather than invented state.
    ///
    /// Uses actual saved controls/default autonomy, saved slot policy, complete
    /// costs, the actual saved budget admission state, persisted cockpit-task
    /// priority rows and French work validation. Missing autonomy, malformed
    /// data, out-of-policy slots, SQL/encoding/material/release errors refuse
    /// with typed causes. Empty maps/counts are observed cardinalities; false
    /// paused means no saved pause intent and never implies consent. Per-task
    /// `ready`, start reasons and action lists are preliminary UI mirrors of
    /// the store's admission predicates, never operation admission: any
    /// launcher still rechecks the actual providers, workspace, budget and
    /// identity in its own authoritative transaction.
    ///
    /// The material/settings reservation stabilizes cooperative settings
    /// publication and checks opened Root association before/after. SQL readers
    /// own separate transactions; Work/settings and producer revisions are
    /// rechecked and mismatches refuse. This is not an atomic filesystem/SQL
    /// snapshot, protection against noncooperative writers, a later-action lease
    /// or process/identity authority. The clock is observed, not authenticated.
    ///
    /// No Work, settings, event or agent mutation, provider, signal or acceptance
    /// is requested. Root locking/descriptor bookkeeping uses existing helpers.
    /// Callers still owe Root close and output/flush handling. The Linux CLI
    /// adapter validates its transport before opening; direct library callers
    /// receive the same authoritative Work/Root checks.
    pub fn inspect_cockpit(&self, work: &str) -> Result<serde_json::Value, RootStoreError> {
        material_path::with_material_read(self, || self.cockpit_observation(work))
    }

    /// Body of `inspect_cockpit` inside the held material read; private to
    /// this slice. Producer order is fixed so the cheapest declared refusals
    /// (the supported-task-shape walk and the unported planning/agent
    /// capabilities) precede the reserved policy, validation, cost, budget and
    /// priority reads.
    ///
    /// Task shape contract: every persisted task must sit in the initial todo
    /// state — status `todo`, empty attempt history, a valid dependency DAG of these TODO tasks, and
    /// no gate, independent-review, previous-review or revalidation record.
    /// Other task states, evidence or history refuse with
    /// `cockpit-task-projection-unported`. Invalid dependency DAGs retain the
    /// shared order validator refusal before any task projection is assembled. Fields this slice never consumes (planning inputs, model,
    /// memory or prompt data, ...) stay serialized as the actual stored data:
    /// they are neither refused nor erased.
    ///
    /// Per task, `can_start` is the actual mirror of the Go admission
    /// predicates observable in this slice: not launch-held, saved mission
    /// controls not paused, `plan_brief_hash` matching the Work's planning
    /// brief, and the actual saved budget state (`None` or a zero limit is
    /// the existing unlimited contract; otherwise
    /// `limit - committed >= reserve`, where the Native committed figure
    /// already includes every ledger and reservation and is never reset or
    /// ignored). Unsatisfied TODO dependencies deny readiness. The todo status, the empty retained
    /// agent set and a positive `plan_max_attempts` that zero attempts cannot
    /// exhaust make the remaining Go checks vacuous here. When a start is
    /// refused, the exact Go `startBlockReason` precedence is reproduced (see
    /// [`RootStore::initial_start_block_reason`]); otherwise
    /// `start_block_reason` stays unobserved instead of guessed. Review,
    /// gate-file, repair, last-finished, extension and recovery facts stay
    /// unobserved because the eligible todo branch never consumes them: the
    /// Go recommendation for a todo task is fixed to `start` and the Go
    /// extension/recovery reasons are necessarily nonempty for a non-blocked
    /// task — no absent gate file, passed review or empty history is ever
    /// claimed without observation. `prepared` is observed as `None` because
    /// `w.planning == None` is proven, exactly where the Go
    /// `preparedLaunchForTask` returns before any SQL or filesystem access;
    /// `override_dependency_reason` is observed from the first saved dependency,
    /// or as `None` only when that slice is actually empty. A missing observation needed by the
    /// pure projection refuses with its typed cause.
    ///
    /// `pilotage.tasks` carries the actual per-task delivery entry from the
    /// same work-validation observation used for the top-level document (a
    /// task missing from it refuses — no null fallback), a complete
    /// `waiting_on` (all dependencies are TODO and not accepted-fresh), and the
    /// native attempt projection for an absent attempt: empty agent/attempt
    /// fields, `validation_state` set to the actual task status and
    /// `cost_state: "unknown"`, with the struct's own omission semantics.
    /// Empty `agents`/`health` reflect the actual no-agent observation; edges
    /// retain saved task/dependency order, revision and the snapshot clock.
    ///
    /// Priority contains only actually persisted `cockpit_tasks` rows, read
    /// through the shared typed reader and validated against the Work graph
    /// by the existing `ordered` contract; no per-task neutral default is
    /// synthesized and orphan rows refuse. Changed-observation fencing
    /// rereads the Work and the settings revision and compares every
    /// producer-embedded revision with the opening observation before
    /// anything is emitted. The reads still happen in separate SQL
    /// transactions and statements: this is not an atomic SQL/filesystem
    /// snapshot, protection against noncooperative writers, a later-action
    /// lease or process/identity authority. The clock is observed, not
    /// authenticated.
    ///
    /// No Work, settings, event or agent mutation, provider, signal or
    /// acceptance is requested. Callers still owe Root close and output/flush
    /// handling.
    fn cockpit_observation(&self, work: &str) -> Result<serde_json::Value, RootStoreError> {
        let saved = self.read_work(work)?;
        let tasks = saved.tasks.as_deref().unwrap_or(&[]);
        // Supported initial task shape: reject unsupported task state,
        // evidence and history before anything further is read
        // or emitted. One declared code covers every unsupported shape.
        for task in tasks {
            if task.status != "todo"
                || task.attempts.as_ref().is_some_and(|attempts| !attempts.is_empty())
                || task.gate.is_some()
                || task.independent_review.is_some()
                || task.previous_reviews.as_ref().is_some_and(|reviews| !reviews.is_empty())
                || task.revalidation.is_some()
            {
                return Err(RootStoreError::Refused("cockpit-task-projection-unported"));
            }
        }
        if saved.planning.is_some() {
            return Err(RootStoreError::Refused(
                "cockpit-planning-projection-unported",
            ));
        }
        if !self.list_agents(work)?.is_empty() {
            return Err(RootStoreError::Refused("cockpit-agent-projection-unported"));
        }
        let settings_revision = self.read_settings()?.revision;
        let dispatch = self
            .read_mission_dispatch_controls(work)
            .map_err(mission_cause)?;
        let autonomy = self.read_cockpit_autonomy(work).map_err(mission_cause)?;
        let autonomy_token = match autonomy.autonomy.as_deref() {
            Some(token) => token,
            None => return Err(RootStoreError::Refused("cockpit-autonomy-not-configured")),
        };
        let validation = self.inspect_work_validation(work, crate::WorkViewLanguage::French)?;
        let costs = self.reported_costs(work)?;
        // The budget admission question exists only when a task exists; the
        // no-task observation is emitted without reading the budget at all.
        // `None` is the existing no-record unlimited contract and the Native
        // committed figure already includes every ledger and reservation.
        let budget_allows = if tasks.is_empty() {
            true
        } else {
            match self.work_budget_state(work)? {
                None => true,
                Some(state) => {
                    state.limit_usd == 0.0
                        || state.limit_usd - state.committed_usd
                            >= state.reserve_per_launch_usd
                }
            }
        };
        let paused = dispatch
            .controls
            .controls
            .as_ref()
            .is_some_and(|saved_controls| saved_controls.paused);
        // Validate the complete saved DAG and raw priority keys before any
        // branch derives readiness, reasons or graph facts. Every eligible
        // task is TODO, so each dependency is observed not accepted-fresh.
        let priorities =
            super::mission_task_order::read_priorities(&self.db, work).map_err(mission_cause)?;
        let order =
            super::mission_task_order::ordered(&saved, &priorities).map_err(mission_cause)?;
        let snapshot_as_of = crate::system_timestamp().map_err(RootStoreError::Runtime)?;
        let mut edges = Vec::<serde_json::Value>::new();
        let mut pilotage_tasks = BTreeMap::<&str, serde_json::Value>::new();
        let mut task_actions = BTreeMap::<&str, serde_json::Value>::new();
        for task in tasks {
            let brief_ok = task.plan_brief_hash.is_empty()
                || saved
                    .planning_brief
                    .as_ref()
                    .is_some_and(|brief| brief.sha256 == task.plan_brief_hash);
            let dependencies = task.depends.as_deref().unwrap_or(&[]);
            let can_start = !task.launch_held && !paused && dependencies.is_empty()
                && brief_ok && budget_allows;
            for dependency in dependencies {
                edges.push(serde_json::json!({
                    "from_task_id": dependency,
                    "to_task_id": task.id,
                    "satisfied_now": false,
                    "reason_code": "dependency_waiting",
                    "reason_label": "Dépendance non validée ou périmée",
                    "evaluated_revision": saved.revision,
                    "evaluated_at": snapshot_as_of,
                }));
            }
            // The Go reason is consulted exactly when the start is refused;
            // otherwise the fact stays unobserved instead of guessed.
            let start_block_reason = if can_start {
                None
            } else {
                Some(self.initial_start_block_reason(task, paused, brief_ok, budget_allows)?)
            };
            let facts = Facts {
                can_start: Observation::Observed(can_start),
                start_block_reason: match start_block_reason.as_deref() {
                    Some(reason) => Observation::Observed(reason.to_owned()),
                    None => Observation::Unobserved,
                },
                gate_files_present: Observation::Unobserved,
                gate_current: Observation::Unobserved,
                independent_review_reason: Observation::Unobserved,
                // All dependencies are validated existing TODO tasks, hence
                // not accepted-fresh; preserve the saved first reason.
                override_dependency_reason: Observation::Observed(dependencies.first().map(|id| {
                    format!("Acceptez d'abord la dépendance {id} (revue ou dérogation).")
                })),
                legacy_submission_repairable: Observation::Unobserved,
                last_finished_requires_environment_verification: Observation::Unobserved,
                // `w.planning == None` is proven, exactly where the Go
                // `preparedLaunchForTask` returns before SQL/filesystem.
                prepared: Observation::Observed(None),
                show_attempt_extension: Observation::Unobserved,
                show_corrective_recovery: Observation::Unobserved,
                attempt_extension_reason: Observation::Unobserved,
                corrective_recovery_reason: Observation::Unobserved,
            };
            let actions = project(task, &[], facts)?;
            // Native attempt projection for an absent attempt: empty
            // agent/attempt fields, the actual task status and the explicit
            // unknown cost state; the struct's omission semantics apply.
            let attempt = crate::mission_status_contract::MissionAttemptProjection {
                agent_id: String::new(),
                attempt_id: String::new(),
                role: String::new(),
                process_state: String::new(),
                activity: String::new(),
                validation: task.status.clone(),
                requested_model: String::new(),
                observed_model: String::new(),
                cost_state: "unknown".to_owned(),
            };
            // Same actual validation observation as the top-level document;
            // a task missing from it refuses instead of degrading to null.
            let delivery = validation
                .validation
                .tasks
                .get(task.id.as_str())
                .ok_or(RootStoreError::Refused("cockpit-snapshot-validation-task-missing"))?;
            let attempt_value = model_value(&attempt)?;
            let delivery_value = model_value(delivery)?;
            pilotage_tasks.insert(
                task.id.as_str(),
                serde_json::json!({
                    "attempt": attempt_value,
                    "delivery": delivery_value,
                    "ready": can_start,
                    // The full validated dependency slice stays pending,
                    // preserving saved order; no acceptance is inferred.
                    "waiting_on": dependencies,
                }),
            );
            task_actions.insert(task.id.as_str(), model_value(&actions)?);
        }
        let priority_value = model_value(&priorities)?;
        let pilotage_tasks_value = model_value(&pilotage_tasks)?;
        let task_actions_value = model_value(&task_actions)?;
        // Changed-observation fencing: reread the Work and the settings
        // revision, and compare every producer-embedded revision with the
        // opening observation, before anything is emitted.
        let work_after = self.read_work(work)?;
        if work_after.revision != saved.revision {
            return Err(RootStoreError::Refused("cockpit-snapshot-work-changed"));
        }
        let settings_after = self.read_settings()?;
        if settings_after.revision != settings_revision
            || dispatch.settings_revision != settings_revision
            || autonomy.settings_revision != settings_revision
            || validation.settings_revision != settings_revision
        {
            return Err(RootStoreError::Refused("cockpit-snapshot-settings-changed"));
        }
        if validation.work_revision != saved.revision || order.revision != saved.revision {
            return Err(RootStoreError::Refused("cockpit-snapshot-work-changed"));
        }
        let work_value = model_value(&saved)?;
        let validation_value = model_value(&validation.validation)?;
        let cost_value = model_value(&costs)?;
        // Empty collections and zero counts follow the observed Work and the
        // actual no-agent/validated TODO-dependency observations; JSON value
        // compatibility does not imply byte-identical formatting.
        Ok(serde_json::json!({
            "agents": [],
            "autonomy": autonomy_token,
            "autonomy_label": autonomy_label(autonomy_token),
            "cost": cost_value,
            "cost_text": costs.total.text(),
            "independent_reviews": {},
            "paused": paused,
            "pilotage": {
                "edges": edges,
                "health": {},
                "project_view_key": project_view_key(self),
                "schema_version": 1,
                "snapshot_as_of": snapshot_as_of,
                "summary": {
                    "complete": true,
                    "confirmed": 0,
                    "loaded": 0,
                    "occupied": 0,
                    "total": 0,
                    "unknown": 0,
                },
                "tasks": pilotage_tasks_value,
            },
            // A todo task is never delivery-ready and `sources` stays an
            // actual empty array, never null.
            "post_delivery": {"ready": false, "sources": []},
            "priority": priority_value,
            "slots": dispatch.slots.0,
            "task_actions": task_actions_value,
            "validation": validation_value,
            "work": work_value,
        }))
    }

    /// Go `startBlockReason` for one eligible todo task of this slice,
    /// reproducing the exact Go precedence: held launch, paused saved
    /// controls, the first unsatisfied TODO dependency,
    /// plan-brief mismatch, the plan attempt cap, the global active workspace
    /// overlap, the budget, then the Go generic tail.
    ///
    /// The attempt-cap reason is unreachable in this slice and is therefore
    /// not observed: with zero retained attempts a positive
    /// `plan_max_attempts` cannot be exhausted (`0 < N`), a nonpositive cap
    /// skips the Go guard, and both readings of the Go execution-observed
    /// short-circuit leave the reason empty. No attempt-count constant is
    /// reintroduced.
    ///
    /// The global overlap count is queried only when the budget denies and no
    /// earlier reason applied — exactly where the Go precedence needs it —
    /// through [`RootStore::active_workspace_overlap`]; its SQL, finalization
    /// and Root-path-encoding refusals propagate and are never folded into a
    /// reason or a zero count. A budget read failure has already refused the
    /// whole projection before this runs, so no error is stringified into a
    /// reason. The generic Go tail is kept for totality; on the supported
    /// todo/no-agent shape it is unreachable because the only remaining
    /// denial cause is the budget. The returned text is a preliminary UI
    /// explanation, never a permission decision or a lease: any launcher
    /// rechecks the actual providers, workspace, budget and identity.
    fn initial_start_block_reason(
        &self,
        task: &crate::Task,
        paused: bool,
        brief_ok: bool,
        budget_allows: bool,
    ) -> Result<String, RootStoreError> {
        if task.launch_held {
            return Ok(
                "Démarrage non autorisé : ouvrir la préparation puis autoriser les missions de ce plan."
                    .to_owned(),
            );
        }
        if paused {
            return Ok(
                "Départs suspendus : réautoriez les départs depuis l'en-tête du cockpit.".to_owned(),
            );
        }
        if let Some(dependency) = task.depends.as_ref().and_then(|ids| ids.first()) {
            return Ok(format!("Dépendance non validée ou périmée : {dependency}."));
        }
        if !brief_ok {
            return Ok("Brief du plan modifié : relisez le brief avant de lancer.".to_owned());
        }
        if !budget_allows {
            if self.active_workspace_overlap()? > 0 {
                return Ok("Un agent occupe déjà cet espace de travail.".to_owned());
            }
            return Ok("Budget estimatif insuffisant pour un nouveau départ.".to_owned());
        }
        Ok("Conditions de départ non réunies (dépendances, budget ou espace de travail).".to_owned())
    }

    /// Count the active agent rows whose recorded working directory overlaps
    /// the opened Root path, reproducing the Go global query exactly: the
    /// same active status set and the same cwd predicate (`cwd` equal to,
    /// under, or containing the Root path). Consumed only to keep the Go
    /// reason precedence (workspace overlap before budget) when a start is
    /// refused.
    ///
    /// The Root path is bound as UTF-8 text without any lossy conversion; a
    /// non-UTF-8 Root path refuses with an explicit unported code instead of
    /// a rewritten or truncated path. One statement on the opened connection
    /// inside the caller's material-read context: SQL and finalization
    /// failures propagate with typed causes through the shared finish helper
    /// and are never converted into a zero count. This is one observation at
    /// one instant — not a workspace lease, not an atomic snapshot and not a
    /// launch permission.
    fn active_workspace_overlap(&self) -> Result<i64, RootStoreError> {
        let root = std::str::from_utf8(self.root.as_os_str().as_bytes())
            .map_err(|_| RootStoreError::Refused("cockpit-snapshot-root-encoding-unported"))?;
        let mut statement = self
            .db
            .prepare(
                "SELECT count(*) FROM agents WHERE status IN ('queued','starting','running','stopping') AND (cwd=? OR instr(cwd, ? || '/')=1 OR instr(?, cwd || '/')=1)",
            )
            .map_err(RootStoreError::Sql)?;
        let result = (|| {
            let mut rows = statement
                .query(rusqlite::params![root, root, root])
                .map_err(RootStoreError::Sql)?;
            let row = rows
                .next()
                .map_err(RootStoreError::Sql)?
                .ok_or(RootStoreError::Refused("cockpit-snapshot-overlap-count-missing"))?;
            row.get::<_, i64>(0).map_err(RootStoreError::Sql)
        })();
        finished(result, statement.finalize().map_err(RootStoreError::Sql))
    }
}
