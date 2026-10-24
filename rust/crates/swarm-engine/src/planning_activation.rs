//! Root-owned operator scope claims. A logical holder is not an agent identity.
use crate::{AgentLaunchError as E, PlanningRequest, RoleModel, Work};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Reject fields whose presence would imply an unapplied configuration/decision.
/// Canonical omitempty semantics come from the actual request serializer. Only
/// this protocol whitelist is fixed; timing/volume policies come from settings.
pub(crate) fn validate_request(request: &PlanningRequest) -> Result<(), E> {
    if request.schema_version != 1 {
        return Err(E::Refused("planning-schema-required"));
    }
    let raw = crate::encode_planning_request(request)
        .map_err(|e| E::caused("planning-claim-request-encoding", e))?;
    let fields: BTreeMap<String, serde_json::Value> = serde_json::from_slice(&raw)
        .map_err(|e| E::caused("planning-claim-request-encoding", e))?;
    if fields.keys().any(|k| {
        !matches!(
            k.as_str(),
            "schema_version"
                | "event_id"
                | "expected_revision"
                | "scope"
                | "scope_revision"
                | "holder"
                | "lease_seconds"
        )
    }) {
        return Err(E::Refused("planning-claim-field-not-applicable"));
    }
    if !swarm_identity_contract::name(request.holder.as_bytes())
        || !swarm_identity_contract::name(request.scope.as_bytes())
    {
        return Err(E::Refused("planning-claim-holder-or-scope-invalid"));
    }
    Ok(())
}

/// Reobserve the inherited role's actual source under a provider reservation.
/// Refuse digest or logical model changes before spending a scope activation.
/// Readonly preparation is shared with configuration; no process, account health,
/// cooldown clearance, external confinement or execution permission is proved.
#[cfg(target_os = "linux")]
pub(crate) fn observe_model(
    root: &Path,
    work: &Work,
    scope: &str,
    settings: &crate::EngineSettingsDocument,
    at: &str,
) -> Result<RoleModel, E> {
    let p = work
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    let inherited = crate::planning_scope_ancestors(work, scope)
        .map_err(|e| E::caused("planning-claim-ancestors-invalid", e))?;
    let selected = inherited.iter().find_map(|s| s.model_selection.as_deref());
    let mut model = selected.cloned().unwrap_or_else(|| RoleModel {
        provider: p.provider.clone(),
        provider_digest: p.provider_digest.clone(),
        route: p.model_route.clone(),
        ..Default::default()
    });
    model.actor.clear();
    model.at = at.into();
    if model.provider.is_empty() {
        if !model.provider_digest.is_empty() || model.route.is_some() {
            return Err(E::Refused("planning-claim-local-model-inconsistent"));
        }
        return Ok(model);
    }
    if model.provider_digest.is_empty() {
        return Err(E::Refused("planning-claim-provider-digest-required"));
    }
    // Claim is an admitted activation, unlike metadata-only enablement. Use the
    // selected inherited provider, not a sibling/global fallback; source cooldown
    // expiry establishes no provider health or permission for an actual process.
    crate::agent_launch::cooldown(root, &model.provider)?;
    let policy = settings
        .values
        .agent_launch_policy()
        .map_err(|e| E::caused(e.reason(), e))?;
    let directory = crate::local_storage::LocalDirectory::open(&root.join(".swarm"))
        .map_err(|e| E::caused("planning-claim-provider-directory", e))?;
    let result = (|| {
        let reservation = directory
            .reserve_provider_configuration()
            .map_err(|e| E::caused("planning-claim-provider-reservation", e))?;
        let observed = (|| {
            let request = crate::Launch {
                provider: model.provider.clone(),
                level: model
                    .route
                    .as_ref()
                    .map_or_else(String::new, |r| r.level.clone()),
                model_policy_hash: model
                    .route
                    .as_ref()
                    .map_or_else(String::new, |r| r.policy_hash.clone()),
                ..Default::default()
            };
            let (digest, route) = crate::reviewer_bootstrap::observe_provider(
                &directory, &request, settings, &policy,
            )?;
            if digest != model.provider_digest {
                return Err(E::Refused("planning-claim-provider-changed"));
            }
            match (model.route.as_deref(), route.as_deref()) {
                (None, None) => {}
                (Some(old), Some(new))
                    if old.policy_hash == new.policy_hash
                        && old.level == new.level
                        && old.model == new.model
                        && old.effort == new.effort => {}
                _ => return Err(E::Refused("planning-claim-model-changed")),
            }
            model.route = route;
            Ok(model)
        })();
        crate::agent_launch_error::finish(
            observed,
            reservation
                .close()
                .map_err(|e| E::caused("planning-claim-provider-reservation-close", e)),
        )
    })();
    crate::agent_launch_error::finish(
        result,
        directory
            .close()
            .map_err(|e| E::caused("planning-claim-provider-directory-close", e)),
    )
}

/// Recheck a prepared lease against the transaction's final clock observation.
/// Call immediately before staging: expired or malformed deadlines refuse before
/// Work/event/counter publication. This is the requested lease contract, not an
/// additional timeout or a guarantee of monotonic clocks/commit completion time.
/// The surrounding transaction must still preserve source/settings and fence
/// checks, rollback failures and uncertainty after publication separately.
pub(crate) fn validate_publication(work: &Work, scope: &str, at: &str) -> Result<(), E> {
    let p = work
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    let selected = p
        .scopes
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|s| s.id == scope)
        .ok_or(E::Refused("planning-scope-not-found"))?;
    let now = swarm_wire_json::go_rfc3339::parse(at)
        .map_err(|e| E::caused("planning-publication-clock-invalid", e))?;
    let until = swarm_wire_json::go_rfc3339::parse(&selected.lease_until)
        .map_err(|e| E::caused("planning-publication-deadline-invalid", e))?;
    if selected.holder.is_empty() || p.paused || now >= until {
        return Err(E::Refused("planning-claim-expired-before-publication"));
    }
    Ok(())
}

/// Build one new operator claim from current authoritative Work/settings/time.
/// The transaction owner must validate transport, event/revision and replay first,
/// retain its settings reservation, then atomically stage/commit this candidate.
/// Active/malformed leases, paused/failed/closed scopes, unchanged rejection,
/// exhausted ancestor/global budgets, stale sources and projection failures refuse
/// without changing input. Expired claims keep prior consumption and get a new
/// generation. Complete workflow/context snapshots are bound before counters
/// advance; no event is acknowledged and no process or automatic dispatch starts.
/// Managed repositories require their original prepared copy/commit observations;
/// only planning is enabled, not managed workers or source exploration.
/// Checked integer/timestamp limits are technical representation, not extra policy.
#[cfg(target_os = "linux")]
pub(crate) fn prepare_claim(
    root: &Path,
    work: &Work,
    request: &PlanningRequest,
    at: &str,
    settings: &crate::EngineSettingsDocument,
) -> Result<(Work, ClaimInputs), E> {
    validate_request(request)?;
    settings.validate().map_err(|e| E::caused(e.reason(), e))?;
    crate::validate_planning_state(work)
        .map_err(|e| E::caused("planning-claim-state-invalid", e))?;
    let policy = settings
        .values
        .planning_action_policy()
        .map_err(|e| E::caused(e.reason(), e))?;
    policy
        .check_lease(request.lease_seconds)
        .map_err(|e| E::caused(e.reason(), e))?;
    let p = work
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    if p.paused {
        return Err(E::Refused("planning_paused"));
    }
    if !p.failure.is_empty() {
        return Err(E::Refused("planning-stopped"));
    }
    let scopes = p.scopes.as_deref().unwrap_or_default();
    let index = scopes
        .iter()
        .position(|s| s.id == request.scope)
        .ok_or(E::Refused("planning-scope-not-found"))?;
    let scope = &scopes[index];
    if scope.revision != request.scope_revision {
        return Err(E::Refused("revision_conflict"));
    }
    if scope.state == "closed" {
        return Err(E::Refused("planning-scope-closed"));
    }
    if !matches!(scope.state.as_str(), "ready" | "waiting") {
        return Err(E::Refused("planning-scope-state-invalid"));
    }
    if scope.generation < 0 {
        return Err(E::Refused("planning-generation-negative"));
    }
    let generation = scope
        .generation
        .checked_add(1)
        .ok_or(E::Refused("planning-generation-overflow"))?;
    let now = swarm_wire_json::go_rfc3339::parse(at)
        .map_err(|e| E::caused("planning-claim-clock-invalid", e))?;
    if scope.holder.is_empty() != scope.lease_until.is_empty()
        || (!scope.holder.is_empty() && !swarm_identity_contract::name(scope.holder.as_bytes()))
    {
        return Err(E::Refused("planning-scope-lease-inconsistent"));
    }
    if !scope.holder.is_empty() {
        let until = swarm_wire_json::go_rfc3339::parse(&scope.lease_until)
            .map_err(|e| E::caused("planning-scope-lease-invalid", e))?;
        if now < until {
            return Err(E::Refused("lease_busy"));
        }
    }
    let until = now
        .checked_add(time::Duration::seconds(request.lease_seconds))
        .ok_or(E::Refused("planning-lease-timestamp-representation"))?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| E::caused("planning-lease-timestamp-format", e))?;
    let at = now
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| E::caused("planning-claim-clock-format", e))?;
    if crate::planning_rejection_unchanged(work, &scope.id)
        .map_err(|e| E::caused("planning-rejection-state-invalid", e))?
    {
        return Err(E::Refused("rejection_requires_change"));
    }
    crate::check_planning_activation_budget(work, &scope.id).map_err(|e| E::caused(e.code(), e))?;
    let ancestors: BTreeSet<String> = crate::planning_scope_ancestors(work, &scope.id)
        .map_err(|e| E::caused("planning-claim-ancestors-invalid", e))?
        .iter()
        .map(|s| s.id.clone())
        .collect();
    let body_cap = settings
        .values
        .agent_workflow_body_limit()
        .map_err(|e| E::caused(e.reason(), e))?;
    let limits = settings
        .values
        .context_policy_limits()
        .map_err(|e| E::caused(e.reason(), e))?;
    let workflow = crate::read_agent_workflow(
        root,
        if scope.parent.is_empty() {
            "planner"
        } else {
            "subplanner"
        },
        body_cap,
        &limits,
    )
    .map_err(|e| E::caused("planning-claim-workflow-invalid", e))?;
    if p.repository.is_some() {
        crate::managed_repository::observe(root,work,settings)?;
    }
    let context = crate::planning_scope_context::observe(root, work, scope, settings)?;
    let total = workflow
        .prompt
        .len()
        .checked_add(context.bytes.len())
        .ok_or(E::Refused("planning-prompt-size-representation"))?;
    let launch_policy = settings
        .values
        .agent_launch_policy()
        .map_err(|e| E::caused(e.reason(), e))?;
    if launch_policy.context_bytes.is_some_and(|max| total > max) {
        return Err(E::Refused("planning-prompt-byte-limit-exceeded"));
    }
    let model = observe_model(root, work, &scope.id, settings, &at)?;
    let mut candidate = work.clone();
    let p = candidate
        .planning
        .as_mut()
        .ok_or(E::Refused("planning-not-enabled"))?;
    p.activations = p
        .activations
        .checked_add(1)
        .ok_or(E::Refused("planning-activation-overflow"))?;
    for s in p
        .scopes
        .iter_mut()
        .flatten()
        .filter(|s| ancestors.contains(&s.id))
    {
        s.activations = s
            .activations
            .checked_add(1)
            .ok_or(E::Refused("planning-activation-overflow"))?;
    }
    let scope = p
        .scopes
        .as_mut()
        .and_then(|s| s.get_mut(index))
        .ok_or(E::Refused("planning-scope-not-found"))?;
    scope.generation = generation;
    scope.holder = request.holder.clone();
    scope.lease_until = until;
    scope.workflow = Some(Box::new(workflow.workflow));
    scope.delivery = Some(Box::new(context.delivery));
    scope.last_model = Some(Box::new(model));
    crate::validate_planning_state(&candidate)
        .map_err(|e| E::caused("planning-claim-result-invalid", e))?;
    Ok((
        candidate,
        ClaimInputs {
            context: context.bytes,
            workflow_prompt: workflow.prompt,
            workflow_prompt_sha256: workflow.prompt_sha256,
        },
    ))
}

/// Exact pre-consumption input bytes, retained by the owning claim transaction.
/// Internal data only; this is no caller authority or permission to start.
pub(crate) struct ClaimInputs {
    pub context: Vec<u8>,
    pub workflow_prompt: String,
    pub workflow_prompt_sha256: String,
}
/// Compatibility test projection of pure claim preparation. The real Root
/// consumer also stages immutable input binding and the financial reservation.
#[cfg(all(test, target_os = "linux"))]
fn claim(
    root: &Path,
    work: &Work,
    r: &PlanningRequest,
    at: &str,
    settings: &crate::EngineSettingsDocument,
) -> Result<Work, E> {
    prepare_claim(root, work, r, at, settings).map(|(work, _)| work)
}

/// Refresh an active operator-owned scope lease under current saved v24 bounds.
/// Only generation is additionally applicable relative to claim. The original
/// live fence must be rechecked by the transaction at final publication, as must
/// the new deadline; expired/replaced/paused claims cannot be resurrected. No
/// activation, scope revision/generation, workflow, delivery or budget is reset.
/// A changed duration may shorten the deadline; there is no implicit duration,
/// automatic timer, provider call or cryptographic holder authentication here.
pub(crate) fn renew(
    work: &Work,
    request: &PlanningRequest,
    at: &str,
    settings: &crate::EngineSettingsDocument,
) -> Result<Work, E> {
    validate_request(&PlanningRequest {
        generation: 0,
        ..request.clone()
    })?;
    if request.generation < 1 {
        return Err(E::Refused("stale_lease"));
    }
    settings.validate().map_err(|e| E::caused(e.reason(), e))?;
    crate::validate_planning_state(work)
        .map_err(|e| E::caused("planning-renew-state-invalid", e))?;
    crate::planning_decision::fence(work, request, at)?;
    let policy = settings
        .values
        .planning_action_policy()
        .map_err(|e| E::caused(e.reason(), e))?;
    policy
        .check_lease(request.lease_seconds)
        .map_err(|e| E::caused(e.reason(), e))?;
    let now = swarm_wire_json::go_rfc3339::parse(at)
        .map_err(|e| E::caused("planning-renew-clock-invalid", e))?;
    let until = now
        .checked_add(time::Duration::seconds(request.lease_seconds))
        .ok_or(E::Refused("planning-lease-timestamp-representation"))?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| E::caused("planning-lease-timestamp-format", e))?;
    let mut next = work.clone();
    let scope = next
        .planning
        .as_deref_mut()
        .and_then(|p| p.scopes.as_deref_mut())
        .and_then(|scopes| scopes.iter_mut().find(|s| s.id == request.scope))
        .ok_or(E::Refused("planning-scope-not-found"))?;
    scope.lease_until = until;
    crate::validate_planning_state(&next)
        .map_err(|e| E::caused("planning-renew-result-invalid", e))?;
    Ok(next)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::{EngineSettingsDocument, EngineSettingsValues, PlanningControlAction};

    /// Pure Work fixture with an owned empty project root; source/model execution
    /// is absent. Clock observations are supplied explicitly, without real waits.
    struct Fixture {
        root: std::path::PathBuf,
        work: Work,
        settings: EngineSettingsDocument,
    }
    impl Fixture {
        /// Initialize actual current policy history and a valid local-only plan.
        fn new() -> Self {
            use std::os::unix::fs::PermissionsExt;
            let root =
                std::env::temp_dir().join(crate::generate_identifier("scope-claim113-").unwrap());
            std::fs::create_dir(&root).unwrap();
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
            let settings = EngineSettingsDocument::initial(
                EngineSettingsValues::installation_defaults().unwrap(),
                "fixture".into(),
                "2026-10-20T12:00:00Z".into(),
            )
            .unwrap();
            let work = Work {
                schema_version: 1,
                id: "work".into(),
                title: "Root title".into(),
                objective: "Root objective".into(),
                criteria: Some(vec!["Own requirement".into()]),
                ..Default::default()
            };
            let work = crate::apply_planning_control(
                &work,
                PlanningControlAction::Enable,
                &PlanningRequest {
                    schema_version: 1,
                    event_id: "brief".into(),
                    max_tasks: 3,
                    max_activations: 3,
                    max_decisions: 3,
                    ..Default::default()
                },
                "2026-10-20T12:00:00Z",
                &settings,
            )
            .unwrap();
            Self {
                root,
                work,
                settings,
            }
        }
        /// One explicit five-second test request; this is no production default.
        fn request(&self) -> PlanningRequest {
            PlanningRequest {
                schema_version: 1,
                event_id: "claim".into(),
                scope: "root".into(),
                scope_revision: 1,
                holder: "fixture-planner".into(),
                lease_seconds: 5,
                ..Default::default()
            }
        }
    }
    impl Drop for Fixture {
        /// Preserve a failed fixture; successful checks remove only their own root.
        fn drop(&mut self) {
            if !std::thread::panicking() {
                std::fs::remove_dir_all(&self.root).unwrap();
            }
        }
    }

    /// Healthy claim -> busy refusal -> exact expiry permits a new generation;
    /// prior activations stay consumed, exhaustion and pause preserve the input.
    #[test]
    fn scope_claim_busy_expiry_budgets_pause_and_recovery_use_observed_clock() {
        let f = Fixture::new();
        let r = f.request();
        let original = f.work.clone();
        let first = claim(&f.root, &f.work, &r, "2026-10-20T12:00:00Z", &f.settings).unwrap();
        assert_eq!(f.work, original);
        assert_eq!(first.planning.as_ref().unwrap().activations, 1);
        assert_eq!(
            claim(&f.root, &first, &r, "2026-10-20T12:00:04Z", &f.settings)
                .unwrap_err()
                .code(),
            "lease_busy"
        );
        let second = claim(&f.root, &first, &r, "2026-10-20T12:00:05Z", &f.settings).unwrap();
        assert_eq!(
            second.planning.as_ref().unwrap().scopes.as_ref().unwrap()[0].generation,
            2
        );
        assert_eq!(second.planning.as_ref().unwrap().activations, 2);
        let third = claim(&f.root, &second, &r, "2026-10-20T12:00:10Z", &f.settings).unwrap();
        assert_eq!(
            claim(&f.root, &third, &r, "2026-10-20T12:00:15Z", &f.settings)
                .unwrap_err()
                .code(),
            "budget_exhausted"
        );
        assert_eq!(third.planning.as_ref().unwrap().activations, 3);
        let paused = crate::apply_planning_control(
            &first,
            PlanningControlAction::Pause,
            &PlanningRequest {
                schema_version: 1,
                ..Default::default()
            },
            "2026-10-20T12:00:01Z",
            &f.settings,
        )
        .unwrap();
        assert_eq!(
            claim(&f.root, &paused, &r, "2026-10-20T12:00:02Z", &f.settings)
                .unwrap_err()
                .code(),
            "planning_paused"
        );
        assert_eq!(paused.planning.as_ref().unwrap().activations, 1);
        assert!(
            paused.planning.as_ref().unwrap().scopes.as_ref().unwrap()[0]
                .holder
                .is_empty()
        );
    }

    /// Each fault follows a healthy control. Invalid lease data and supplied
    /// decision/config fields refuse; restoration needs no counter reset/sleep.
    #[test]
    fn scope_claim_malformed_fields_generation_and_representation_refuse_safely() {
        let f = Fixture::new();
        let r = f.request();
        let control = || claim(&f.root, &f.work, &r, "2026-10-20T12:00:00Z", &f.settings).unwrap();
        let first = control();
        let mut bad = first.clone();
        bad.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].lease_until =
            "PRIVATE invalid".into();
        let error = claim(&f.root, &bad, &r, "2026-10-20T12:00:01Z", &f.settings).unwrap_err();
        assert_eq!(error.code(), "planning-scope-lease-invalid");
        assert!(!error.to_string().contains("PRIVATE"));
        control();
        let mut bad = f.work.clone();
        bad.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].generation = i64::MAX;
        assert_eq!(
            claim(&f.root, &bad, &r, "2026-10-20T12:00:00Z", &f.settings)
                .unwrap_err()
                .code(),
            "planning-generation-overflow"
        );
        control();
        let mut request = r.clone();
        request.max_tasks = 9;
        assert_eq!(
            claim(
                &f.root,
                &f.work,
                &request,
                "2026-10-20T12:00:00Z",
                &f.settings
            )
            .unwrap_err()
            .code(),
            "planning-claim-field-not-applicable"
        );
        control();
        let mut values = f.settings.values.clone();
        values.planning_scope_lease_max_seconds = Some(crate::SettingCap(None));
        let settings = EngineSettingsDocument::initial(
            values,
            "fixture".into(),
            "2026-10-20T12:00:00Z".into(),
        )
        .unwrap();
        let mut request = r.clone();
        request.lease_seconds = i64::MAX;
        assert_eq!(
            claim(
                &f.root,
                &f.work,
                &request,
                "2026-10-20T12:00:00Z",
                &settings
            )
            .unwrap_err()
            .code(),
            "planning-lease-timestamp-representation"
        );
        control();
    }

    /// Drive the public Root transaction with a supplied preparation/publication
    /// clock. No wall-clock wait or private SQL seed manufactures expiry.
    #[test]
    fn scope_claim_renewal_and_decision_late_clock_refuse_before_publication() {
        use crate::WorkEnvironment;
        struct Clock {
            first: bool,
            initial: &'static str,
            publication: &'static str,
        }
        impl WorkEnvironment for Clock {
            fn identifier(&mut self, prefix: &str) -> Result<String, crate::RuntimeError> {
                crate::generate_identifier(prefix)
            }
            fn timestamp(&mut self) -> Result<String, crate::RuntimeError> {
                let value = if self.first {
                    self.initial
                } else {
                    self.publication
                };
                self.first = false;
                Ok(value.into())
            }
            fn operator(&mut self) -> Result<String, crate::RuntimeError> {
                crate::SystemEnvironment.operator()
            }
            fn git(&mut self, root: &Path) -> Result<crate::GitObservation, crate::RuntimeError> {
                crate::SystemEnvironment.git(root)
            }
        }
        for (clock, expired, decision_publication) in [
            ("2026-10-20T12:00:04Z", false, "2026-10-20T12:00:04Z"),
            ("2026-10-20T12:00:04Z", false, "2026-10-20T12:00:06Z"),
            ("2026-10-20T12:00:06Z", true, ""),
        ] {
            let f = Fixture::new();
            crate::RootStore::initialize(&f.root, &mut crate::SystemEnvironment)
                .unwrap()
                .close()
                .unwrap();
            let mut store = crate::RootStore::open(&f.root, true).unwrap();
            let created=store.execute_external("","work.create",&serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"clock-create","expected_revision":0,"title":"Clock witness","objective":"Observe lease publication","scope":"Owned fixture","criteria":["No expired publication"]})).unwrap(),&mut crate::SystemEnvironment).unwrap().result.work;
            let enabled=store.execute_external(&created.id,"planning.enable",&serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"clock-enable","expected_revision":created.revision,"max_activations":3,"max_tasks":3,"max_decisions":3})).unwrap(),&mut crate::SystemEnvironment).unwrap().result.work;
            let request = PlanningRequest {
                expected_revision: enabled.revision,
                ..f.request()
            };
            let before = store.read_work(&created.id).unwrap();
            let result = store.execute_external(
                &created.id,
                "planning.claim",
                &crate::encode_planning_request(&request).unwrap(),
                &mut Clock {
                    first: true,
                    initial: "2026-10-20T12:00:00Z",
                    publication: clock,
                },
            );
            let after = store.read_work(&created.id).unwrap();
            store.close().unwrap();
            // Read-only ledger observation distinguishes Work rollback from a
            // leaked financial reservation after late prepublication refusal.
            let db = rusqlite::Connection::open_with_flags(
                f.root.join(".swarm/state.db"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            let calls: i64 = db
                .query_row(
                    "SELECT count(*) FROM planning_calls WHERE work_id=?1",
                    [&created.id],
                    |r| r.get(0),
                )
                .unwrap();
            db.close().map_err(|(_, e)| e).unwrap();
            assert_eq!(
                calls,
                if expired { 0 } else { 1 },
                "Claim and financial reservation must commit or roll back together"
            );
            if expired {
                assert!(result.is_err(), "Expired claim was accepted");
                assert_eq!(
                    result.err().unwrap().code(),
                    "planning-claim-expired-before-publication"
                );
                assert_eq!(
                    after, before,
                    "Rejected claim must preserve Work and activation count"
                );
            } else {
                assert!(result.is_ok());
                assert_eq!(after.planning.as_ref().unwrap().activations, 1);
                let mut decision = PlanningRequest {
                    schema_version: 1,
                    event_id: "clock-decision".into(),
                    expected_revision: after.revision,
                    scope: "root".into(),
                    scope_revision: 1,
                    holder: request.holder.clone(),
                    generation: 1,
                    input_events: Some(vec!["clock-enable".into()]),
                    reason: "Observed clock boundary".into(),
                    ..Default::default()
                };
                let mut store = crate::RootStore::open(&f.root, true).unwrap();
                let lease_before = store.read_work(&created.id).unwrap();
                let renewal = PlanningRequest {
                    event_id: "clock-renew".into(),
                    expected_revision: lease_before.revision,
                    generation: 1,
                    ..request.clone()
                };
                let renewed = store.execute_external(
                    &created.id,
                    "planning.lease-renew",
                    &crate::encode_planning_request(&renewal).unwrap(),
                    &mut Clock {
                        first: true,
                        initial: "2026-10-20T12:00:04Z",
                        publication: decision_publication,
                    },
                );
                if decision_publication == "2026-10-20T12:00:06Z" {
                    assert_eq!(renewed.err().unwrap().code(), "stale_lease");
                    assert_eq!(store.read_work(&created.id).unwrap(), lease_before);
                } else {
                    let work = renewed.unwrap().result.work;
                    let p = work.planning.as_ref().unwrap();
                    assert_eq!(p.activations, 1);
                    assert_eq!(p.decisions, 0);
                    let scope = p
                        .scopes
                        .as_ref()
                        .unwrap()
                        .iter()
                        .find(|s| s.id == "root")
                        .unwrap();
                    assert_eq!(scope.generation, 1);
                    assert_eq!(scope.revision, 1);
                    assert_eq!(scope.lease_until, "2026-10-20T12:00:09Z");
                }
                decision.expected_revision = store.read_work(&created.id).unwrap().revision;
                let before = store.read_work(&created.id).unwrap();
                let result = store.execute_external(
                    &created.id,
                    "planning.decide",
                    &crate::encode_planning_request(&decision).unwrap(),
                    &mut Clock {
                        first: true,
                        initial: "2026-10-20T12:00:04Z",
                        publication: decision_publication,
                    },
                );
                if decision_publication == "2026-10-20T12:00:06Z" {
                    assert_eq!(result.err().unwrap().code(), "stale_lease");
                    assert_eq!(
                        store.read_work(&created.id).unwrap(),
                        before,
                        "Expired decision must preserve the complete recorded Work"
                    );
                    // Recover with a new public claim after expiry, preserving
                    // consumed activations and advancing clocks only forward.
                    let reclaimed = store
                        .execute_external(
                            &created.id,
                            "planning.claim",
                            &crate::encode_planning_request(&PlanningRequest {
                                event_id: "clock-reclaim".into(),
                                expected_revision: before.revision,
                                ..request.clone()
                            })
                            .unwrap(),
                            &mut Clock {
                                first: true,
                                initial: "2026-10-20T12:00:06Z",
                                publication: "2026-10-20T12:00:06Z",
                            },
                        )
                        .unwrap()
                        .result
                        .work;
                    assert_eq!(reclaimed.planning.as_ref().unwrap().activations, 2);
                    let recovered = store
                        .execute_external(
                            &created.id,
                            "planning.decide",
                            &crate::encode_planning_request(&PlanningRequest {
                                expected_revision: reclaimed.revision,
                                generation: 2,
                                ..decision.clone()
                            })
                            .unwrap(),
                            &mut Clock {
                                first: true,
                                initial: "2026-10-20T12:00:06Z",
                                publication: "2026-10-20T12:00:07Z",
                            },
                        )
                        .unwrap();
                    assert_eq!(recovered.result.work.planning.unwrap().decisions, 1);
                } else {
                    assert_eq!(result.unwrap().result.work.planning.unwrap().decisions, 1);
                }
                store.close().unwrap();
            }
        }
    }
}
