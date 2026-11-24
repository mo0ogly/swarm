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
