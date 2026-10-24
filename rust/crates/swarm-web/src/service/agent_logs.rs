//! Historical Agent log read capability; no view publication or visit mutation.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;

/// Strict explicit Work/Agent/signed cursor triple. Reject unknown, duplicate,
/// empty, or non-canonical cursor values before any store IO. The cursor must
/// be a canonical signed i64 decimal: optional leading `-`, no `+`, no leading
/// zeros except the single digit `0`, and `-0` is refused. Identifiers are
/// data, never another Root or path choice.
fn query(raw: Option<&str>) -> Result<(String, String, i64), E> {
    let (mut work, mut agent, mut after) = (None, None, None);
    for (key, value) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        let slot = match key.as_ref() {
            "work" => &mut work,
            "agent" => &mut agent,
            "after" => &mut after,
            _ => return Err(E::Refused("agent-logs-query-invalid")),
        };
        if slot.replace(value.into_owned()).is_some() {
            return Err(E::Refused("agent-logs-query-invalid"));
        }
    }
    let raw = after.ok_or(E::Refused("agent-logs-query-invalid"))?;
    let parsed = raw.parse::<i64>().map_err(|_| E::Refused("agent-logs-query-invalid"))?;
    if parsed.to_string() != raw { return Err(E::Refused("agent-logs-query-invalid")); }
    match (work, agent) {
        (Some(work), Some(agent))
            if swarm_identity_contract::name(work.as_bytes())
                && swarm_identity_contract::name(agent.as_bytes()) =>
        {
            Ok((work, agent, parsed))
        }
        _ => Err(E::Refused("agent-logs-query-invalid")),
    }
}

/// Native observation DTO. Recorded log text is not agent authentication,
/// liveness or permission; consumers must escape it for their rendering context.
#[derive(serde::Serialize)]
struct AgentLogs {
    schema_version: i64,
    work_id: String,
    agent_id: String,
    after: i64,
    rows: Vec<swarm_engine::HistoricalAgentLog>,
}

impl NativeHttpService {
    /// Read historical Agent log pages with the CLI cursor contract using
    /// historical_agent_logs under shared HTTP admission/capacity and the
    /// opened-readonly owned Root binding.
    ///
    /// Zero selects latest200, negative first200, positive rows after that seq.
    /// This existing transport pagination is not an execution/retention policy.
    /// Preconditions: GET/HEAD only, empty request body only, query carrying
    /// exactly `work`, `agent` and `after`, each once, with canonical identity
    /// names and a canonical signed i64 cursor. The Agent must exist and its
    /// recorded work_id must equal the requested work.
    ///
    /// Errors: unknown/duplicate/empty/unsafe query parameters, a missing or
    /// non-canonical cursor, or a non-empty body are refused before store IO
    /// (`agent-logs-query-invalid`); a stored Agent belonging to a different
    /// Work is refused (`agent-logs-work-mismatch`); typed read/encode/close
    /// failures propagate, never an empty successful placeholder.
    ///
    /// No mutation: no Work, Agent, log, visit/OODA/budget or view state is
    /// changed; the Root is opened readonly and rows are observations only.
    pub(super) fn agent_logs(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !request.body().is_empty() {
            return Err(E::Refused("agent-logs-query-invalid"));
        }
        let (work, agent, after) = query(request.uri().query())?;
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            let work_id = store.read_work(&work).map_err(|e| root("agent-logs-work-read", e))?.id;
            if work_id != work {
                return Err(E::Refused("agent-logs-work-mismatch"));
            }
            let stored = store.read_agent(&agent).map_err(|e| root("agent-logs-agent-read", e))?;
            if stored.work_id != work {
                return Err(E::Refused("agent-logs-work-mismatch"));
            }
            let rows = store
                .historical_agent_logs(&agent, after)
                .map_err(|e| root("agent-logs-read", e))?;
            serde_json::to_vec(&AgentLogs {
                schema_version: 1,
                work_id: work,
                agent_id: agent,
                after,
                rows,
            })
            .map(RouteBody::plain)
            .map_err(E::Json)
        })();
        close(result, store.close())
    }
}
