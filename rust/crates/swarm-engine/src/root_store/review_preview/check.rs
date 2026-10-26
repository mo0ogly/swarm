//! Read-only supplied managed output validation under current candidate custody.
use super::{task_validation, RootStore, TaskReviewPreview, E};
use crate::{AgentLaunchError as A, managed_review_output::ManagedReviewOutputStream, managed_review_protocol::ManagedReviewProtocol};
use serde::{Deserialize, Serialize};

/// Exact native request plus the original materialized prompt digest and supplied
/// JSONL bytes. No actor, provider, commands or candidate authority is accepted.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: i64,
    event_id: String,
    expected_revision: i64,
    task_id: String,
    prompt_sha256: String,
    output: String,
}

/// Checked supplied opinion data, not a native provider execution or permission.
/// Private evidence has no Debug representation. All false flags describe absent
/// effects, not configurable policy; criterion truth and reviewer identity remain
/// unestablished even when every supplied criterion says pass.
#[derive(Serialize)]
pub struct TaskReviewCheck {
    pub schema_version: i64,
    pub preview: TaskReviewPreview,
    pub prompt_sha256: String,
    pub context_sha256: String,
    pub catalog_sha256: String,
    pub response_sha256: String,
    pub opinion_state: crate::ReviewReplyState,
    pub tasks: Vec<crate::ManagedTaskReview>,
    pub supplied_output_checked: bool,
    pub provider_started: bool,
    pub review_recorded: bool,
    pub native_execution_proven: bool,
    pub reviewer_authenticated: bool,
    pub verdict_applicable: bool,
    pub task_accepted: bool,
}

impl RootStore {
    /// Check supplied JSONL against fresh complete managed inputs and their exact
    /// original prompt digest. Uses current saved transport/domain limits while
    /// retaining candidate custody, snapshot, final refresh and explicit cleanup.
    /// Invalid requests, stale source/prompt, ordinary tasks, tool events, partial
    /// streams/cohorts, malformed citations and limits refuse with typed causes.
    /// No provider, writes, debit, review record or acceptance occurs. Supplied
    /// terminal framing proves neither actual provider EOF nor reviewer identity.
    pub fn check_task_review_output(&self, work: &str, raw: &[u8]) -> Result<TaskReviewCheck, E> {
        let launch = |e| E::Launch(Box::new(e));
        let boundaries = crate::agent_tool_guard::validated_json(raw)
            .map_err(|e| launch(A::caused("review-check-request-invalid", e)))?;
        crate::review_output::canonical_fields(raw, &boundaries,
            &["schema_version", "event_id", "expected_revision", "task_id", "prompt_sha256", "output"])
            .map_err(launch)?;
        // Type-check original values before serde can render a supplied scalar in
        // a diagnostic. Missing/null fields still refuse in the required DTO.
        use crate::model_wire::Kind;
        crate::model_wire::project_mode(raw, &[
            ("schema_version", Kind::Int64), ("event_id", Kind::String),
            ("expected_revision", Kind::Int64), ("task_id", Kind::String),
            ("prompt_sha256", Kind::String), ("output", Kind::String),
        ], true).map_err(|e| launch(A::caused("review-check-request-invalid", e)))?;
        let request: Request = swarm_wire_json::decode(raw)
            .map_err(|e| launch(A::caused("review-check-request-invalid", e)))?;
        if !swarm_identity_contract::digest(request.prompt_sha256.as_bytes()) {
            return Err(E::Refused("review-check-prompt-digest-invalid"));
        }
        let native = task_validation::Request {
            schema_version: request.schema_version,
            event_id: request.event_id,
            expected_revision: request.expected_revision,
            task_id: request.task_id,
        };
        let canonical = serde_json::to_vec(&native)
            .map_err(|e| launch(A::caused("review-check-request-invalid", e)))?;
        self.with_prepared_review(work, &canonical, |tx, prepared, candidate| {
            let settings = prepared.settings.clone();
            let inputs = self.assemble_review_inputs(tx, prepared, candidate)?;
            if inputs.prompt_sha256 != request.prompt_sha256 {
                return Err(E::Refused("review-check-prompt-stale"));
            }
            // Decode actual JSON bytes so opaque GoRaw receipts retain the same
            // representation that the public prompt/catalogue digest binds.
            let protocol_bytes = crate::encode_model(&inputs.managed_protocol
                .ok_or(E::Refused("review-check-managed-context-required"))?)
                .map_err(|e| launch(A::caused("review-check-protocol-invalid", e)))?;
            let protocol: ManagedReviewProtocol = serde_json::from_slice(&protocol_bytes)
                .map_err(|e| launch(A::caused("review-check-protocol-invalid", e)))?;
            let mut stream = ManagedReviewOutputStream::from_saved_settings(&settings.values, &protocol)
                .map_err(launch)?;
            stream.feed(request.output.as_bytes()).map_err(launch)?;
            let checked = stream.finish().map_err(launch)?;
            Ok(TaskReviewCheck {
                schema_version: 1,
                preview: inputs.preview,
                prompt_sha256: inputs.prompt_sha256,
                context_sha256: protocol.context_sha256,
                catalog_sha256: protocol.catalog_sha256,
                response_sha256: crate::provider_configuration::configuration_digest(&checked.transport.response),
                opinion_state: checked.review.state,
                tasks: checked.review.tasks,
                supplied_output_checked: true,
                provider_started: false,
                review_recorded: false,
                native_execution_proven: false,
                reviewer_authenticated: false,
                verdict_applicable: false,
                task_accepted: false,
            })
        })
    }
}
