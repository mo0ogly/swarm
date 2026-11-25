//! Protected task action file. The same representation serves the dialog and export.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;

impl NativeHttpService {
    /// Read a task action file under the opened Root and views policy.
    /// GET/HEAD require one Work and task key. Invalid queries, missing rows
    /// and storage/serialization failures are distinct refusals. No visit,
    /// process, review, or file publication is performed.
    pub(super) fn task_actions(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        let mut work = None;
        let mut task = None;
        for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
            let slot = match key.as_ref() {
                "work" => &mut work,
                "task" => &mut task,
                _ => return Err(E::Refused("task-actions-query-invalid")),
            };
            if slot.replace(value.into_owned()).is_some() {
                return Err(E::Refused("task-actions-query-invalid"));
            }
        }
        let (Some(work), Some(task)) = (work, task) else {
            return Err(E::Refused("task-actions-query-invalid"));
        };
        if !request.body().is_empty()
            || !swarm_identity_contract::name(work.as_bytes())
            || !swarm_identity_contract::name(task.as_bytes())
        {
            return Err(E::Refused("task-actions-query-invalid"));
        }
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let result = (|| {
            self.check_binding(&store)?;
            let data = store
                .task_actions(&work, &task)
                .map_err(|cause| root("task-actions-read", cause))?;
            serde_json::to_vec(&data)
                .map(RouteBody::plain)
                .map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}

impl NativeHttpService {
    /// GET/HEAD of the task acceptance precheck, under the ordinary cockpit
    /// read permission. The query identifies one Work, task and revision.
    /// Returns an eligible observation or a typed refusal. No write permit.
    pub(super) fn task_human_review_preview(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        let mut work = None;
        let mut task = None;
        let mut revision = None;
        let invalid = || E::Refused("task-review-query-invalid");
        for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
            let slot = match key.as_ref() {
                "work" => &mut work,
                "task" => &mut task,
                "revision" => &mut revision,
                _ => return Err(invalid()),
            };
            if slot.replace(value.into_owned()).is_some() {
                return Err(invalid());
            }
        }
        let (Some(work), Some(task), Some(revision)) = (work, task, revision) else {
            return Err(invalid());
        };
        let revision = revision.parse::<i64>().ok().filter(|n| *n > 0).ok_or_else(invalid)?;
        if !request.body().is_empty()
            || !swarm_identity_contract::name(work.as_bytes())
            || !swarm_identity_contract::name(task.as_bytes())
        {
            return Err(invalid());
        }
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let result = (|| {
            self.check_binding(&store)?;
            store.preview_task_human_review(&work, &task, revision)
                .map_err(|cause| root("task-review-preview", cause))?;
            serde_json::to_vec(&serde_json::json!({
                "schema_version": 1, "work": work, "task": task, "revision": revision, "eligible": true
            })).map(RouteBody::plain).map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}

/// Safe static diagnostics from the engine for the acceptance precheck or an
/// ordinary task mutation. Typed causes stay available to the service. A
/// late/uncertain failure retains its normal error and recorded-state metadata.
pub(super) fn public_reason(error: &E) -> Option<&'static str> {
    use swarm_engine::{RootStoreError, TaskLifecycleError, WorkEngineError};
    let E::Root { operation: "task-review-preview" | "mutate", cause } = error else {
        return None;
    };
    match cause.as_ref() {
        RootStoreError::Mutation(WorkEngineError::Lifecycle(TaskLifecycleError::Gate(e))) => Some(e.reason()),
        RootStoreError::Mutation(WorkEngineError::Lifecycle(TaskLifecycleError::Review(e))) => Some(e.reason()),
        RootStoreError::Mutation(WorkEngineError::Lifecycle(e)) => Some(e.reason()),
        RootStoreError::Mutation(WorkEngineError::Guard(e)) => Some(e.reason()),
        RootStoreError::Mutation(WorkEngineError::ValidationController(e)) => Some(e.code()),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
impl NativeHttpService {
    /// Read task review originals under the existing Logs view and Root binding.
    /// GET/HEAD accept one Work, task, revision and optional declared file key.
    /// The engine owns admission and byte integrity; this adapter records nothing.
    pub(super) fn task_review_files(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        let invalid = || E::Refused("task-review-files-query-invalid");
        let mut fields = std::collections::BTreeMap::new();
        for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
            if !matches!(key.as_ref(), "work" | "task" | "revision" | "file" | "include_report")
                || value.is_empty() || fields.insert(key.into_owned(), value.into_owned()).is_some()
            {
                return Err(invalid());
            }
        }
        let work = fields.get("work").ok_or_else(invalid)?;
        let task = fields.get("task").ok_or_else(invalid)?;
        let revision = fields.get("revision").and_then(|s| s.parse::<i64>().ok())
            .filter(|n| *n > 0).ok_or_else(invalid)?;
        let include_report = match fields.get("include_report").map(String::as_str) {
            None => false,
            Some("1") if !fields.contains_key("file") => true,
            _ => return Err(invalid()),
        };
        if !request.body().is_empty() || ![work, task].iter().all(|id| swarm_identity_contract::name(id.as_bytes())) {
            return Err(invalid());
        }
        let raw = serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"task-review-read",
            "expected_revision":revision,"task_id":task})).map_err(E::ResponseJson)?;
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let result = (|| {
            self.check_binding(&store)?;
            let value = store.task_review_files_cached(work, &raw, fields.get("file").map(String::as_str), include_report, &self.task_review_reader)
                .map_err(|cause| root("task-review-files-read", cause))?;
            serde_json::to_vec(&value).map(RouteBody::plain).map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}
