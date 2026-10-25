//! Planning request codec and later controls; data establishes no authority.
use std::{error::Error, fmt};

/// Safe request-codec refusal; the original cause is available without rendering
/// request contents, repository paths or supplied actor declarations.
pub struct PlanningRequestError(crate::WorkError);
impl PlanningRequestError {
    /// Return the stable safe format family; no request, IO or action is changed.
    pub fn code(&self) -> &'static str {
        "planning-request-format"
    }
}
impl fmt::Display for PlanningRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for PlanningRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PlanningRequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}
/// Decode the actual complete Go PlanningRequest JSON contract with strict known
/// fields, casefold/duplicate/null semantics and typed numbers. Request and nested
/// custom dependency marshaling remain data: no schema/action/authority is accepted.
/// Unknown fields, invalid UTF8/types or trailing data refuse with a preserved safe
/// cause. This allocates proportionally to input; the external caller owns its
/// transport boundary. No IO, Work modification, identity or operation occurs.
pub fn decode_planning_request(raw: &[u8]) -> Result<crate::PlanningRequest, PlanningRequestError> {
    crate::work::decode_mode(raw, crate::planning_request::shape_planning_request(), true)
        .map_err(PlanningRequestError)
}
/// Encode Go-compatible request data, preserving the explicit empty dependency
/// revision operation and ordinary omitempty rules. Returns codec/allocation
/// errors through WorkError; performs no IO or operation and grants no authority.
pub fn encode_planning_request(
    request: &crate::PlanningRequest,
) -> Result<Vec<u8>, crate::WorkError> {
    crate::encode_model(request)
}

/// Protocol actions, not a configurable permission or proof of the caller's role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanningControlAction {
    /// Hierarchy creation; public pure calls require local-only data. Root observes
    /// provider/reviewer metadata separately; repository remains unported.
    Enable,
    /// Root-only operator scope claim; pure controls cannot establish source IO.
    Claim,
    /// Root-only leased acknowledgement and supported planning operations.
    Decide,
    /// Root-only renewal of a live claim; distinct from a judge cycle.
    LeaseRenew,
    /// Root operator cycle verdict; no claim, model call or input acknowledgement.
    Judge,
    /// Root operator cycle renewal, distinct from renewal of a leased claim.
    CycleRenew,
    /// Operator rejection of a future, unused attributed instruction addition.
    RejectInstructions,
    Pause,
    Resume,
}
impl PlanningControlAction {
    /// Protocol classification for canonical mission-state fencing, not an
    /// editable policy or permission grant. The exhaustive match forces future
    /// action ports to choose their boundary explicitly; configuration and
    /// pause/resume remain available without authorizing activation or execution.
    pub(crate) fn requires_active_mission(self) -> bool {
        match self {
            Self::Claim | Self::Decide | Self::LeaseRenew | Self::Judge | Self::CycleRenew => true,
            Self::Enable | Self::Pause | Self::Resume | Self::RejectInstructions => false,
        }
    }
    /// Select the exact supported recorded operation; unknown kinds remain unported.
    pub fn from_kind(kind: &str) -> Option<Self> {
        match kind {
            "planning.enable" => Some(Self::Enable),
            "planning.claim" => Some(Self::Claim),
            "planning.decide" => Some(Self::Decide),
            "planning.lease-renew" => Some(Self::LeaseRenew),
            "planning.judge" => Some(Self::Judge),
            "planning.renew" => Some(Self::CycleRenew),
            "planning.pause" => Some(Self::Pause),
            "planning.resume" => Some(Self::Resume),
            "planning.reject-instructions" => Some(Self::RejectInstructions),
            _ => None,
        }
    }
}
/// Safe planning-control refusal with typed original causes. No payload is printed.
pub enum PlanningControlError {
    Refused(&'static str),
    Settings(crate::EngineSettingsError),
    State(crate::PlanningStateError),
    Policy(crate::ValidationPolicyError),
    ClockParse(swarm_wire_json::go_rfc3339::GoTimestampError),
    ClockFormat(time::error::Format),
    Allocation(std::collections::TryReserveError),
}
impl PlanningControlError {
    /// Return the safe failure family; settings/state retain their own categories.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Settings(e) => e.code(),
            Self::State(e) => e.code(),
            Self::Policy(e) => e.code(),
            Self::Refused(
                "planning-enable-provider-unported" | "planning-enable-repository-unported",
            ) => "command_not_ported",
            Self::Refused("planning-schema-required") => "format",
            Self::Refused("planning-review-budget-not-applicable") => "invalid_review_budget",
            Self::Refused(_) => "planning-control-invalid",
            Self::ClockParse(_) | Self::ClockFormat(_) => "planning-control-clock-invalid",
            Self::Allocation(_) => "planning-control-allocation",
        }
    }
    /// Return static diagnostic context, including the precise durable-state
    /// reason, without supplied scope/reason/actor values or private cause text.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(reason) => reason,
            Self::Settings(e) => e.reason(),
            Self::State(e) => e.reason(),
            Self::Policy(e) => e.reason(),
            Self::ClockParse(_) | Self::ClockFormat(_) => "planning-control-timestamp",
            Self::Allocation(_) => "planning-control-buffer",
        }
    }
}
impl fmt::Display for PlanningControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for PlanningControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PlanningControlError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Refused(_) => None,
            Self::Settings(e) => Some(e),
            Self::State(e) => Some(e),
            Self::Policy(e) => Some(e),
            Self::ClockParse(e) => Some(e),
            Self::ClockFormat(e) => Some(e),
            Self::Allocation(e) => Some(e),
        }
    }
}
/// Validate Go planningChange's pretransaction controls for pause/resume.
/// Schema1 is required; scope-file edits, nonzero review-call budget and an
/// instruction-rejection ID belong to other actions. Other typed metadata,
/// including Operations, is passive for controls and remains in the replay intent.
/// Returns a safe refusal without IO, Work edits or any authority decision.
pub fn validate_planning_control_request(
    request: &crate::PlanningRequest,
) -> Result<(), PlanningControlError> {
    validate_control_fields(request, false)
}
/// Action-aware envelope validation permits a positive review budget only for
/// provider enablement; only RejectInstructions accepts its revision ID. The
/// Root writer separately validates the operator intent, task and active owners.
/// This is wire applicability, not a role/caller grant.
pub(crate) fn validate_planning_control_action(
    request: &crate::PlanningRequest,
    action: PlanningControlAction,
) -> Result<(), PlanningControlError> {
    if action == PlanningControlAction::RejectInstructions {
        let mut envelope=request.clone();
        envelope.instruction_revision_id.clear();
        return validate_control_fields(&envelope, false);
    }
    validate_control_fields(
        request,
        action == PlanningControlAction::Enable && !request.provider.is_empty(),
    )
}
/// Share the original strict checks; only the explicit action selects whether
/// the optional review budget applies. Negative values never become inheritance.
fn validate_control_fields(
    request: &crate::PlanningRequest,
    review_budget_applies: bool,
) -> Result<(), PlanningControlError> {
    use PlanningControlError::Refused as E;
    if request.schema_version != 1 {
        return Err(E("planning-schema-required"));
    }
    if request
        .scope_files
        .as_ref()
        .is_some_and(|files| !files.is_empty())
    {
        return Err(E("planning-scope-files-reserved"));
    }
    if request.max_review_calls != 0 && (!review_budget_applies || request.max_review_calls < 1) {
        return Err(E("planning-review-budget-not-applicable"));
    }
    if !request.instruction_revision_id.is_empty() {
        return Err(E("planning-instruction-revision-reserved"));
    }
    Ok(())
}
/// Prepare local enablement or a control candidate, never mutate the input.
/// Enable without a private source preparation remains local-only. The caller must
/// enforce the canonical request cap, reservation/revision, authority and replay.
/// The pure function does not establish the external transport boundary.
/// Pause/resume revoke every scope lease by incrementing its nonnegative generation
/// and clearing holder/until. Resume clears failure; a nonblank reason additionally
/// marks its scope (canonical root when omitted) ready and appends operator_resume.
/// Only that reason consumes explicit v11 byte policy: legacy absence refuses it,
/// zero forbids it, null is unlimited. Pause and resume without a nonblank reason
/// remain available on valid older settings. Other request metadata is passive.
///
/// `at` is a trusted caller's RFC3339 clock observation, normalized for the event;
/// it authenticates no clock. Schema/global controls, settings/state, scope/size,
/// nonnegative generation/checked overflow, clock/format and fallible buffers
/// refuse with safe causes. Cloning/serde-owned memory still follows standard Rust
/// allocation behavior; this is not an aggregate memory bound or OS confinement.
/// No persistence, process stop, budget reset, caller identity/authority or replay
/// is established. The transaction owner must enforce reservation, event/revision,
/// authenticated transport and durable publication separately.
pub fn apply_planning_control(
    work: &crate::Work,
    action: PlanningControlAction,
    request: &crate::PlanningRequest,
    at: &str,
    settings: &crate::EngineSettingsDocument,
) -> Result<crate::Work, PlanningControlError> {
    apply_planning_control_prepared(work, action, request, at, settings, None)
}
/// Trusted transaction adapter adds source-observed provider configuration only
/// to enable. Public pure calls retain unported-provider refusal. No prepared
/// object comes from request JSON and no worker/planner process is authorized.
pub(crate) fn apply_planning_control_prepared(
    work: &crate::Work,
    action: PlanningControlAction,
    request: &crate::PlanningRequest,
    at: &str,
    settings: &crate::EngineSettingsDocument,
    provider: Option<&crate::reviewer_bootstrap::Prepared>,
) -> Result<crate::Work, PlanningControlError> {
    use PlanningControlError as E;
    validate_planning_control_action(request, action)?;
    if action == PlanningControlAction::Enable {
        return apply_local_planning_enable(work, request, at, settings, provider);
    }
    let root_reason = match action {
        PlanningControlAction::Claim => Some("planning-claim-requires-root"),
        PlanningControlAction::Decide => Some("planning-decision-requires-root"),
        PlanningControlAction::RejectInstructions => Some("instruction-rejection-requires-root"),
        PlanningControlAction::LeaseRenew => Some("planning-lease-renew-requires-root"),
        PlanningControlAction::Judge | PlanningControlAction::CycleRenew => Some("planning-judge-operator-requires-root"),
        _ => None,
    };
    if let Some(reason) = root_reason {
        return Err(E::Refused(reason));
    }
    settings.validate().map_err(E::Settings)?;
    crate::validate_planning_state(work).map_err(E::State)?;
    let planning = work
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    let scopes = planning.scopes.as_deref().unwrap_or_default();
    let reason = action == PlanningControlAction::Resume
        && !crate::text::trim_space(&request.reason).is_empty();
    let selected = if reason {
        let cap = settings
            .values
            .planning_resume_reason_limit()
            .map_err(E::Settings)?;
        if cap.is_some_and(|cap| request.reason.len() > cap) {
            return Err(E::Refused("planning-resume-reason-limit-exceeded"));
        }
        let name = if request.scope.is_empty() {
            "root"
        } else {
            &request.scope
        };
        Some(
            scopes
                .iter()
                .position(|scope| scope.id == name)
                .ok_or(E::Refused("planning-scope-not-found"))?,
        )
    } else {
        None
    };
    let mut generations = Vec::new();
    generations
        .try_reserve_exact(scopes.len())
        .map_err(E::Allocation)?;
    for scope in scopes {
        if scope.generation < 0 {
            return Err(E::Refused("planning-generation-negative"));
        }
        generations.push(
            scope
                .generation
                .checked_add(1)
                .ok_or(E::Refused("planning-generation-representation"))?,
        );
    }
    let instant = swarm_wire_json::go_rfc3339::parse(at).map_err(E::ClockParse)?;
    let timestamp = instant
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(E::ClockFormat)?;
    let mut next = work.clone();
    let planning = next
        .planning
        .as_deref_mut()
        .ok_or(E::Refused("planning-not-enabled"))?;
    if selected.is_some() {
        planning
            .inbox
            .get_or_insert_with(Vec::new)
            .try_reserve(1)
            .map_err(E::Allocation)?;
    }
    planning.paused = action == PlanningControlAction::Pause;
    if action == PlanningControlAction::Resume {
        planning.failure.clear();
    }
    for (scope, generation) in planning.scopes.iter_mut().flatten().zip(generations) {
        scope.generation = generation;
        scope.holder.clear();
        scope.lease_until.clear();
    }
    if let Some(index) = selected {
        let scope = planning
            .scopes
            .as_mut()
            .and_then(|scopes| scopes.get_mut(index))
            .ok_or(E::Refused("planning-scope-not-found"))?;
        scope.state = "ready".into();
        let event = crate::PlanningEvent {
            id: request.event_id.clone(),
            scope: scope.id.clone(),
            kind: "operator_resume".into(),
            message: request.reason.clone(),
            at: timestamp,
            ..Default::default()
        };
        planning
            .inbox
            .as_mut()
            .ok_or(E::Refused("planning-inbox-buffer"))?
            .push(event);
    }
    crate::validate_planning_state(&next).map_err(E::State)?;
    Ok(next)
}

/// Check saved v12 canonical intent bytes for a new transaction. Zero refuses any
/// nonempty intent; null removes this policy cap, not the transport bound. Missing
/// adoption/platform overflow/size refuse safely. Call after recorded replay and
/// before candidate edits; this performs no IO or authority decision. The caller
/// has already allocated serialized bytes: this cap bounds admitted intents, not
/// aggregate allocation or authenticated transport.
pub fn validate_planning_intent_size(
    bytes: usize,
    settings: &crate::EngineSettingsDocument,
) -> Result<(), PlanningControlError> {
    let cap = settings
        .values
        .planning_request_limit()
        .map_err(PlanningControlError::Settings)?;
    if cap.is_some_and(|cap| bytes > cap) {
        return Err(PlanningControlError::Refused(
            "planning-request-byte-limit-exceeded",
        ));
    }
    Ok(())
}
/// Validate new-target/budget/repository prerequisites before catalogue IO.
/// Shared by the public pure builder and transaction bootstrap; this grants no
/// provider source preparation and never permits historical adoption or launch.
pub(crate) fn validate_planning_enable_target(
    work: &crate::Work,
    request: &crate::PlanningRequest,
) -> Result<(), PlanningControlError> {
    use PlanningControlError as E;
    if work.planning.is_some() || work.tasks.as_ref().is_some_and(|tasks| !tasks.is_empty()) {
        return Err(E::Refused("planning-enable-new-work-required"));
    }
    if request.max_activations < 1
        || request.max_tasks < 1
        || request.max_decisions < 1
        || request.max_judge_cycles < 0
    {
        return Err(E::Refused("planning-enable-budget-invalid"));
    }
    if request.repository.is_some() {
        return Err(E::Refused("planning-enable-repository-unported"));
    }
    Ok(())
}
/// Build a fresh hierarchy candidate from explicit budgets/requirements/checks
/// under saved limits. Private source preparation may add provider digest/route
/// and initial reviewer configuration; public pure calls refuse providers.
/// Normalized clock/controls and valid state are required. No process, runtime
/// review enforcement, repository setup or authenticated identity is implied.
/// Errors preserve the input Work; the caller owns replay/transaction/authority.
fn apply_local_planning_enable(
    work: &crate::Work,
    request: &crate::PlanningRequest,
    at: &str,
    settings: &crate::EngineSettingsDocument,
    provider: Option<&crate::reviewer_bootstrap::Prepared>,
) -> Result<crate::Work, PlanningControlError> {
    use PlanningControlError as E;
    settings.validate().map_err(E::Settings)?;
    crate::validate_planning_state(work).map_err(E::State)?;
    validate_planning_enable_target(work, request)?;
    if !request.provider.is_empty() && provider.is_none() {
        return Err(E::Refused("planning-enable-provider-unported"));
    }
    if let Some(provider) = provider {
        if provider.provider != request.provider
            || provider.reviewer.provider != request.provider
            || provider.provider_digest != provider.reviewer.provider_digest
        {
            return Err(E::Refused("planning-enable-provider-preparation-mismatch"));
        }
    }
    let criteria = work.criteria.as_deref().unwrap_or_default();
    let cap = settings
        .values
        .planning_requirement_limit()
        .map_err(E::Settings)?;
    if criteria.is_empty() || cap.is_some_and(|cap| criteria.len() > cap) {
        return Err(E::Refused("planning-enable-requirement-limit"));
    }
    let mut requirements = Vec::new();
    requirements
        .try_reserve_exact(criteria.len())
        .map_err(E::Allocation)?;
    for i in 0..criteria.len() {
        requirements.push(format!("req-{}", i + 1));
    }
    let limits = settings.values.mutation_limits().map_err(E::Settings)?;
    let mut checks = std::collections::BTreeMap::new();
    for (requirement, controls) in request.checks.iter().flatten() {
        if !requirements.contains(requirement) {
            return Err(E::Refused("planning-enable-unknown-requirement"));
        }
        let mut controls = controls.clone();
        for control in controls.iter_mut().flatten() {
            control.criteria = Some(vec![1]);
        }
        let normalized = crate::normalize_validation_policy(
            crate::ValidationPolicy {
                mode: "automatic".into(),
                controls,
                ..Default::default()
            },
            &limits.policy,
        )
        .map_err(E::Policy)?;
        checks.insert(requirement.clone(), normalized.controls);
    }
    let instant = swarm_wire_json::go_rfc3339::parse(at).map_err(E::ClockParse)?;
    let timestamp = instant
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(E::ClockFormat)?;
    let planning = crate::PlanningState {
        provider: request.provider.clone(),
        provider_digest: provider.map_or_else(String::new, |p| p.provider_digest.clone()),
        model_route: provider.and_then(|p| p.route.clone()),
        reviewer: provider.map(|p| p.reviewer.clone()),
        reviewer_required: provider.is_some(),
        version: 1,
        max_tasks: request.max_tasks,
        max_decisions: request.max_decisions,
        max_activations: request.max_activations,
        scopes: Some(vec![crate::PlanningScope {
            id: "root".into(),
            objective: work.objective.clone(),
            requirements: Some(requirements),
            revision: 1,
            state: "ready".into(),
            ..Default::default()
        }]),
        inbox: Some(vec![crate::PlanningEvent {
            id: request.event_id.clone(),
            scope: "root".into(),
            kind: "brief".into(),
            message: work.objective.clone(),
            at: timestamp,
            ..Default::default()
        }]),
        judge: Some(Box::new(crate::JudgeState {
            cycle: 1,
            max_cycles: request.max_judge_cycles,
            ..Default::default()
        })),
        checks: Some(checks),
        ..Default::default()
    };
    let mut next = work.clone();
    next.planning = Some(Box::new(planning));
    crate::validate_planning_state(&next).map_err(E::State)?;
    Ok(next)
}
