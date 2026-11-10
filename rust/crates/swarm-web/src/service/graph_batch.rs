//! Protected native graph preview/application; transport custody stays shared.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request, StatusCode};
use serde::Deserialize;
use swarm_engine::{AgentLaunchError, RootStore, RootStoreError, WorkEngineError};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    work: String,
    request: swarm_engine::GraphBatchRequest,
}
impl NativeHttpService {
    /// Use authenticated POST/CSRF and the opened datastore binding for a strict
    /// native graph batch. Preview does not write; apply uses the common Root
    /// transaction/replay boundary. Known recorded Work survives encoding/close
    /// failure; uncertainty is never converted into an empty successful reply.
    pub(super) fn graph_batch(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if request.method() != Method::POST {
            return Err(E::Method("POST"));
        }
        if request.uri().query().is_some() {
            return Err(E::Refused("graph-batch-query-invalid"));
        }
        swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
        let envelope: Envelope = serde_json::from_slice(request.body())
            .map_err(|_| E::Refused("graph-batch-envelope-invalid"))?;
        if !swarm_identity_contract::name(envelope.work.as_bytes()) {
            return Err(E::Refused("graph-batch-work-invalid"));
        }
        let raw = swarm_engine::encode_model(&envelope.request).map_err(E::Model)?;
        swarm_engine::decode_graph_batch(&raw).map_err(|e| {
            root(
                "graph-batch-input",
                RootStoreError::Mutation(WorkEngineError::Graph(e)),
            )
        })?;
        let apply = request.uri().path() == "/api/v1/graph-batch/apply";
        let mut store = RootStore::open(&self.root, apply).map_err(|e| root("open", e))?;
        let mut recorded = None;
        let result = (|| {
            self.check_binding(&store)?;
            if apply {
                let outcome = store
                    .execute_external(
                        &envelope.work,
                        "graph.batch",
                        &raw,
                        &mut swarm_engine::SystemEnvironment,
                    )
                    .map_err(|e| root("graph-batch-apply", e))?;
                let replayed = outcome.result.replayed;
                let work = outcome.result.work;
                recorded = Some((work.id.clone(), work.revision));
                let applied_revision = envelope
                    .request
                    .expected_revision
                    .checked_add(1)
                    .ok_or(E::Refused("graph-applied-revision-overflow"))?;
                let bytes=serde_json::to_vec(&serde_json::json!({"work":work,"event_id":envelope.request.event_id,"applied_revision":applied_revision,"replayed":replayed})).map_err(E::ResponseJson)?;
                let mut body = RouteBody::plain(bytes);
                body.recorded_work = recorded.clone();
                Ok(body)
            } else {
                let preview = store
                    .preview_graph_batch(&envelope.work, &raw, &mut swarm_engine::SystemEnvironment)
                    .map_err(|e| root("graph-batch-preview", e))?;
                swarm_engine::encode_model(&preview)
                    .map(RouteBody::plain)
                    .map_err(E::Model)
            }
        })();
        match (close(result, store.close()), recorded) {
            (Err(primary), Some((work, revision))) => Err(E::Recorded {
                primary: Box::new(primary),
                work,
                revision,
            }),
            (r, _) => r,
        }
    }
}
/// Classify only typed caller/rights/conflict refusals. Stored-source, SQL,
/// rollback/release and observed publication failures retain server-error status.
pub(super) fn status(operation: &str, error: &RootStoreError) -> Option<StatusCode> {
    if !matches!(
        operation,
        "graph-batch-input" | "graph-batch-preview" | "graph-batch-apply"
    ) {
        return None;
    }
    let RootStoreError::Mutation(error) = error else {
        return None;
    };
    Some(match error {
        WorkEngineError::Graph(AgentLaunchError::Refused(
            "graph-authorization-required" | "graph-authorization-scope",
        )) => StatusCode::FORBIDDEN,
        WorkEngineError::Graph(AgentLaunchError::Refused(
            "graph-preview-stale"
            | "graph-definition-stale"
            | "graph-undo-not-last-batch"
            | "graph-batch-active-work"
            | "graph-task-not-editable"
            | "graph-undo-task-used",
        )) => StatusCode::CONFLICT,
        WorkEngineError::Graph(AgentLaunchError::Refused(
            "graph-batch-hierarchical-work" | "graph-batch-native-platform-unavailable",
        )) => StatusCode::NOT_IMPLEMENTED,
        WorkEngineError::Graph(AgentLaunchError::Refused(
            "graph-batch-envelope"
            | "graph-preview-digest-invalid"
            | "graph-change-empty"
            | "graph-change-invalid"
            | "graph-delete-requires-compensation"
            | "graph-criteria-invalid"
            | "graph-undo-already-compensated"
            | "graph-undo-event-missing",
        )) => StatusCode::UNPROCESSABLE_ENTITY,
        WorkEngineError::Graph(_) if operation == "graph-batch-input" => StatusCode::BAD_REQUEST,
        WorkEngineError::Guard(swarm_engine::MutationGuardError::Revision { .. }) => {
            StatusCode::CONFLICT
        }
        WorkEngineError::Task(_) | WorkEngineError::Definition(_) => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        _ => return None,
    })
}

/// Expose static definition-refusal reasons only on the new native graph route.
/// No request, SQL, file path or secondary cleanup data is rendered. This does
/// not reinterpret publication uncertainty as a harmless caller rejection.
pub(super) fn public_reason(error: &E) -> Option<&'static str> {
    let E::Root { operation, cause } = error else {
        return None;
    };
    if !matches!(
        *operation,
        "graph-batch-input" | "graph-batch-preview" | "graph-batch-apply"
    ) {
        return None;
    }
    match cause.as_ref() {
        RootStoreError::Mutation(WorkEngineError::Task(e)) => Some(e.reason()),
        RootStoreError::Mutation(WorkEngineError::Definition(e)) => Some(e.reason()),
        RootStoreError::Mutation(WorkEngineError::Graph(AgentLaunchError::Refused(reason))) => {
            Some(reason)
        }
        _ => None,
    }
}
