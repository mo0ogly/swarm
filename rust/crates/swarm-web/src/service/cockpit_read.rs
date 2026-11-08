//! Minimal protected observations for the existing budget/agent product sections.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;
impl NativeHttpService {
    /// Read one explicit Work behind shared admission and owned Root binding.
    /// GET/HEAD only; duplicate/unknown/unsafe query data refuse before IO. Checks
    /// Work existence, returns saved budget observations and a deliberately narrow
    /// agent projection (no prompts, command, environment, workspace or contexts).
    /// Adds one additive narrow autonomy_observation field for cockpit
    /// consumption: the recorded initial autonomy choice, its source
    /// ("mission", "installation" or "not_configured"), the settings revision
    /// and the newest-consent flag; launch_authorized stays false and
    /// schema_version stays 1 with budget and agents unchanged.
    /// Missing budget stays null (unlimited, figures unavailable), not zero costs.
    /// Recorded states/links are neither authenticated identity nor live probes.
    /// No reservations, visits, agents or providers start. Read/encode/close errors
    /// retain typed causes; no failure is converted into an empty successful list.
    /// Reads share the opened store but are not a cross-producer atomic snapshot.
    pub(super) fn cockpit_read(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        let mut work = None;
        for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
            if key != "work" || work.is_some() { return Err(E::Refused("cockpit-query-invalid")); }
            work = Some(value.into_owned());
        }
        let work = work.filter(|id: &String| swarm_identity_contract::name(id.as_bytes()))
            .ok_or(E::Refused("cockpit-query-invalid"))?;
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let result = (|| {
            self.check_binding(&store)?;
            store.read_work(&work).map_err(|cause| root("read", cause))?;
            let budget = store.work_budget_state(&work).map_err(|cause| root("budget-read", cause))?;
            let agents = store.list_agents(&work).map_err(|cause| root("agents-read", cause))?;
            let autonomy = store.read_cockpit_autonomy(&work).map_err(|cause| root("autonomy-read", swarm_engine::RootStoreError::Mutation(swarm_engine::WorkEngineError::Mission(Box::new(cause)))))?;
            let agents: Vec<_> = agents.iter().map(|a| serde_json::json!({
                "id":a.id,"work_id":a.work_id,"task_id":a.task_id,"attempt_id":a.attempt_id,
                "status":a.status,"provider":a.provider,"role":a.role,"parent":a.parent,
                "previous":a.previous,"started":a.started,"ended":a.ended
            })).collect();
            serde_json::to_vec(&serde_json::json!({"schema_version":1,"work_id":work,"budget":budget,"agents":agents,"autonomy_observation":autonomy}))
                .map(RouteBody::plain).map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}
