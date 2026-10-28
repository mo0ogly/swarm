//! Frozen legacy Bundle JSON type graph; no import or runtime authority.
//! Known unused fields are validated too; unknown fields follow Go decoding.
use crate::model_wire::{Fields, Kind};
// internal/engine/plan_contract.go SHA-256 1dc52ac99dee53d507d7ee00f994ecd6f7e3e820b41ac1dc749c12f09b44b0a8
/// Frozen ActionPlan field types; pure metadata, no IO or grant.
pub(super) fn shape_action_plan() -> Fields {
    &[
        ("product", Kind::Pointer(&Kind::Object(shape_product_plan))),
        ("version", Kind::Int64),
        ("objective", Kind::String),
        ("assumptions", Kind::Array(&Kind::String)),
        ("questions", Kind::Array(&Kind::Object(shape_plan_question))),
        ("tasks", Kind::Array(&Kind::Object(shape_plan_mission))),
    ]
}
// internal/engine/action_skills.go SHA-256 55a787ab8b74049c648c32c414914bcf1dbc2bd2737e1e35629854d51127bb6f
/// Frozen ActionSkill field types; pure metadata, no IO or grant.
pub(super) fn shape_action_skill() -> Fields {
    &[
        ("path", Kind::String),
        ("sha256", Kind::String),
        ("name", Kind::String),
        ("description", Kind::String),
        ("bytes", Kind::Int64),
        ("available", Kind::Bool),
        ("reason", Kind::String),
    ]
}
// internal/engine/action_skills.go SHA-256 55a787ab8b74049c648c32c414914bcf1dbc2bd2737e1e35629854d51127bb6f
/// Frozen ActionSkillSelection field types; pure metadata, no IO or grant.
pub(super) fn shape_action_skill_selection() -> Fields {
    &[("path", Kind::String), ("sha256", Kind::String)]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Frozen Agent field types; pure metadata, no IO or grant.
pub(super) fn shape_agent() -> Fields {
    &[
        (
            "dialogue_restore",
            Kind::Pointer(&Kind::Object(shape_dialogue_restore_receipt)),
        ),
        ("delivery_version", Kind::Int64),
        (
            "workflow",
            Kind::Pointer(&Kind::Object(shape_agent_workflow)),
        ),
        (
            "provider_cooldown",
            Kind::Pointer(&Kind::Object(shape_provider_cooldown)),
        ),
        (
            "preflight",
            Kind::Pointer(&Kind::Object(shape_preflight_result)),
        ),
        ("precondition_evidence", Kind::String),
        ("mode", Kind::String),
        (
            "model_route",
            Kind::Pointer(&Kind::Object(shape_model_route)),
        ),
        (
            "reported_model",
            Kind::Pointer(&Kind::Object(shape_reported_model)),
        ),
        (
            "context",
            Kind::Pointer(&Kind::Object(shape_context_manifest)),
        ),
        (
            "prepared_context",
            Kind::Pointer(&Kind::Object(shape_context_manifest)),
        ),
        ("brainstorm", Kind::Bool),
        ("reply", Kind::String),
        ("usage", Kind::Pointer(&Kind::Object(shape_usage))),
        ("progress", Kind::Object(shape_agent_progress)),
        ("diagnostic", Kind::Object(shape_attempt_diagnostic)),
        ("recovery", Kind::Object(shape_recovery_state)),
        ("limits", Kind::Object(shape_run_limits)),
        ("id", Kind::String),
        ("work_id", Kind::String),
        ("task_id", Kind::String),
        ("attempt_id", Kind::String),
        ("origin", Kind::String),
        ("relay", Kind::String),
        ("stop_kind", Kind::String),
        ("provider", Kind::String),
        ("role", Kind::String),
        ("parent", Kind::String),
        ("previous", Kind::String),
        ("workspace", Kind::String),
        ("status", Kind::String),
        ("desired", Kind::String),
        ("activity", Kind::String),
        ("started", Kind::String),
        ("heartbeat", Kind::String),
        ("ended", Kind::String),
        ("exit_code", Kind::Pointer(&Kind::Int64)),
        ("supervisor_pid", Kind::Int64),
        ("supervisor_identity", Kind::String),
        ("child_pid", Kind::Int64),
        ("child_identity", Kind::String),
        ("host", Kind::String),
        ("timeout_seconds", Kind::Int64),
        ("capture_output", Kind::Bool),
        ("prompt", Kind::String),
        ("command", Kind::String),
        ("args", Kind::Array(&Kind::String)),
        ("env_allow", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Frozen AgentLog field types; pure metadata, no IO or grant.
pub(super) fn shape_agent_log() -> Fields {
    &[
        ("seq", Kind::Int64),
        ("agent_id", Kind::String),
        ("at", Kind::String),
        ("kind", Kind::String),
        ("message", Kind::String),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Frozen AgentProgress field types; pure metadata, no IO or grant.
pub(super) fn shape_agent_progress() -> Fields {
    &[
        ("elapsed_seconds", Kind::Int64),
        ("time_threshold_exceeded", Kind::Bool),
        ("tool_wait_seconds", Kind::Int64),
        ("tool_time_threshold_exceeded", Kind::Bool),
        ("action", Kind::String),
        ("detail", Kind::String),
        ("tool_calls", Kind::Int64),
        ("tool_results", Kind::Int64),
        ("pending_tools", Kind::Int64),
        ("last_tool", Kind::String),
        ("last_result_at", Kind::String),
        ("last_output_at", Kind::String),
        ("output_state", Kind::String),
        ("tool_state", Kind::String),
        ("provider_vitality", Kind::String),
        ("degraded", Kind::String),
        ("metrics_version", Kind::Int64),
        ("tool_reads", Kind::Int64),
        ("tool_writes", Kind::Int64),
        ("tool_unclassified", Kind::Int64),
        ("tool_errors_total", Kind::Int64),
        ("tool_repeats_total", Kind::Int64),
    ]
}
// internal/engine/agent_workflow.go SHA-256 70fc614c1c07aa5e117f55945c3b9fa674519ff3c662ad21bcc029224c85be76
/// Frozen AgentWorkflow field types; pure metadata, no IO or grant.
pub(super) fn shape_agent_workflow() -> Fields {
    &[
        ("skills", Kind::Array(&Kind::Object(shape_action_skill))),
        (
            "project",
            Kind::Pointer(&Kind::Object(shape_project_context)),
        ),
        ("version", Kind::Int64),
        ("role", Kind::String),
        ("methods", Kind::Array(&Kind::String)),
        ("sha256", Kind::String),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen AnswerFact field types; pure metadata, no IO or grant.
pub(super) fn shape_answer_fact() -> Fields {
    &[
        ("text", Kind::String),
        ("source_ids", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen AnswerStep field types; pure metadata, no IO or grant.
pub(super) fn shape_answer_step() -> Fields {
    &[
        ("action_id", Kind::String),
        ("why", Kind::String),
        ("source_ids", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/plan_contract.go SHA-256 1dc52ac99dee53d507d7ee00f994ecd6f7e3e820b41ac1dc749c12f09b44b0a8
/// Frozen ApprovedPlan field types; pure metadata, no IO or grant.
pub(super) fn shape_approved_plan() -> Fields {
    &[
        ("task_map", Kind::Map(&Kind::String)),
        ("source", Kind::String),
        ("brief_hash", Kind::String),
        ("response_hash", Kind::String),
        ("spec", Kind::Object(shape_action_plan)),
        ("task_ids", Kind::Array(&Kind::String)),
        ("at", Kind::String),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen AssistRefusal field types; pure metadata, no IO or grant.
pub(super) fn shape_assist_refusal() -> Fields {
    &[
        ("code", Kind::String),
        ("kind", Kind::String),
        ("message", Kind::String),
        ("detail", Kind::String),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen AssistTurn field types; pure metadata, no IO or grant.
pub(super) fn shape_assist_turn() -> Fields {
    &[
        (
            "model_route",
            Kind::Pointer(&Kind::Object(shape_model_route)),
        ),
        ("timeout_seconds", Kind::Int64),
        ("provider_digest", Kind::String),
        ("supervisor_pid", Kind::Int64),
        ("supervisor_stamp", Kind::String),
        ("imported", Kind::Bool),
        ("pid", Kind::Int64),
        ("process_stamp", Kind::String),
        ("host", Kind::String),
        ("coordinates", Kind::Object(shape_page_coordinates)),
        ("request_hash", Kind::String),
        ("stale", Kind::Bool),
        ("id", Kind::String),
        ("work_id", Kind::String),
        ("status", Kind::String),
        ("created_at", Kind::String),
        ("ended_at", Kind::String),
        ("question", Kind::String),
        ("template_id", Kind::String),
        ("prompt_version", Kind::String),
        ("provider", Kind::String),
        ("context", Kind::Object(shape_page_context)),
        ("prompt", Kind::String),
        (
            "answer",
            Kind::Pointer(&Kind::Object(shape_assistant_answer)),
        ),
        (
            "refusal",
            Kind::Pointer(&Kind::Object(shape_assist_refusal)),
        ),
        ("format_repaired", Kind::Bool),
        ("raw_reply_bytes", Kind::Int64),
        (
            "usage",
            Kind::Pointer(&Kind::Object(shape_assist_usage_note)),
        ),
        ("answer_bytes", Kind::Int64),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen AssistUsageNote field types; pure metadata, no IO or grant.
pub(super) fn shape_assist_usage_note() -> Fields {
    &[
        ("provider", Kind::String),
        ("usage", Kind::Pointer(&Kind::Object(shape_usage))),
        ("note", Kind::String),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen AssistantAnswer field types; pure metadata, no IO or grant.
pub(super) fn shape_assistant_answer() -> Fields {
    &[
        ("version", Kind::Int64),
        ("template_id", Kind::String),
        ("context_hash", Kind::String),
        ("facts", Kind::Array(&Kind::Object(shape_answer_fact))),
        ("interpretation", Kind::String),
        ("missing_information", Kind::Array(&Kind::String)),
        ("next_steps", Kind::Array(&Kind::Object(shape_answer_step))),
        ("limitations", Kind::Array(&Kind::String)),
        ("questions", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen Attempt field types; pure metadata, no IO or grant.
pub(super) fn shape_attempt() -> Fields {
    &[
        ("id", Kind::String),
        ("status", Kind::String),
        ("started", Kind::String),
        ("ended", Kind::String),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Frozen AttemptDiagnostic field types; pure metadata, no IO or grant.
pub(super) fn shape_attempt_diagnostic() -> Fields {
    &[
        ("agent_id", Kind::String),
        ("attempt_id", Kind::String),
        ("observed_errors", Kind::Int64),
        ("consecutive_error_limit", Kind::Int64),
        ("limit_reached", Kind::Bool),
        ("consecutive_error_limit_reached", Kind::Bool),
        ("limit_reason", Kind::String),
        ("summary", Kind::String),
        ("items", Kind::Array(&Kind::Object(shape_diagnostic_item))),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen AutomaticValidation field types; pure metadata, no IO or grant.
pub(super) fn shape_automatic_validation() -> Fields {
    &[
        ("delivery_version", Kind::Int64),
        ("attempt_id", Kind::String),
        ("revision", Kind::Int64),
        ("candidate_sha", Kind::String),
        ("producer_agent_id", Kind::String),
        ("controller", Kind::String),
        ("policy_digest", Kind::String),
        ("policy", Kind::Object(shape_validation_policy)),
        ("artifacts", Kind::Map(&Kind::String)),
        (
            "controls",
            Kind::Array(&Kind::Object(shape_validation_control_result)),
        ),
        ("receipt", Kind::String),
        ("state", Kind::String),
        ("reason", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen BrainstormAnswer field types; pure metadata, no IO or grant.
pub(super) fn shape_brainstorm_answer() -> Fields {
    &[
        ("attempt", Kind::String),
        ("text", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/archive.go SHA-256 a29a36d01925427ca66088b5500850ab485beb330f22637dc74dd0de88d7fc94
/// Frozen Bundle field types; pure metadata, no IO or grant.
pub(super) fn shape_bundle() -> Fields {
    &[
        (
            "automation_state",
            Kind::Array(&Kind::Object(shape_lifecycle_table)),
        ),
        (
            "assistant_history",
            Kind::Array(&Kind::Object(shape_assist_turn)),
        ),
        (
            "cockpit_history",
            Kind::Pointer(&Kind::Object(shape_cockpit_history)),
        ),
        ("schema_version", Kind::Int64),
        ("work", Kind::Object(shape_work)),
        ("events", Kind::Array(&Kind::Object(shape_event))),
        ("files", Kind::Map(&Kind::String)),
        ("missing", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Frozen CockpitHistory field types; pure metadata, no IO or grant.
pub(super) fn shape_cockpit_history() -> Fields {
    &[
        ("decisions", Kind::Array(&Kind::Object(shape_decision))),
        ("visits", Kind::Array(&Kind::Object(shape_visit))),
        ("agents", Kind::Array(&Kind::Object(shape_agent))),
        ("logs", Kind::Array(&Kind::Object(shape_agent_log))),
        ("controls", Kind::Array(&Kind::Map(&Kind::String))),
        ("limits", Kind::String),
    ]
}
// internal/engine/dialogue_context.go SHA-256 510d596e9f20ee3e27c3791b3ebf1ef2c96cf2d3eb348907b5e76725f99bf079
/// Frozen ContextManifest field types; pure metadata, no IO or grant.
pub(super) fn shape_context_manifest() -> Fields {
    &[
        (
            "summary",
            Kind::Pointer(&Kind::Object(shape_memory_summary_receipt)),
        ),
        ("memory", Kind::Array(&Kind::Object(shape_memory_reference))),
        (
            "memory_decision",
            Kind::Pointer(&Kind::Object(shape_memory_selection_decision)),
        ),
        ("sha256", Kind::String),
        ("bytes", Kind::Int64),
        ("included", Kind::Array(&Kind::String)),
        ("excluded", Kind::Array(&Kind::String)),
        ("references", Kind::Array(&Kind::Object(shape_dialogue_ref))),
        ("paths", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/corrective_recovery.go SHA-256 032a71d86ebb2d1fc40ef4b3cbdaa68e01feb6362d8de44368578cbb20049696
/// Frozen CorrectiveRecovery field types; pure metadata, no IO or grant.
pub(super) fn shape_corrective_recovery() -> Fields {
    &[
        ("missing_report", Kind::Bool),
        ("result_commit", Kind::String),
        ("event_id", Kind::String),
        ("authorized_revision", Kind::Int64),
        ("review_id", Kind::String),
        ("attempt_id", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("reason", Kind::String),
        ("instruction", Kind::String),
    ]
}
// internal/engine/tracking.go SHA-256 66198f09a6f5b2b8459d5e27c46cd74b61fe689c9875c916a24789d9683b78df
/// Frozen Decision field types; pure metadata, no IO or grant.
pub(super) fn shape_decision() -> Fields {
    &[
        ("id", Kind::String),
        ("task_id", Kind::String),
        ("agent_id", Kind::String),
        ("kind", Kind::String),
        ("summary", Kind::String),
        ("evidence", Kind::String),
        ("created", Kind::String),
        ("author", Kind::String),
        ("resolved_at", Kind::String),
        ("resolution", Kind::String),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Frozen DiagnosticItem field types; pure metadata, no IO or grant.
pub(super) fn shape_diagnostic_item() -> Fields {
    &[
        ("category", Kind::String),
        ("label", Kind::String),
        ("count", Kind::Int64),
        ("cause", Kind::String),
        ("consequence", Kind::String),
        ("action", Kind::String),
        ("action_kind", Kind::String),
        ("unknown", Kind::Bool),
        ("traces", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/dialogue_context.go SHA-256 510d596e9f20ee3e27c3791b3ebf1ef2c96cf2d3eb348907b5e76725f99bf079
/// Frozen DialogueRef field types; pure metadata, no IO or grant.
pub(super) fn shape_dialogue_ref() -> Fields {
    &[
        ("task", Kind::String),
        ("attempt", Kind::String),
        ("sha256", Kind::String),
    ]
}
// internal/engine/dialogue_restore.go SHA-256 5bf4294aa733eabd0df9e26e958acbadc200729375ca01a368c21c4f8b2f829e
/// Frozen DialogueRestoreReceipt field types; pure metadata, no IO or grant.
pub(super) fn shape_dialogue_restore_receipt() -> Fields {
    &[
        ("source_agent", Kind::String),
        ("source_attempt", Kind::String),
        ("source_state_sha256", Kind::String),
        ("checkpoint_sha256", Kind::String),
        ("facts_sha256", Kind::String),
        ("root_agent", Kind::String),
        ("prior_turns", Kind::Int64),
        ("reason", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("decision_recorded", Kind::Bool),
    ]
}
// internal/engine/score.go SHA-256 3d417b06da3753c2ad86bbaa061ea5409e275166956256b844a030e25192d922
/// Frozen Evaluation field types; pure metadata, no IO or grant.
pub(super) fn shape_evaluation() -> Fields {
    &[
        ("method_version", Kind::String),
        ("scope_id", Kind::String),
        ("phase", Kind::String),
        ("config_digest", Kind::String),
        ("applicability_digest", Kind::String),
        ("artifacts", Kind::Map(&Kind::String)),
        ("domain_scores", Kind::Map(&Kind::Float64)),
        ("quality", Kind::Pointer(&Kind::Float64)),
        ("provisional", Kind::Bool),
        ("quality_band", Kind::String),
        ("progress", Kind::Object(shape_progress)),
        ("max_severity", Kind::String),
        ("blockers", Kind::Array(&Kind::String)),
        ("allowed", Kind::Bool),
        ("ship_allowed", Kind::Bool),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen Event field types; pure metadata, no IO or grant.
pub(super) fn shape_event() -> Fields {
    &[
        ("id", Kind::String),
        ("work_id", Kind::String),
        ("revision", Kind::Int64),
        ("kind", Kind::String),
        ("at", Kind::String),
        ("payload", Kind::Raw),
    ]
}
// internal/engine/agent_exchange.go SHA-256 73e0991f3f455f6d7cd6c5016799c26940abb60d69d6bd2f45691c1405f3fa92
/// Frozen ExchangeArtifact field types; pure metadata, no IO or grant.
pub(super) fn shape_exchange_artifact() -> Fields {
    &[("path", Kind::String), ("sha256", Kind::String)]
}
// internal/engine/score.go SHA-256 3d417b06da3753c2ad86bbaa061ea5409e275166956256b844a030e25192d922
/// Frozen GateRecord field types; pure metadata, no IO or grant.
pub(super) fn shape_gate_record() -> Fields {
    &[
        ("name", Kind::String),
        ("document", Kind::Raw),
        ("evaluation", Kind::Object(shape_evaluation)),
        ("at", Kind::String),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen GitState field types; pure metadata, no IO or grant.
pub(super) fn shape_git_state() -> Fields {
    &[
        ("head", Kind::String),
        ("branch", Kind::String),
        ("changes", Kind::String),
    ]
}
// internal/engine/historical_requalification.go SHA-256 28df882dcda883630a4ad6e5d5c8ae938b681e8e3a7b48aa70aa576cdcf0d04e
/// Frozen HistoricalRequalification field types; pure metadata, no IO or grant.
pub(super) fn shape_historical_requalification() -> Fields {
    &[
        ("event_id", Kind::String),
        ("request_sha256", Kind::String),
        ("proof_key", Kind::String),
        ("agent_id", Kind::String),
        ("attempt_id", Kind::String),
        ("result_commit", Kind::String),
        ("previous_candidate", Kind::String),
        ("contract_sha256", Kind::String),
        ("previous_status", Kind::String),
        (
            "previous_gate",
            Kind::Pointer(&Kind::Object(shape_gate_record)),
        ),
        (
            "previous_validation",
            Kind::Pointer(&Kind::Object(shape_automatic_validation)),
        ),
        (
            "previous_review",
            Kind::Pointer(&Kind::Object(shape_independent_review)),
        ),
        ("reason", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/independent_review.go SHA-256 8aaa695b6bcc95e61684c0cbc099dd34be73cb18374a14c3a7ea8b2d604f20f5
/// Frozen IndependentReview field types; pure metadata, no IO or grant.
pub(super) fn shape_independent_review() -> Fields {
    &[
        ("result_event", Kind::String),
        ("failure_code", Kind::String),
        ("report_artifacts", Kind::Map(&Kind::String)),
        (
            "fragment_journal",
            Kind::Pointer(&Kind::Object(shape_managed_fragment_journal_anchor)),
        ),
        (
            "model_route",
            Kind::Pointer(&Kind::Object(shape_model_route)),
        ),
        ("batch_plan_sha256", Kind::String),
        ("batch_provider_sha256", Kind::String),
        (
            "batches",
            Kind::Array(&Kind::Object(shape_managed_review_batch_verdict)),
        ),
        ("reply_path", Kind::String),
        ("reply_sha256", Kind::String),
        ("timeout_seconds", Kind::Int64),
        (
            "workflow",
            Kind::Pointer(&Kind::Object(shape_agent_workflow)),
        ),
        ("git_report", Kind::String),
        ("candidate_commit", Kind::String),
        ("previous_candidate", Kind::String),
        ("receipt", Kind::String),
        ("receipt_sha256", Kind::String),
        ("context", Kind::String),
        ("context_sha256", Kind::String),
        (
            "managed_tasks",
            Kind::Array(&Kind::Object(shape_managed_task_review)),
        ),
        ("id", Kind::String),
        ("attempt", Kind::String),
        ("producer", Kind::String),
        ("reviewer", Kind::String),
        ("report", Kind::String),
        ("sha256", Kind::String),
        ("contract", Kind::String),
        ("state", Kind::String),
        ("reason", Kind::String),
        (
            "criteria",
            Kind::Array(&Kind::Object(shape_review_criterion)),
        ),
        ("started", Kind::String),
        ("finished", Kind::String),
        ("usage", Kind::Pointer(&Kind::Object(shape_usage))),
    ]
}
// internal/engine/instruction_revision.go SHA-256 8fb06af1125ed0785f282e77be4607e81e4a76e32f1a1ee36a4863cb6efc8615
/// Frozen InstructionRevision field types; pure metadata, no IO or grant.
pub(super) fn shape_instruction_revision() -> Fields {
    &[
        ("id", Kind::String),
        ("addition", Kind::String),
        ("reason", Kind::String),
        ("scope", Kind::String),
        ("holder", Kind::String),
        ("generation", Kind::Int64),
        ("input_events", Kind::Array(&Kind::String)),
        ("decision", Kind::String),
        ("at", Kind::String),
        ("state", Kind::String),
        ("attempts_at_creation", Kind::Int64),
        ("before_sha256", Kind::String),
        ("after_sha256", Kind::String),
        ("used_by_attempt", Kind::String),
        ("rejected_at", Kind::String),
        ("rejected_reason", Kind::String),
    ]
}
// internal/engine/managed_integration_retry.go SHA-256 297962b946fdfc3e57297c10f1fff9dcea444c2f6562ef32dafb7193084212c1
/// Frozen IntegrationRetry field types; pure metadata, no IO or grant.
pub(super) fn shape_integration_retry() -> Fields {
    &[
        ("control_runtime_budget_sha256", Kind::String),
        ("event_id", Kind::String),
        ("request_sha256", Kind::String),
        ("agent_id", Kind::String),
        ("attempt_id", Kind::String),
        ("result_commit", Kind::String),
        ("failed_base", Kind::String),
        ("expected_candidate", Kind::String),
        ("contract_sha256", Kind::String),
        ("original_failure", Kind::String),
        ("failure_evidence", Kind::String),
        ("evidence_sha256", Kind::String),
        ("legacy_missing_output", Kind::Bool),
        ("reason", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/planning_judge.go SHA-256 053019b5a85ab2f292edca745178700117cbf0bfd588194d981e5dc24d9bcf38
/// Frozen JudgeCycleRecord field types; pure metadata, no IO or grant.
pub(super) fn shape_judge_cycle_record() -> Fields {
    &[
        ("verdict", Kind::String),
        ("reason", Kind::String),
        ("evidence_ref", Kind::String),
        ("at", Kind::String),
        ("renewed_at", Kind::String),
        ("cycle", Kind::Int64),
    ]
}
// internal/engine/planning_judge.go SHA-256 053019b5a85ab2f292edca745178700117cbf0bfd588194d981e5dc24d9bcf38
/// Frozen JudgeState field types; pure metadata, no IO or grant.
pub(super) fn shape_judge_state() -> Fields {
    &[
        ("verdict", Kind::String),
        ("reason", Kind::String),
        ("evidence_ref", Kind::String),
        ("at", Kind::String),
        ("cycle", Kind::Int64),
        ("max_cycles", Kind::Int64),
        ("closed", Kind::Bool),
        (
            "history",
            Kind::Array(&Kind::Object(shape_judge_cycle_record)),
        ),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen LaunchProfile field types; pure metadata, no IO or grant.
pub(super) fn shape_launch_profile() -> Fields {
    &[
        (
            "skills",
            Kind::Array(&Kind::Object(shape_action_skill_selection)),
        ),
        ("limits", Kind::Pointer(&Kind::Object(shape_run_limits))),
        ("provider", Kind::String),
        ("role", Kind::String),
        ("workspace", Kind::String),
        ("instruction", Kind::String),
        ("level", Kind::String),
        ("timeout_seconds", Kind::Int64),
        ("capture_output", Kind::Bool),
        ("updated", Kind::String),
        ("actor", Kind::String),
    ]
}
// internal/engine/managed_review_fragment_store.go SHA-256 c561116865ff7ff3b514e9ec328b9e5b609ca164e0eee614cd5921873b02151b
/// Frozen ManagedFragmentJournalAnchor field types; pure metadata, no IO or grant.
pub(super) fn shape_managed_fragment_journal_anchor() -> Fields {
    &[
        ("workflow_prompt", Kind::String),
        ("workflow_prompt_sha256", Kind::String),
        (
            "retry_directive",
            Kind::Pointer(&Kind::Object(shape_managed_review_retry_directive)),
        ),
        (
            "replanned_from",
            Kind::Pointer(&Kind::Object(shape_managed_fragment_journal_anchor)),
        ),
        ("resume_calls", Kind::Array(&Kind::String)),
        ("inspection_task", Kind::String),
        ("inspection_attempt", Kind::String),
        ("inspection_producer", Kind::String),
        ("final_journal", Kind::String),
        ("final_journal_sha256", Kind::String),
        ("workflow_sha256", Kind::String),
        ("model_config_sha256", Kind::String),
        ("plan", Kind::String),
        ("plan_sha256", Kind::String),
        ("journal", Kind::String),
        ("journal_sha256", Kind::String),
    ]
}
// internal/engine/managed_git.go SHA-256 c8a62e0ba38a8c19f18815655f84ce6620fd4f78505216842b1d472de29626a5
/// Frozen ManagedRepository field types; pure metadata, no IO or grant.
pub(super) fn shape_managed_repository() -> Fields {
    &[
        ("project_subdir", Kind::String),
        ("source", Kind::String),
        ("storage", Kind::String),
        ("base_commit", Kind::String),
        ("candidate_commit", Kind::String),
        ("snapshot_digest", Kind::String),
        ("included", Kind::Array(&Kind::String)),
        ("request_digest", Kind::String),
    ]
}
// internal/engine/independent_review.go SHA-256 8aaa695b6bcc95e61684c0cbc099dd34be73cb18374a14c3a7ea8b2d604f20f5
/// Frozen ManagedReviewBatchVerdict field types; pure metadata, no IO or grant.
pub(super) fn shape_managed_review_batch_verdict() -> Fields {
    &[
        ("retry_feedback", Kind::String),
        ("id", Kind::String),
        ("tasks", Kind::Array(&Kind::String)),
        ("context", Kind::String),
        ("context_sha256", Kind::String),
        ("reply_path", Kind::String),
        ("reply_sha256", Kind::String),
        ("state", Kind::String),
        ("started", Kind::String),
        ("finished", Kind::String),
    ]
}
// internal/engine/managed_review_retry_delivery.go SHA-256 7aec051895efb15b343508062ab34bab7a8e2e202695fcfaeb4c2e7454abb59b
/// Frozen ManagedReviewRetryDirective field types; pure metadata, no IO or grant.
pub(super) fn shape_managed_review_retry_directive() -> Fields {
    &[
        (
            "citation_diagnostic",
            Kind::Pointer(&Kind::Object(shape_managed_review_citation_diagnostic)),
        ),
        ("decision", Kind::String),
        ("scope", Kind::String),
        ("result_event", Kind::String),
        ("review", Kind::String),
        ("candidate_commit", Kind::String),
        ("attempt", Kind::String),
        ("failure_code", Kind::String),
        ("cause", Kind::String),
        ("instruction", Kind::String),
    ]
}
// internal/engine/managed_review.go SHA-256 eb826e542a0c9d7188cd847fedcc6e457a7adffb1ef7900bf3c641634796f60d
/// Frozen ManagedTaskReview field types; pure metadata, no IO or grant.
pub(super) fn shape_managed_task_review() -> Fields {
    &[
        ("task", Kind::String),
        ("attempt", Kind::String),
        ("producer", Kind::String),
        ("contract", Kind::String),
        ("policy", Kind::String),
        ("report", Kind::String),
        ("report_sha256", Kind::String),
        ("reason", Kind::String),
        (
            "criteria",
            Kind::Array(&Kind::Object(shape_review_criterion)),
        ),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen ManualOverride field types; pure metadata, no IO or grant.
pub(super) fn shape_manual_override() -> Fields {
    &[
        ("reason", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("previous_status", Kind::String),
        ("revision", Kind::Int64),
    ]
}
// internal/engine/project_memory.go SHA-256 16aa64873f1a26f102a9c1e26d4021425bf063ea7ff1b58c7634a423ec77070d
/// Frozen MemoryReference field types; pure metadata, no IO or grant.
pub(super) fn shape_memory_reference() -> Fields {
    &[
        ("work", Kind::String),
        ("id", Kind::String),
        ("version", Kind::Int64),
        ("sha256", Kind::String),
    ]
}
// internal/engine/project_memory.go SHA-256 16aa64873f1a26f102a9c1e26d4021425bf063ea7ff1b58c7634a423ec77070d
/// Frozen MemorySelectionDecision field types; pure metadata, no IO or grant.
pub(super) fn shape_memory_selection_decision() -> Fields {
    &[
        ("event_id", Kind::String),
        ("actor", Kind::String),
        ("reason", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/memory_summary_selection.go SHA-256 09151f72964711349efd2038f9aef0ea137851c967638ceadc38e9fea5f12c52
/// Frozen MemorySummaryProtection field types; pure metadata, no IO or grant.
pub(super) fn shape_memory_summary_protection() -> Fields {
    &[
        ("id", Kind::String),
        ("message", Kind::String),
        ("kind", Kind::String),
        ("state", Kind::String),
    ]
}
// internal/engine/memory_summary_selection.go SHA-256 09151f72964711349efd2038f9aef0ea137851c967638ceadc38e9fea5f12c52
/// Frozen MemorySummaryReceipt field types; pure metadata, no IO or grant.
pub(super) fn shape_memory_summary_receipt() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("selection", Kind::Object(shape_memory_summary_selection)),
        ("selection_sha256", Kind::String),
        ("work", Kind::String),
        ("task", Kind::String),
        ("prepared_revision", Kind::Int64),
        ("source_message_count", Kind::Int64),
        ("received_message_ids", Kind::Array(&Kind::String)),
        ("omitted_message_ids", Kind::Array(&Kind::String)),
        ("operator_snapshot_injected", Kind::Bool),
        ("attachment_sha256", Kind::String),
        ("attachment_bytes", Kind::Int64),
        ("queued_revision", Kind::Int64),
        ("queued_source_sha256", Kind::String),
        ("checked_before_start_at", Kind::String),
        ("checked_source_sha256", Kind::String),
    ]
}
// internal/engine/memory_summary_selection.go SHA-256 09151f72964711349efd2038f9aef0ea137851c967638ceadc38e9fea5f12c52
/// Frozen MemorySummarySelection field types; pure metadata, no IO or grant.
pub(super) fn shape_memory_summary_selection() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("source_sha256", Kind::String),
        ("dialogues", Kind::Array(&Kind::Object(shape_dialogue_ref))),
        ("cutoff", Kind::Int64),
        (
            "protected",
            Kind::Array(&Kind::Object(shape_memory_summary_protection)),
        ),
        ("reason", Kind::String),
    ]
}
// internal/engine/project_memory.go SHA-256 16aa64873f1a26f102a9c1e26d4021425bf063ea7ff1b58c7634a423ec77070d
/// Frozen MemoryVersion field types; pure metadata, no IO or grant.
pub(super) fn shape_memory_version() -> Fields {
    &[
        ("version", Kind::Int64),
        ("action", Kind::String),
        ("kind", Kind::String),
        ("body", Kind::String),
        ("conditions", Kind::String),
        ("source_retex", Kind::String),
        ("source_sha256", Kind::String),
        ("project_sha256", Kind::String),
        ("actor", Kind::String),
        ("reason", Kind::String),
        ("at", Kind::String),
        ("sha256", Kind::String),
    ]
}
// internal/engine/provider_models.go SHA-256 d1c32751ad1420662b630defb18a04e5bdca71e38a0c5e8fe5dc1aa96e6b3a4b
/// Frozen ModelRoute field types; pure metadata, no IO or grant.
pub(super) fn shape_model_route() -> Fields {
    &[
        ("level", Kind::String),
        ("model", Kind::String),
        ("effort", Kind::String),
        ("billing", Kind::String),
        ("policy_hash", Kind::String),
        ("reason", Kind::String),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen PageAction field types; pure metadata, no IO or grant.
pub(super) fn shape_page_action() -> Fields {
    &[
        ("operation", Kind::String),
        ("id", Kind::String),
        ("label", Kind::String),
        ("target", Kind::String),
        ("available", Kind::Bool),
        ("precondition", Kind::String),
        ("confirmation_required", Kind::Bool),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen PageContext field types; pure metadata, no IO or grant.
pub(super) fn shape_page_context() -> Fields {
    &[
        ("version", Kind::Int64),
        ("contract", Kind::String),
        ("work_id", Kind::String),
        ("work_title", Kind::String),
        ("page_id", Kind::String),
        ("page_title", Kind::String),
        ("revision", Kind::Int64),
        ("captured_at", Kind::String),
        ("context_hash", Kind::String),
        ("selected_entities", Kind::Array(&Kind::String)),
        ("visible_slice", Kind::Object(shape_visible_slice)),
        ("validation", Kind::Object(shape_validation_summary)),
        ("facts", Kind::Array(&Kind::Object(shape_page_fact))),
        ("actions", Kind::Array(&Kind::Object(shape_page_action))),
        ("omissions", Kind::Array(&Kind::String)),
        ("missing", Kind::Array(&Kind::String)),
        ("limits", Kind::Array(&Kind::String)),
        ("freshness", Kind::String),
        ("truncated", Kind::Bool),
        ("untrusted_data", Kind::Bool),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen PageCoordinates field types; pure metadata, no IO or grant.
pub(super) fn shape_page_coordinates() -> Fields {
    &[
        ("report", Kind::String),
        ("kind", Kind::String),
        ("page_id", Kind::String),
        ("selected", Kind::Array(&Kind::String)),
        ("filter", Kind::String),
        ("offset", Kind::Int64),
        ("count", Kind::Int64),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen PageFact field types; pure metadata, no IO or grant.
pub(super) fn shape_page_fact() -> Fields {
    &[
        ("id", Kind::String),
        ("name", Kind::String),
        ("value", Kind::String),
        ("source", Kind::String),
        ("kind", Kind::String),
    ]
}
// internal/engine/plan_contract.go SHA-256 1dc52ac99dee53d507d7ee00f994ecd6f7e3e820b41ac1dc749c12f09b44b0a8
/// Frozen PlanMission field types; pure metadata, no IO or grant.
pub(super) fn shape_plan_mission() -> Fields {
    &[
        ("phase", Kind::String),
        ("id", Kind::String),
        ("title", Kind::String),
        ("role", Kind::String),
        ("scope", Kind::String),
        ("deliverable", Kind::String),
        ("depends", Kind::Array(&Kind::String)),
        ("criteria", Kind::Array(&Kind::String)),
        ("proof", Kind::String),
        ("entry", Kind::String),
        ("validation", Kind::String),
        ("delivery", Kind::String),
        ("stop", Kind::String),
        ("max_attempts", Kind::Int64),
        ("max_tool_calls", Kind::Int64),
    ]
}
// internal/engine/plan_contract.go SHA-256 1dc52ac99dee53d507d7ee00f994ecd6f7e3e820b41ac1dc749c12f09b44b0a8
/// Frozen PlanQuestion field types; pure metadata, no IO or grant.
pub(super) fn shape_plan_question() -> Fields {
    &[("question", Kind::String), ("answer", Kind::String)]
}
// internal/engine/brainstorm.go SHA-256 dc1dd724d07d6d963de53d13048609c619adbc3b7ba726d4234f06bb72b8ae10
/// Frozen PlanningBrief field types; pure metadata, no IO or grant.
pub(super) fn shape_planning_brief() -> Fields {
    &[
        ("task", Kind::String),
        ("path", Kind::String),
        ("sha256", Kind::String),
        ("text", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/planning_delivery.go SHA-256 01bf9ce7df57441449fa2f89be8e353b0b2faa424b109577ae85c4966217ecb1
/// Frozen PlanningDelivery field types; pure metadata, no IO or grant.
pub(super) fn shape_planning_delivery() -> Fields {
    &[
        ("judge_available", Kind::Bool),
        ("judge_cycle", Kind::Int64),
        ("renew_available", Kind::Bool),
        ("judge_end_available", Kind::Bool),
        ("task_budget", Kind::Int64),
        ("activation_budget", Kind::Int64),
        ("delegation_unavailable", Kind::Bool),
        ("closure_unavailable", Kind::Bool),
        ("exploration_available", Kind::Bool),
        (
            "unresolved_review_sources",
            Kind::Map(&Kind::Object(shape_review_failure_routing)),
        ),
        ("review_retry_unavailable", Kind::Bool),
        ("production_retry_unavailable", Kind::Bool),
        ("historical_only", Kind::Bool),
        ("context_sha256", Kind::String),
        ("events", Kind::Array(&Kind::String)),
        (
            "reports",
            Kind::Array(&Kind::Object(shape_exchange_artifact)),
        ),
        ("decision", Kind::String),
        ("unavailable_reports", Kind::Map(&Kind::String)),
    ]
}
// internal/engine/planning.go SHA-256 66ee75df2ce0490e2d362b23030ed033e4e8cf1db402c5448b4464e1af5f62eb
/// Frozen PlanningEvent field types; pure metadata, no IO or grant.
pub(super) fn shape_planning_event() -> Fields {
    &[
        (
            "exploration",
            Kind::Pointer(&Kind::Object(shape_planning_exploration)),
        ),
        ("rejection_key", Kind::String),
        (
            "run_observation",
            Kind::Pointer(&Kind::Object(shape_run_observation)),
        ),
        ("run_decision", Kind::String),
        ("run_reason", Kind::String),
        (
            "handoff",
            Kind::Pointer(&Kind::Object(shape_exchange_artifact)),
        ),
        ("id", Kind::String),
        ("scope", Kind::String),
        ("kind", Kind::String),
        ("task", Kind::String),
        ("attempt", Kind::String),
        ("message", Kind::String),
        (
            "artifacts",
            Kind::Array(&Kind::Object(shape_exchange_artifact)),
        ),
        ("decision", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/planning_exploration.go SHA-256 b5b140a17abdc17d8fea13e91a9d3507b00fbc583c9364ffdb58153f7760d735
/// Frozen PlanningExploration field types; pure metadata, no IO or grant.
pub(super) fn shape_planning_exploration() -> Fields {
    &[
        ("scope", Kind::String),
        ("scope_revision", Kind::Int64),
        ("generation", Kind::Int64),
        ("observed_revision", Kind::Int64),
        ("request_decision", Kind::String),
        ("candidate_commit", Kind::String),
        ("request", Kind::Object(shape_planning_exploration_request)),
        ("entries", Kind::Array(&Kind::String)),
        ("sources", Kind::Array(&Kind::Object(shape_review_source))),
        ("sha256", Kind::String),
    ]
}
// internal/engine/planning_exploration.go SHA-256 b5b140a17abdc17d8fea13e91a9d3507b00fbc583c9364ffdb58153f7760d735
/// Frozen PlanningExplorationRequest field types; pure metadata, no IO or grant.
pub(super) fn shape_planning_exploration_request() -> Fields {
    &[
        ("mode", Kind::String),
        ("candidate_commit", Kind::String),
        ("directory", Kind::String),
        ("files", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/planning.go SHA-256 66ee75df2ce0490e2d362b23030ed033e4e8cf1db402c5448b4464e1af5f62eb
/// Frozen PlanningRejectionWait field types; pure metadata, no IO or grant.
pub(super) fn shape_planning_rejection_wait() -> Fields {
    &[("event_id", Kind::String), ("state_sha256", Kind::String)]
}
// internal/engine/planning.go SHA-256 66ee75df2ce0490e2d362b23030ed033e4e8cf1db402c5448b4464e1af5f62eb
/// Frozen PlanningScope field types; pure metadata, no IO or grant.
pub(super) fn shape_planning_scope() -> Fields {
    &[
        (
            "rejection_wait",
            Kind::Pointer(&Kind::Object(shape_planning_rejection_wait)),
        ),
        (
            "model_selection",
            Kind::Pointer(&Kind::Object(shape_role_model)),
        ),
        ("last_model", Kind::Pointer(&Kind::Object(shape_role_model))),
        (
            "delivery",
            Kind::Pointer(&Kind::Object(shape_planning_delivery)),
        ),
        (
            "workflow",
            Kind::Pointer(&Kind::Object(shape_agent_workflow)),
        ),
        ("max_tasks", Kind::Int64),
        ("max_activations", Kind::Int64),
        ("activations", Kind::Int64),
        ("id", Kind::String),
        ("parent", Kind::String),
        ("objective", Kind::String),
        ("requirements", Kind::Array(&Kind::String)),
        ("revision", Kind::Int64),
        ("generation", Kind::Int64),
        ("holder", Kind::String),
        ("lease_until", Kind::String),
        ("state", Kind::String),
    ]
}
// internal/engine/planning.go SHA-256 66ee75df2ce0490e2d362b23030ed033e4e8cf1db402c5448b4464e1af5f62eb
/// Frozen PlanningState field types; pure metadata, no IO or grant.
pub(super) fn shape_planning_state() -> Fields {
    &[
        (
            "quota_authorization",
            Kind::Pointer(&Kind::Object(shape_quota_authorization)),
        ),
        ("reviewer_required", Kind::Bool),
        (
            "reviewer",
            Kind::Pointer(&Kind::Object(shape_reviewer_config)),
        ),
        (
            "model_route",
            Kind::Pointer(&Kind::Object(shape_model_route)),
        ),
        ("human_review_authorized", Kind::String),
        (
            "checks",
            Kind::Map(&Kind::Array(&Kind::Object(shape_validation_control))),
        ),
        (
            "repository",
            Kind::Pointer(&Kind::Object(shape_managed_repository)),
        ),
        ("provider", Kind::String),
        ("provider_digest", Kind::String),
        ("activations", Kind::Int64),
        ("max_activations", Kind::Int64),
        ("failure", Kind::String),
        ("version", Kind::Int64),
        ("paused", Kind::Bool),
        ("max_tasks", Kind::Int64),
        ("max_scopes", Kind::Int64),
        ("max_depth", Kind::Int64),
        ("max_decisions", Kind::Int64),
        ("decisions", Kind::Int64),
        ("scopes", Kind::Array(&Kind::Object(shape_planning_scope))),
        ("inbox", Kind::Array(&Kind::Object(shape_planning_event))),
        ("judge", Kind::Pointer(&Kind::Object(shape_judge_state))),
    ]
}
// internal/engine/preflight.go SHA-256 f514435ab9c8227834ac4ac038d3adc9927af02111c2e517f5dfa0cba3ba0cba
/// Frozen PreflightCheck field types; pure metadata, no IO or grant.
pub(super) fn shape_preflight_check() -> Fields {
    &[
        ("name", Kind::String),
        ("status", Kind::String),
        ("detail", Kind::String),
    ]
}
// internal/engine/preflight.go SHA-256 f514435ab9c8227834ac4ac038d3adc9927af02111c2e517f5dfa0cba3ba0cba
/// Frozen PreflightResult field types; pure metadata, no IO or grant.
pub(super) fn shape_preflight_result() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("verdict", Kind::String),
        ("checked_at", Kind::String),
        ("valid_until", Kind::String),
        ("scope", Kind::String),
        ("provider", Kind::String),
        ("workspace", Kind::String),
        ("duration_ms", Kind::Int64),
        ("verification", Kind::String),
        ("capabilities", Kind::Map(&Kind::String)),
        ("checks", Kind::Array(&Kind::Object(shape_preflight_check))),
    ]
}
// internal/engine/product_plan.go SHA-256 61e4dba172381644c4b1177c68f027a13965ceecf1d98a64bcd2467f21f00f78
/// Frozen ProductPlan field types; pure metadata, no IO or grant.
pub(super) fn shape_product_plan() -> Fields {
    &[
        ("mode", Kind::String),
        ("journeys", Kind::Array(&Kind::Object(shape_user_journey))),
        ("stories", Kind::Array(&Kind::Object(shape_user_story))),
    ]
}
// internal/engine/score.go SHA-256 3d417b06da3753c2ad86bbaa061ea5409e275166956256b844a030e25192d922
/// Frozen Progress field types; pure metadata, no IO or grant.
pub(super) fn shape_progress() -> Fields {
    &[
        ("passed", Kind::Int64),
        ("applicable", Kind::Int64),
        ("percent", Kind::Pointer(&Kind::Float64)),
    ]
}
// internal/engine/project_profile.go SHA-256 f266bba43f3570daa22981d830544c62c937730ee3cf150c07ac74d5e92f90d8
/// Frozen ProjectContext field types; pure metadata, no IO or grant.
pub(super) fn shape_project_context() -> Fields {
    &[
        ("name", Kind::String),
        ("role", Kind::String),
        ("profile_sha256", Kind::String),
        ("source", Kind::String),
        ("sources", Kind::Array(&Kind::Object(shape_project_source))),
        ("sha256", Kind::String),
    ]
}
// internal/engine/project_memory.go SHA-256 16aa64873f1a26f102a9c1e26d4021425bf063ea7ff1b58c7634a423ec77070d
/// Frozen ProjectMemory field types; pure metadata, no IO or grant.
pub(super) fn shape_project_memory() -> Fields {
    &[
        ("id", Kind::String),
        ("imported", Kind::Bool),
        ("versions", Kind::Array(&Kind::Object(shape_memory_version))),
    ]
}
// internal/engine/project_profile.go SHA-256 f266bba43f3570daa22981d830544c62c937730ee3cf150c07ac74d5e92f90d8
/// Frozen ProjectSource field types; pure metadata, no IO or grant.
pub(super) fn shape_project_source() -> Fields {
    &[
        ("path", Kind::String),
        ("sha256", Kind::String),
        ("bytes", Kind::Int64),
    ]
}
// internal/engine/provider_cooldown.go SHA-256 934d6066800bcd6b4a1f351a63de5fd683faf171c2f7222e4a219cc67c572b7f
/// Frozen ProviderCooldown field types; pure metadata, no IO or grant.
pub(super) fn shape_provider_cooldown() -> Fields {
    &[
        ("provider", Kind::String),
        ("observed_at", Kind::String),
        ("resets_at", Kind::Int64),
        ("source", Kind::String),
        ("signal", Kind::String),
        ("cleared_at", Kind::String),
        ("clear_event", Kind::String),
        ("clear_reason", Kind::String),
        ("clear_actor", Kind::String),
        ("clear_account_change", Kind::Bool),
    ]
}
// internal/engine/provider_relay.go SHA-256 21a7733dcdbc225baa15b924b320d36b32ee5134434014e9acf71d51da945370
/// Frozen ProviderRelayDecision field types; pure metadata, no IO or grant.
pub(super) fn shape_provider_relay_decision() -> Fields {
    &[
        ("agent_id", Kind::String),
        ("action", Kind::String),
        ("provider", Kind::String),
        ("event_id", Kind::String),
        ("view_digest", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/quotas.go SHA-256 91138131f7215dfe4678d65ee1b451cb566f52f47ec0da13ddc6b9260aac7077
/// Frozen QuotaAuthorization field types; pure metadata, no IO or grant.
pub(super) fn shape_quota_authorization() -> Fields {
    &[
        ("before_scope_activations", Kind::Map(&Kind::Int64)),
        ("after_scope_activations", Kind::Map(&Kind::Int64)),
        ("before", Kind::Object(shape_quota_values)),
        ("after", Kind::Object(shape_quota_values)),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("reason", Kind::String),
    ]
}
// internal/engine/quotas.go SHA-256 91138131f7215dfe4678d65ee1b451cb566f52f47ec0da13ddc6b9260aac7077
/// Frozen QuotaValues field types; pure metadata, no IO or grant.
pub(super) fn shape_quota_values() -> Fields {
    &[
        ("review_context_tokens", Kind::Pointer(&Kind::Int64)),
        ("review_context_files", Kind::Pointer(&Kind::Int64)),
        ("review_source_bytes", Kind::Pointer(&Kind::Int64)),
        ("review_context_bytes", Kind::Pointer(&Kind::Int64)),
        ("review_delivery_files", Kind::Pointer(&Kind::Int64)),
        ("planning_scopes", Kind::Pointer(&Kind::Int64)),
        ("delegation_depth", Kind::Pointer(&Kind::Int64)),
        ("planning_tasks", Kind::Pointer(&Kind::Int64)),
        ("judge_cycles", Kind::Pointer(&Kind::Int64)),
        ("planning_activations", Kind::Int64),
        ("planning_decisions", Kind::Int64),
        ("review_calls", Kind::Pointer(&Kind::Int64)),
    ]
}
// internal/engine/recovered_result.go SHA-256 e584a63a06359e5e4f35f01317eb406d5d1c85396bc37d0bee182d10eb5aa088
/// Frozen RecoveredResult field types; pure metadata, no IO or grant.
pub(super) fn shape_recovered_result() -> Fields {
    &[
        ("proof_key", Kind::String),
        ("replaces_result", Kind::String),
        ("prior_review_id", Kind::String),
        ("event_id", Kind::String),
        ("request_sha256", Kind::String),
        ("agent_id", Kind::String),
        ("attempt_id", Kind::String),
        ("result_commit", Kind::String),
        ("tree", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("reason", Kind::String),
        ("original_process_status", Kind::String),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Frozen RecoveryState field types; pure metadata, no IO or grant.
pub(super) fn shape_recovery_state() -> Fields {
    &[
        ("category", Kind::String),
        ("cause_fingerprint", Kind::String),
        ("observed_category", Kind::String),
        ("observed_cause_fingerprint", Kind::String),
        ("operation_id", Kind::String),
        ("automatic_attempts_used", Kind::Int64),
        ("automatic_attempts_max", Kind::Int64),
        ("budget_remaining", Kind::Int64),
        ("next_eligible_at", Kind::String),
        ("disposition", Kind::String),
    ]
}
// internal/engine/agent_reported_model.go SHA-256 4650883eb58441260a7e6fb172635276c04772471ef13b5beef4d1d594b5d43e
/// Frozen ReportedModel field types; pure metadata, no IO or grant.
pub(super) fn shape_reported_model() -> Fields {
    &[
        ("model", Kind::String),
        ("source", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/retex_model.go SHA-256 ff4f37000ad5f3da5c29511a96378095ee3cc98c45c4699c0a45936d25600456
/// Frozen Retex field types; pure metadata, no IO or grant.
pub(super) fn shape_retex() -> Fields {
    &[
        ("impact", Kind::String),
        ("opportunity_kind", Kind::String),
        ("source_kind", Kind::String),
        ("id", Kind::String),
        ("source", Kind::Object(shape_dialogue_ref)),
        ("problem", Kind::String),
        ("nature", Kind::String),
        ("proposal", Kind::String),
        ("effort", Kind::String),
        ("risk", Kind::String),
        ("criteria", Kind::String),
        ("proof", Kind::String),
        ("status", Kind::String),
        ("task", Kind::String),
        ("at", Kind::String),
        ("export_pending", Kind::Bool),
        ("export_hash", Kind::String),
        ("export_error", Kind::String),
        ("lesson", Kind::String),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen Revalidation field types; pure metadata, no IO or grant.
pub(super) fn shape_revalidation() -> Fields {
    &[
        ("previous_artifacts", Kind::Map(&Kind::String)),
        ("config_digest", Kind::String),
        ("report", Kind::String),
        ("report_hash", Kind::String),
    ]
}
// internal/engine/managed_review_coordination.go SHA-256 000f14fba98ff32d8906e0411f260a03f543daa1e0e778488e8754fc308970f1
/// Frozen ReviewCoordinationLot field types; pure metadata, no IO or grant.
pub(super) fn shape_review_coordination_lot() -> Fields {
    &[
        ("id", Kind::String),
        ("kind", Kind::String),
        ("objective", Kind::String),
        ("files", Kind::Array(&Kind::String)),
        ("criteria", Kind::Array(&Kind::String)),
        ("depends", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/managed_review_coordination.go SHA-256 000f14fba98ff32d8906e0411f260a03f543daa1e0e778488e8754fc308970f1
/// Frozen ReviewCoordinationProposal field types; pure metadata, no IO or grant.
pub(super) fn shape_review_coordination_proposal() -> Fields {
    &[
        ("candidate_commit", Kind::String),
        ("evidence_sha256", Kind::String),
        (
            "lots",
            Kind::Array(&Kind::Object(shape_review_coordination_lot)),
        ),
        ("final_review", Kind::String),
    ]
}
// internal/engine/managed_review_coordination.go SHA-256 000f14fba98ff32d8906e0411f260a03f543daa1e0e778488e8754fc308970f1
/// Frozen ReviewCoordinationRecord field types; pure metadata, no IO or grant.
pub(super) fn shape_review_coordination_record() -> Fields {
    &[
        ("proposal", Kind::Object(shape_review_coordination_proposal)),
        ("sha256", Kind::String),
        ("scope", Kind::String),
        ("decision", Kind::String),
        ("state", Kind::String),
        ("order", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/independent_review.go SHA-256 8aaa695b6bcc95e61684c0cbc099dd34be73cb18374a14c3a7ea8b2d604f20f5
/// Frozen ReviewCriterion field types; pure metadata, no IO or grant.
pub(super) fn shape_review_criterion() -> Fields {
    &[
        ("index", Kind::Int64),
        ("verdict", Kind::String),
        ("evidence", Kind::String),
    ]
}
// internal/engine/managed_review_sources.go SHA-256 e4af44691e4d1d2da8b7792a231e2832203415367b0efc2e12d1dbfcf39136b8
/// Frozen ReviewSource field types; pure metadata, no IO or grant.
pub(super) fn shape_review_source() -> Fields {
    &[
        ("path", Kind::String),
        ("git_blob", Kind::String),
        ("sha256", Kind::String),
        ("bytes", Kind::Int64),
        ("content", Kind::String),
    ]
}
// internal/engine/independent_review.go SHA-256 8aaa695b6bcc95e61684c0cbc099dd34be73cb18374a14c3a7ea8b2d604f20f5
/// Frozen ReviewerConfig field types; pure metadata, no IO or grant.
pub(super) fn shape_reviewer_config() -> Fields {
    &[
        ("input_context_tokens", Kind::Int64),
        ("max_context_files", Kind::Pointer(&Kind::Int64)),
        ("max_source_bytes", Kind::Pointer(&Kind::Int64)),
        ("max_context_bytes", Kind::Pointer(&Kind::Int64)),
        ("max_files", Kind::Pointer(&Kind::Int64)),
        (
            "model_selection",
            Kind::Pointer(&Kind::Object(shape_role_model)),
        ),
        ("timeout_seconds", Kind::Int64),
        ("failure", Kind::String),
        ("provider", Kind::String),
        ("provider_digest", Kind::String),
        (
            "model_route",
            Kind::Pointer(&Kind::Object(shape_model_route)),
        ),
        ("max_calls", Kind::Int64),
        ("calls", Kind::Int64),
        ("authorized", Kind::String),
    ]
}
// internal/engine/role_model.go SHA-256 5ba48e5ebf640d7dc37d08f00544db18e57a58e6f6d8d24ca282489d4455a72d
/// Frozen RoleModel field types; pure metadata, no IO or grant.
pub(super) fn shape_role_model() -> Fields {
    &[
        ("provider", Kind::String),
        ("provider_digest", Kind::String),
        ("route", Kind::Pointer(&Kind::Object(shape_model_route))),
        ("actor", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/run_limits.go SHA-256 6bb48452ed56abe5984c4759ca2105aaf2b15098eaa2511a5de8c8a88daf9157
/// Frozen RunLimits field types; pure metadata, no IO or grant.
pub(super) fn shape_run_limits() -> Fields {
    &[
        ("observation_mode", Kind::Int64),
        ("silence_seconds", Kind::Int64),
        ("tool_seconds", Kind::Int64),
        ("max_tool_calls", Kind::Int64),
        ("max_repeated_calls", Kind::Int64),
        ("max_consecutive_errors", Kind::Int64),
    ]
}
// internal/engine/run_limits_admin.go SHA-256 dcb133d944ac0ed56283866a370208fc74f43f6be0924a5e1cfb5e8394c15ca4
/// Frozen RunLimitsConfigEntry field types; pure metadata, no IO or grant.
pub(super) fn shape_run_limits_config_entry() -> Fields {
    &[
        ("scope", Kind::String),
        ("mission_id", Kind::String),
        ("scope_key", Kind::String),
        ("values", Kind::Object(shape_run_limits)),
        ("revision", Kind::Int64),
        ("actor", Kind::String),
        ("reason", Kind::String),
        ("updated", Kind::String),
    ]
}
// internal/engine/run_supervision.go SHA-256 c4b53f49951102f8fe9778ac3b8b8a8f1868ef63ba38b601bd8e07343b84c01f
/// Frozen RunObservation field types; pure metadata, no IO or grant.
pub(super) fn shape_run_observation() -> Fields {
    &[
        (
            "diagnostic",
            Kind::Pointer(&Kind::Object(shape_attempt_diagnostic)),
        ),
        ("agent_id", Kind::String),
        ("trigger", Kind::String),
        ("threshold_seconds", Kind::Int64),
        ("progress", Kind::Object(shape_agent_progress)),
    ]
}
// internal/engine/dialogue_context.go SHA-256 510d596e9f20ee3e27c3791b3ebf1ef2c96cf2d3eb348907b5e76725f99bf079
/// Frozen SavedContext field types; pure metadata, no IO or grant.
pub(super) fn shape_saved_context() -> Fields {
    &[
        ("agent", Kind::String),
        ("at", Kind::String),
        ("prompt", Kind::String),
        ("manifest", Kind::Object(shape_context_manifest)),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen Task field types; pure metadata, no IO or grant.
pub(super) fn shape_task() -> Fields {
    &[
        (
            "memory_selections",
            Kind::Array(&Kind::Object(shape_memory_reference)),
        ),
        (
            "memory_decision",
            Kind::Pointer(&Kind::Object(shape_memory_selection_decision)),
        ),
        ("memory_selection_imported", Kind::Bool),
        ("instruction_base", Kind::Pointer(&Kind::String)),
        ("instruction_operator_base", Kind::Pointer(&Kind::String)),
        (
            "instruction_revisions",
            Kind::Array(&Kind::Object(shape_instruction_revision)),
        ),
        (
            "provider_relay_decision",
            Kind::Pointer(&Kind::Object(shape_provider_relay_decision)),
        ),
        (
            "legacy_report_submissions",
            Kind::Array(&Kind::Object(shape_attempt)),
        ),
        (
            "review_coordination",
            Kind::Pointer(&Kind::Object(shape_review_coordination_record)),
        ),
        ("restarts", Kind::Array(&Kind::Object(shape_task_restart))),
        (
            "model_selection",
            Kind::Pointer(&Kind::Object(shape_task_model)),
        ),
        (
            "requalifications",
            Kind::Array(&Kind::Object(shape_historical_requalification)),
        ),
        (
            "integration_retries",
            Kind::Array(&Kind::Object(shape_integration_retry)),
        ),
        (
            "batch_review_resume",
            Kind::Pointer(&Kind::Object(shape_independent_review)),
        ),
        (
            "recovered_result",
            Kind::Pointer(&Kind::Object(shape_recovered_result)),
        ),
        (
            "corrective_recovery",
            Kind::Pointer(&Kind::Object(shape_corrective_recovery)),
        ),
        (
            "previous_reviews",
            Kind::Array(&Kind::Object(shape_independent_review)),
        ),
        (
            "independent_review",
            Kind::Pointer(&Kind::Object(shape_independent_review)),
        ),
        ("scope_id", Kind::String),
        ("requirements", Kind::Array(&Kind::String)),
        (
            "validation_policy",
            Kind::Pointer(&Kind::Object(shape_validation_policy)),
        ),
        (
            "automatic_validation",
            Kind::Pointer(&Kind::Object(shape_automatic_validation)),
        ),
        ("evidence_stale_reason", Kind::String),
        ("launch_held", Kind::Bool),
        (
            "launch_profile",
            Kind::Pointer(&Kind::Object(shape_launch_profile)),
        ),
        (
            "revalidation",
            Kind::Pointer(&Kind::Object(shape_revalidation)),
        ),
        ("plan_checks", Kind::Map(&Kind::String)),
        ("plan_brief_hash", Kind::String),
        ("plan_role", Kind::String),
        ("planning_retry", Kind::Bool),
        ("plan_max_attempts", Kind::Int64),
        ("plan_tool_limit", Kind::Int64),
        ("contexts", Kind::Array(&Kind::Object(shape_saved_context))),
        (
            "answers",
            Kind::Array(&Kind::Object(shape_brainstorm_answer)),
        ),
        ("question", Kind::String),
        ("response", Kind::String),
        ("response_error", Kind::String),
        ("brainstorm", Kind::Bool),
        (
            "manual_override",
            Kind::Pointer(&Kind::Object(shape_manual_override)),
        ),
        ("id", Kind::String),
        ("title", Kind::String),
        ("owner", Kind::String),
        ("deliverable", Kind::String),
        ("criteria", Kind::Array(&Kind::String)),
        ("depends", Kind::Array(&Kind::String)),
        ("status", Kind::String),
        ("blocker", Kind::String),
        ("next", Kind::String),
        ("attempts", Kind::Array(&Kind::Object(shape_attempt))),
        ("gate", Kind::Pointer(&Kind::Object(shape_gate_record))),
    ]
}
// internal/engine/task_model.go SHA-256 7ad0ed951d0f3a0611a2f57074cef3194f385ea94f1aba37788a09dcf6524055
/// Frozen TaskModel field types; pure metadata, no IO or grant.
pub(super) fn shape_task_model() -> Fields {
    &[
        ("provider", Kind::String),
        ("route", Kind::Object(shape_model_route)),
        ("actor", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/task_restart.go SHA-256 9444aeb059eeeac5970a44f9cb529d81ae9ad18f8370588e3cf188e7fa29607d
/// Frozen TaskRestart field types; pure metadata, no IO or grant.
pub(super) fn shape_task_restart() -> Fields {
    &[
        ("event_id", Kind::String),
        ("previous_attempt", Kind::String),
        ("base_candidate", Kind::String),
        ("reason", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("previous_blocker", Kind::String),
        (
            "previous_recovered_result",
            Kind::Pointer(&Kind::Object(shape_recovered_result)),
        ),
    ]
}
// internal/engine/budgets.go SHA-256 289375d15abafefb5c781a8baad124f0580b9d357c61b12ce22c4c098e1a1814
/// Frozen Usage field types; pure metadata, no IO or grant.
pub(super) fn shape_usage() -> Fields {
    &[
        ("cache_read_input_tokens", Kind::Pointer(&Kind::Int64)),
        ("cache_creation_input_tokens", Kind::Pointer(&Kind::Int64)),
        ("cached_input_tokens", Kind::Pointer(&Kind::Int64)),
        ("provider_reported_cost_usd", Kind::Pointer(&Kind::Float64)),
        ("input_tokens", Kind::Int64),
        ("output_tokens", Kind::Int64),
        ("source", Kind::String),
        ("scope", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/product_plan.go SHA-256 61e4dba172381644c4b1177c68f027a13965ceecf1d98a64bcd2467f21f00f78
/// Frozen UserJourney field types; pure metadata, no IO or grant.
pub(super) fn shape_user_journey() -> Fields {
    &[
        ("id", Kind::String),
        ("title", Kind::String),
        ("goal", Kind::String),
        ("story_ids", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/product_plan.go SHA-256 61e4dba172381644c4b1177c68f027a13965ceecf1d98a64bcd2467f21f00f78
/// Frozen UserStory field types; pure metadata, no IO or grant.
pub(super) fn shape_user_story() -> Fields {
    &[
        ("id", Kind::String),
        ("title", Kind::String),
        ("user", Kind::String),
        ("value", Kind::String),
        ("criteria", Kind::Array(&Kind::String)),
        ("complexity", Kind::Int64),
        ("risk", Kind::String),
        ("depends", Kind::Array(&Kind::String)),
        ("task_ids", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen ValidationControl field types; pure metadata, no IO or grant.
pub(super) fn shape_validation_control() -> Fields {
    &[
        ("environment_exit_codes", Kind::Array(&Kind::Int64)),
        ("review_output", Kind::Bool),
        ("inputs", Kind::Array(&Kind::String)),
        ("id", Kind::String),
        ("command", Kind::Array(&Kind::String)),
        ("criteria", Kind::Array(&Kind::Int64)),
        ("justification", Kind::String),
        ("dir", Kind::String),
        ("timeout_seconds", Kind::Int64),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen ValidationControlResult field types; pure metadata, no IO or grant.
pub(super) fn shape_validation_control_result() -> Fields {
    &[
        (
            "command_runtime_limits",
            Kind::Pointer(&Kind::Object(shape_run_limits_config_entry)),
        ),
        (
            "runtime_limits",
            Kind::Pointer(&Kind::Object(shape_run_limits_config_entry)),
        ),
        ("environment_failure", Kind::Bool),
        ("review_output", Kind::String),
        ("output_bytes", Kind::Int64),
        ("review_output_truncated", Kind::Bool),
        ("id", Kind::String),
        ("command", Kind::Array(&Kind::String)),
        ("executed", Kind::Bool),
        ("passed", Kind::Bool),
        ("exit_code", Kind::Int64),
        ("output_sha256", Kind::String),
        ("summary", Kind::String),
        ("started_at", Kind::String),
        ("finished_at", Kind::String),
        ("wall_duration_ms", Kind::Pointer(&Kind::Int64)),
        ("cpu_duration_ms", Kind::Pointer(&Kind::Int64)),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen ValidationPolicy field types; pure metadata, no IO or grant.
pub(super) fn shape_validation_policy() -> Fields {
    &[
        ("mode", Kind::String),
        (
            "controls",
            Kind::Array(&Kind::Object(shape_validation_control)),
        ),
        ("authorized_at", Kind::String),
        ("actor", Kind::String),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen ValidationSummary field types; pure metadata, no IO or grant.
pub(super) fn shape_validation_summary() -> Fields {
    &[
        ("current_state", Kind::String),
        ("validated_now", Kind::Int64),
        ("accepted_historically", Kind::Int64),
        ("to_revalidate", Kind::Int64),
    ]
}
// internal/engine/assist_contract.go SHA-256 8cb834a8774d5b1d97ce7796f746ae9344facb10cb26c71ed13cfeab18b8c7a3
/// Frozen VisibleSlice field types; pure metadata, no IO or grant.
pub(super) fn shape_visible_slice() -> Fields {
    &[
        ("offset", Kind::Int64),
        ("count", Kind::Int64),
        ("total", Kind::Int64),
    ]
}
// internal/engine/tracking.go SHA-256 66198f09a6f5b2b8459d5e27c46cd74b61fe689c9875c916a24789d9683b78df
/// Frozen Visit field types; pure metadata, no IO or grant.
pub(super) fn shape_visit() -> Fields {
    &[
        ("revision", Kind::Int64),
        ("at", Kind::String),
        ("operator", Kind::String),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
/// Frozen Work field types; pure metadata, no IO or grant.
pub(super) fn shape_work() -> Fields {
    &[
        (
            "requirement_prerequisites",
            Kind::Map(&Kind::Array(&Kind::String)),
        ),
        (
            "planning",
            Kind::Pointer(&Kind::Object(shape_planning_state)),
        ),
        (
            "launch_profile",
            Kind::Pointer(&Kind::Object(shape_launch_profile)),
        ),
        ("plans", Kind::Array(&Kind::Object(shape_approved_plan))),
        ("retex", Kind::Array(&Kind::Object(shape_retex))),
        (
            "project_memory",
            Kind::Array(&Kind::Object(shape_project_memory)),
        ),
        (
            "planning_brief",
            Kind::Pointer(&Kind::Object(shape_planning_brief)),
        ),
        ("schema_version", Kind::Int64),
        ("id", Kind::String),
        ("revision", Kind::Int64),
        ("title", Kind::String),
        ("objective", Kind::String),
        ("scope", Kind::String),
        ("criteria", Kind::Array(&Kind::String)),
        ("next", Kind::String),
        ("summary", Kind::String),
        ("memory", Kind::Array(&Kind::String)),
        ("created", Kind::String),
        ("updated", Kind::String),
        ("git", Kind::Object(shape_git_state)),
        ("tasks", Kind::Array(&Kind::Object(shape_task))),
    ]
}
// internal/engine/lifecycle.go SHA-256 a54975cd4bbff02f82b08ed64434550e94018862fa4a1c76fbc9fa4ca007a31b
/// Frozen lifecycleCell field types; pure metadata, no IO or grant.
pub(super) fn shape_lifecycle_cell() -> Fields {
    &[("kind", Kind::String), ("value", Kind::String)]
}
// internal/engine/lifecycle.go SHA-256 a54975cd4bbff02f82b08ed64434550e94018862fa4a1c76fbc9fa4ca007a31b
/// Frozen lifecycleTable field types; pure metadata, no IO or grant.
pub(super) fn shape_lifecycle_table() -> Fields {
    &[
        ("name", Kind::String),
        ("columns", Kind::Array(&Kind::String)),
        (
            "rows",
            Kind::Array(&Kind::Array(&Kind::Object(shape_lifecycle_cell))),
        ),
    ]
}
// internal/engine/managed_review_citation_feedback.go SHA-256 350b8c70e19c52663cd835ac2e3ca7e07f8031776ba0a443e3e0c216b862db61
/// Frozen managedReviewCitationDiagnostic field types; pure metadata, no IO or grant.
pub(super) fn shape_managed_review_citation_diagnostic() -> Fields {
    &[
        ("version", Kind::Int64),
        ("review", Kind::String),
        ("candidate_commit", Kind::String),
        ("call_id", Kind::String),
        ("reply_sha256", Kind::String),
        (
            "issues",
            Kind::Array(&Kind::Object(shape_managed_review_citation_issue)),
        ),
    ]
}
// internal/engine/managed_review_citation_feedback.go SHA-256 350b8c70e19c52663cd835ac2e3ca7e07f8031776ba0a443e3e0c216b862db61
/// Frozen managedReviewCitationIssue field types; pure metadata, no IO or grant.
pub(super) fn shape_managed_review_citation_issue() -> Fields {
    &[
        ("packet", Kind::Int64),
        ("artifact", Kind::Int64),
        ("need_index", Kind::Int64),
        ("rejected_quote", Kind::String),
        ("quote_sha256", Kind::String),
        (
            "originals",
            Kind::Array(&Kind::Object(shape_managed_review_citation_original)),
        ),
    ]
}
// internal/engine/managed_review_citation_feedback.go SHA-256 350b8c70e19c52663cd835ac2e3ca7e07f8031776ba0a443e3e0c216b862db61
/// Frozen managedReviewCitationOriginal field types; pure metadata, no IO or grant.
pub(super) fn shape_managed_review_citation_original() -> Fields {
    &[
        ("name", Kind::String),
        ("sha256", Kind::String),
        ("line", Kind::Int64),
        ("byte_offset", Kind::Int64),
        ("original_text", Kind::String),
        ("truncated", Kind::Bool),
    ]
}
// internal/engine/review_failure_routing.go SHA-256 8fc0398e804a835b392fce4d3219c594e6e70604766789fa0e540fb637d71bf9
/// Frozen reviewFailureRouting field types; pure metadata, no IO or grant.
pub(super) fn shape_review_failure_routing() -> Fields {
    &[
        ("result_event", Kind::String),
        ("review_id", Kind::String),
        ("attempt", Kind::String),
        ("candidate_commit", Kind::String),
        ("failure_code", Kind::String),
        ("owner", Kind::String),
        ("operation", Kind::String),
        ("producer_correction_required", Kind::Bool),
    ]
}
