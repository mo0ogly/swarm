//! Read-only live observation of agents across every recorded Work, plus the
//! embedded live dashboard and per-agent harness window. No route here starts,
//! stops, leases or sends input to an agent; pages poll recorded state only.
use super::{close, response, root, HttpReply, HttpServiceError, NativeHttpService, RouteBody};
use http::{
    header::{CONTENT_SECURITY_POLICY, CONTENT_TYPE},
    HeaderValue, Method, Request, StatusCode,
};
use serde_json::{json, Map, Value};
use swarm_engine::{Agent, RootStore, RootStoreError};

/// Statuses the Go engine treats as one active attempt (unique-index set).
pub(super) const ACTIVE: [&str; 4] = ["queued", "starting", "running", "stopping"];
/// Agent fields exposed to the live views; everything else (context manifests,
/// workspace paths, preflight bodies) stays out of the polling payload.
const AGENT_FIELDS: [&str; 19] = [
    "limits",
    "id",
    "work_id",
    "task_id",
    "attempt_id",
    "provider",
    "role",
    "parent",
    "status",
    "desired",
    "activity",
    "mode",
    "started",
    "heartbeat",
    "ended",
    "progress",
    "usage",
    "reported_model",
    "stop_kind",
];

/// Fields of the engine-owned `native_provider_facts` projection shown to the
/// operator: observation quality per channel and safe static error codes. The
/// last-valid history is omitted; canonical values already live in `usage`.
const PROVIDER_FACT_FIELDS: [&str; 8] = [
    "schema_version",
    "source",
    "usage_state",
    "model_state",
    "invalid_usage_events",
    "invalid_model_events",
    "first_usage_error",
    "first_model_error",
];
/// Placeholder replaced once per response by a fresh CSP style nonce; xterm
/// creates `<style>` elements that must carry it.
const NONCE_META: &str = "<meta name=\"swarm-style-nonce\" content=\"\">";

/// Exact embedded resource paths; unknown paths never become IO.
pub(super) fn owns(path: &str) -> bool {
    asset(path).is_some()
}
/// GET/HEAD of an embedded live page; used for browser lane classification only.
pub(super) fn resource<B>(r: &Request<B>) -> bool {
    matches!(*r.method(), Method::GET | Method::HEAD) && owns(r.uri().path())
}
/// Versioned source bytes compiled into the binary; not a public directory.
fn asset(path: &str) -> Option<(&'static str, &'static [u8])> {
    const JS: &str = "application/javascript; charset=utf-8";
    match path {
        "/live.html" => Some((
            "text/html; charset=utf-8",
            include_bytes!("../../../../../web/live.html"),
        )),
        "/live-session.html" => Some((
            "text/html; charset=utf-8",
            include_bytes!("../../../../../web/live-session.html"),
        )),
        "/live/live.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../web/live.css"),
        )),
        "/live/live.js" => Some((JS, include_bytes!("../../../../../web/live.js"))),
        "/live/recorded-cursor.js"=>Some((JS,include_bytes!("../../../../../web/recorded-cursor.js"))),
        "/live/log-query.js"=>Some((JS,include_bytes!("../../../../../web/native-log-query.js"))),
        "/live/live-session.js" => {
            Some((JS, include_bytes!("../../../../../web/live-session.js")))
        }
        "/live/live-common.js" => Some((JS, include_bytes!("../../../../../web/live-common.js"))),
        "/live/live-metrics.js" => Some((JS, include_bytes!("../../../../../web/live-metrics.js"))),
        "/live/i18n.js" => Some((JS, include_bytes!("../../../../../web/i18n.js"))),
        "/live/english.js" => Some((JS, include_bytes!("../../../../../web/i18n-en.js"))),
        "/live/swarm-design.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../web/swarm-design.css"),
        )),
        "/live/live-terminal.js" => Some((
            JS,
            include_bytes!("../../../../../web/lib/terminal/live-terminal.js"),
        )),
        "/live/xterm.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../web/lib/terminal/xterm.css"),
        )),
        "/live/themes.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../web/wattson_themes.css"),
        )),
        _ => None,
    }
}
/// Serve one embedded live resource after the caller authorized the session.
/// Only GET/HEAD; shared CSP/no-store headers apply, HEAD omits the body.
pub(super) fn reply(r: &Request<Vec<u8>>) -> Result<HttpReply, HttpServiceError> {
    if !matches!(*r.method(), Method::GET | Method::HEAD) {
        return Err(HttpServiceError::Method("GET, HEAD"));
    }
    let (kind, bytes) =
        asset(r.uri().path()).ok_or(HttpServiceError::Refused("web-route-unported"))?;
    let (bytes, nonce) = if r.uri().path() == "/live-session.html" {
        harness_page(bytes)?
    } else {
        (bytes.to_vec(), None)
    };
    let mut response = response(StatusCode::OK, bytes, r.method() == Method::HEAD, None);
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(kind));
    if let Some(nonce) = nonce {
        let policy = format!("default-src 'self'; script-src 'self'; style-src 'self' 'nonce-{nonce}'; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'");
        response.headers_mut().insert(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_str(&policy).map_err(HttpServiceError::HeaderEncoding)?,
        );
    }
    Ok(HttpReply {
        response,
        failure: None,
        recorded_work: None,
        configuration_may_be_recorded: false,
        recorded_configuration: None,
    })
}

/// Insert a fresh per-response style nonce into the harness page. A template
/// without exactly one placeholder is a server error, never a page without CSP.
fn harness_page(template: &[u8]) -> Result<(Vec<u8>, Option<String>), HttpServiceError> {
    let text = std::str::from_utf8(template)
        .map_err(|_| HttpServiceError::Refused("web-native-template-invalid"))?;
    if text.matches(NONCE_META).count() != 1 {
        return Err(HttpServiceError::Refused("web-native-template-invalid"));
    }
    let nonce = swarm_engine::generate_identifier("")
        .map_err(|_| HttpServiceError::Refused("live-nonce-unavailable"))?;
    let page = text.replacen(
        NONCE_META,
        &format!("<meta name=\"swarm-style-nonce\" content=\"{nonce}\">"),
        1,
    );
    Ok((page.into_bytes(), Some(nonce)))
}

/// Keep only [`AGENT_FIELDS`] plus the diagnostic summary of one agent.
/// Encoding failure is returned, never replaced by a partial object.
fn agent_view(agent: &Agent) -> Result<Value, HttpServiceError> {
    let full = serde_json::to_value(agent).map_err(HttpServiceError::ResponseJson)?;
    let mut view = Map::new();
    for key in AGENT_FIELDS {
        if let Some(value) = full.get(key) {
            view.insert(key.to_owned(), value.clone());
        }
    }
    view.insert(
        "diagnostic".to_owned(),
        json!({
            "summary": agent.diagnostic.summary,
            "observed_errors": agent.diagnostic.observed_errors,
            "limit_reached": agent.diagnostic.limit_reached,
        }),
    );
    if let Some(raw) = &agent.native_provider_facts {
        let facts: Value = serde_json::from_str(raw.0.get()).map_err(HttpServiceError::Json)?;
        let mut picked = Map::new();
        for key in PROVIDER_FACT_FIELDS {
            if let Some(value) = facts.get(key) {
                picked.insert(key.to_owned(), value.clone());
            }
        }
        view.insert("provider_facts".to_owned(), Value::Object(picked));
    }
    if let Some(context) = &agent.context {
        let count = |v: &Option<Vec<String>>| v.as_ref().map_or(0, Vec::len);
        view.insert(
            "context".to_owned(),
            json!({
                "bytes": context.bytes,
                "included": count(&context.included),
                "excluded": count(&context.excluded),
                "paths": count(&context.paths),
                "memory": context.memory.as_ref().map_or(0, Vec::len),
                "references": context.references.as_ref().map_or(0, Vec::len),
                "summary": context.summary.is_some(),
            }),
        );
    }
    view.insert(
        "active".to_owned(),
        Value::Bool(ACTIVE.contains(&agent.status.as_str())),
    );
    Ok(Value::Object(view))
}

/// Telemetry change marker for the live fingerprint, None without a store (the
/// fingerprint then equals the mission-only digest). A store failure yields a
/// stable `unavailable:<code>` marker so the change is visible, not hidden.
fn telemetry_marker(root: &std::path::Path) -> Option<String> {
    use swarm_engine::{TelemetryStore, TelemetryStoreError};
    match TelemetryStore::open(root, false) {
        Ok(store) => {
            let marker = store.change_marker();
            let closed = store.close();
            Some(match (marker, closed) {
                (Ok(m), Ok(())) => m,
                (Err(e), _) | (Ok(_), Err(e)) => format!("unavailable:{}", e.code()),
            })
        }
        Err(TelemetryStoreError::Refused("telemetry-store-missing")) => None,
        Err(e) => Some(format!("unavailable:{}", e.code())),
    }
}

/// Planned task as recorded in the Work by the initial planning phase, for the
/// live plan graph and the harness: identity, dependencies, planned role,
/// limits and model, and the outcome so far. Read-only projection; instructions,
/// memory selections and context bodies stay out. `plan_max_attempts` and
/// `plan_tool_limit` are 0 when the plan set no value (not an unlimited claim).
/// `delivery` (LangWatch L9) is the task's current candidate as recorded by
/// its independent review — commit, previous candidate, review state and
/// number of reviews — or Null when no review was recorded; it says nothing
/// about acceptance beyond `status` and is not a published release.
fn plan_view(task: &swarm_engine::Task) -> Value {
    json!({
        "id": task.id,
        "title": task.title,
        "owner": task.owner,
        "deliverable": task.deliverable,
        "criteria": task.criteria.as_ref().map_or(0, Vec::len),
        "depends": task.depends.clone().unwrap_or_default(),
        "status": task.status,
        "blocker": task.blocker,
        "next": task.next,
        "attempts": task.attempts.as_ref().map_or(0, Vec::len),
        "plan_role": task.plan_role,
        "plan_max_attempts": task.plan_max_attempts,
        "plan_tool_limit": task.plan_tool_limit,
        "launch_held": task.launch_held,
        "model": task.model_selection.as_ref().map(|m| json!({
            "provider": m.provider,
            "model": m.route.model,
            "level": m.route.level,
        })),
        "delivery": task.independent_review.as_ref().map(|r| json!({
            "commit": r.candidate_commit,
            "previous": r.previous_candidate,
            "review_state": r.state,
            "review_finished": r.finished,
            "reviews": 1 + task.previous_reviews.as_ref().map_or(0, Vec::len),
        })),
    })
}

/// Add `estimated_cost` to a received telemetry view: calls that declare no
/// `cost_usd` are priced with the rate of the same provider and model in the
/// saved catalogue (the most recently `updated` version when several exist).
/// Shape: `{amount, currency, priced_calls, unpriced_calls, rate_versions,
/// missing_rates}`; `amount` is Null when no call could be priced. Calls
/// whose rate lacks a used category stay unpriced (listed in
/// `missing_rates`), never priced at zero. Mixed currencies are refused by
/// leaving `amount` Null with `currency` "mixed". Declared costs are untouched.
fn estimate_call_costs(telemetry: &mut Value, provider: &str, rates: &swarm_engine::RateCatalogue) {
    let Some(calls) = telemetry.get("calls").and_then(Value::as_array) else { return };
    let count = |f: &Value, k: &str| f.get(k).and_then(Value::as_str).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    let (mut amount, mut priced, mut unpriced) = (0.0, 0u64, 0u64);
    let (mut currencies, mut versions, mut missing) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
    for call in calls {
        let f = &call["fields"];
        if f.get("cost_usd").is_some() {
            continue;
        }
        let model = f.get("model").and_then(Value::as_str).unwrap_or_default();
        let rate = rates
            .rates
            .iter()
            .flatten()
            .filter(|r| r.provider == provider && r.model == model)
            .max_by(|a, b| a.updated.cmp(&b.updated));
        let quoted = rate.and_then(|rate| {
            let request = swarm_engine::RateQuoteRequest {
                rate_version: rate.version.clone(),
                non_cached_input_tokens: Some(count(f, "input_tokens")),
                output_tokens: Some(count(f, "output_tokens")),
                cache_read_tokens: count(f, "cache_read_tokens"),
                cache_write_tokens: count(f, "cache_creation_tokens"),
            };
            swarm_engine::quote_model_rate(rate, &request).ok()
        });
        match quoted {
            Some(q) if q.estimated_amount.is_some() => {
                amount += q.estimated_amount.map_or(0.0, |a| a.0);
                priced += 1;
                currencies.insert(q.rate.currency.clone());
                versions.insert(q.rate.version.clone());
            }
            Some(q) => {
                unpriced += 1;
                missing.extend(q.missing_rates);
            }
            None => unpriced += 1,
        }
    }
    if priced + unpriced == 0 {
        return;
    }
    let currency = match currencies.len() {
        0 => Value::Null,
        1 => json!(currencies.iter().next()),
        _ => json!("mixed"),
    };
    let amount = (priced > 0 && currencies.len() == 1).then_some(amount);
    telemetry["estimated_cost"] = json!({
        "amount": amount, "currency": currency, "priced_calls": priced, "unpriced_calls": unpriced,
        "rate_versions": versions, "missing_rates": missing,
    });
}

/// Typed transcript (LangWatch L2) of a finished attempt, derived from the
/// retained provider stream `native_execution_result.capture`; Null while the
/// attempt runs or when no capture was retained. `capture_truncated` reports
/// the retention cap (`agent_capture_max_bytes`); content is never returned
/// (see [`swarm_engine::typed_transcript`]). An undecodable result document is
/// shown as `{"state":"unavailable"}` rather than failing the whole view.
fn transcript_view(agent: &Agent) -> Value {
    let Some(raw) = agent.native_execution_result.as_ref() else {
        return Value::Null;
    };
    let Ok(result) = serde_json::from_str::<Value>(raw.0.get()) else {
        return json!({"state": "unavailable", "code": "live-transcript-result-invalid"});
    };
    let Some(capture) = result.get("capture").and_then(Value::as_str) else {
        return Value::Null;
    };
    let mut view = serde_json::to_value(swarm_engine::typed_transcript(capture))
        .unwrap_or_else(|_| json!({"state": "unavailable", "code": "live-transcript-encoding"}));
    view["capture_truncated"] = result.get("capture_truncated").cloned().unwrap_or(Value::Bool(false));
    view["state"] = Value::String("available".into());
    view
}

/// Provider telemetry read model for one agent (LIVE-UX-03). No store means
/// no token was ever issued (`no-token`); a store failure is shown as state
/// `unavailable` with its stable code instead of failing the whole agent view.
/// Values are provider declarations, labelled as such by the store.
fn telemetry_view(root: &std::path::Path, agent: &str) -> Value {
    use swarm_engine::{TelemetryStore, TelemetryStoreError};
    let store = match TelemetryStore::open(root, false) {
        Ok(store) => store,
        Err(TelemetryStoreError::Refused("telemetry-store-missing")) => {
            return json!({"state": "no-token", "provenance": "provider-declared-unauthenticated"})
        }
        Err(error) => return json!({"state": "unavailable", "code": error.code()}),
    };
    let read = store.read_agent(agent);
    let closed = store.close();
    match (read, closed) {
        (Ok(view), Ok(())) => serde_json::to_value(view)
            .unwrap_or_else(|_| json!({"state": "unavailable", "code": "telemetry-encoding"})),
        (Err(error), _) | (Ok(_), Err(error)) => json!({"state": "unavailable", "code": error.code()}),
    }
}

/// Latest attempt of `work`/`task` by recorded `started` time (an
/// unparsable time sorts first, so a parsable newer attempt wins; ties by
/// id). Task ids repeat across Works, hence the Work filter. Differs from
/// the alert rules' "current attempt" ([`swarm_engine::alert_conditions`]),
/// which keep an attempt with an unparsable time current rather than hide
/// it: a scenario then judges the newest attempt with a readable time.
fn latest_attempt<'a>(agents: &'a [Agent], work: &str, task: &str) -> Option<&'a Agent> {
    let started = |a: &Agent| swarm_wire_json::go_rfc3339::parse(&a.started).ok();
    agents
        .iter()
        .filter(|a| a.work_id == work && a.task_id == task)
        .max_by(|x, y| started(x).cmp(&started(y)).then_with(|| x.id.cmp(&y.id)))
}

/// L6 scenario summary per task of one Work for the plan graph:
/// `{task: {"passed", "failed", "undecided"}}`; empty without scenarios or
/// store. A store failure returns its stable code (shown, not hidden).
fn scenario_summaries(root: &std::path::Path, work: &str, agents: &[Agent]) -> Result<Map<String, Value>, &'static str> {
    let store = match swarm_engine::TelemetryStore::open(root, false) {
        Ok(store) => store,
        Err(swarm_engine::TelemetryStoreError::Refused("telemetry-store-missing")) => return Ok(Map::new()),
        Err(e) => return Err(e.code()),
    };
    let built = (|| -> Result<Map<String, Value>, swarm_engine::TelemetryStoreError> {
        let mut out = Map::new();
        for (task, (spec, _, _)) in store.read_scenarios(work)? {
            let latest = latest_attempt(agents, work, &task);
            let view = match latest { Some(a) => Some(store.read_agent(&a.id)?), None => None };
            let results = swarm_engine::evaluate_scenario(&spec, latest, view.as_ref().filter(|v| v.state == "received"));
            let count = |want: Option<bool>| results.iter().filter(|r| r.passed == want).count();
            out.insert(task, json!({"passed": count(Some(true)), "failed": count(Some(false)), "undecided": count(None)}));
        }
        Ok(out)
    })();
    let closed = store.close();
    match (built, closed) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), _) | (Ok(_), Err(e)) => Err(e.code()),
    }
}

/// Stored L4 evaluations of one attempt for the harness: `[]` when none or
/// no store; a store failure is `{"state":"unavailable","code"}`.
fn evaluations_view(root: &std::path::Path, agent: &str) -> Value {
    match swarm_engine::TelemetryStore::open(root, false) {
        Ok(store) => {
            let read = store.read_evaluations(agent);
            let closed = store.close();
            match (read, closed) {
                (Ok(rows), Ok(())) => json!(rows),
                (Err(e), _) | (Ok(_), Err(e)) => json!({"state": "unavailable", "code": e.code()}),
            }
        }
        Err(swarm_engine::TelemetryStoreError::Refused("telemetry-store-missing")) => json!([]),
        Err(e) => json!({"state": "unavailable", "code": e.code()}),
    }
}

/// Verdict of every annotated attempt (all Works), for the overview queue.
/// No store: empty. A store failure returns its stable code, which the
/// overview exposes as `annotations_state` instead of an empty queue.
fn overview_annotations(root: &std::path::Path) -> Result<std::collections::BTreeMap<String, String>, &'static str> {
    match swarm_engine::TelemetryStore::open(root, false) {
        Ok(store) => {
            let read = store.annotation_verdicts();
            let closed = store.close();
            match (read, closed) {
                (Ok(v), Ok(())) => Ok(v),
                (Err(e), _) | (Ok(_), Err(e)) => Err(e.code()),
            }
        }
        Err(swarm_engine::TelemetryStoreError::Refused("telemetry-store-missing")) => Ok(Default::default()),
        Err(e) => Err(e.code()),
    }
}

/// Operator annotation of one attempt for the harness (LangWatch L10): Null
/// when none or no store; a store failure is a `{"state":"unavailable"}`
/// object with its code, not a refusal of the whole agent view.
fn annotation_view(root: &std::path::Path, work: &str, agent: &str) -> Value {
    match swarm_engine::TelemetryStore::open(root, false) {
        Ok(store) => {
            let read = store.read_annotations(work);
            let closed = store.close();
            match (read, closed) {
                (Ok(mut all), Ok(())) => all.remove(agent).map_or(Value::Null, |a| json!(a)),
                (Err(e), _) | (Ok(_), Err(e)) => json!({"state": "unavailable", "code": e.code()}),
            }
        }
        Err(swarm_engine::TelemetryStoreError::Refused("telemetry-store-missing")) => Value::Null,
        Err(e) => json!({"state": "unavailable", "code": e.code()}),
    }
}

/// Host tuple and one `/proc` scan for a batch of resource observations. On
/// failure the caller exposes the stable code instead of any process number.
#[cfg(target_os = "linux")]
fn resource_probe() -> Result<(String, swarm_engine::ProcessTable), &'static str> {
    let host = swarm_engine::SupervisorIdentity::current()
        .map_err(|_| "live-resources-host-unavailable")?
        .host()
        .to_owned();
    let table =
        swarm_engine::ProcessTable::scan().map_err(|_| "live-resources-proc-unavailable")?;
    Ok((host, table))
}

/// Attach verified process resources to an agent view (Linux only).
#[cfg(target_os = "linux")]
fn attach_resources(
    view: &mut Value,
    agent: &Agent,
    probe: &Result<(String, swarm_engine::ProcessTable), &'static str>,
) -> Result<(), HttpServiceError> {
    view["resources"] = match probe {
        Ok((host, table)) => serde_json::to_value(swarm_engine::observe_agent_resources(agent, host, table))
            .map_err(HttpServiceError::ResponseJson)?,
        Err(code) => json!({"observation": "unavailable", "error": code}),
    };
    Ok(())
}

fn count(counts: &mut Map<String, Value>, status: &str) {
    let next = counts.get(status).and_then(Value::as_i64).unwrap_or(0) + 1;
    counts.insert(status.to_owned(), next.into());
}

/// Parsed `/api/v1/live/agent` query; duplicate or unknown keys are refused so
/// an ambiguous URL never silently selects one value.
struct AgentQuery {
    agent: String,
    work: String,
    logs_after: i64,
    terminal_after: i64,
}
fn agent_query(query: Option<&str>) -> Result<AgentQuery, HttpServiceError> {
    let invalid = || HttpServiceError::Refused("live-query-invalid");
    let (mut agent, mut work, mut logs, mut terminal) = (None, None, None, None);
    for (key, value) in form_urlencoded::parse(query.unwrap_or("").as_bytes()) {
        let slot = match key.as_ref() {
            "agent" => &mut agent,
            "work" => &mut work,
            "logs_after" => &mut logs,
            "terminal_after" => &mut terminal,
            _ => return Err(invalid()),
        };
        if slot.replace(value.into_owned()).is_some() {
            return Err(invalid());
        }
    }
    let cursor = |raw: Option<String>| -> Result<i64, HttpServiceError> {
        match raw {
            None => Ok(0),
            Some(raw) => raw
                .parse::<i64>()
                .ok()
                .filter(|v| *v >= 0)
                .ok_or(HttpServiceError::Refused("live-cursor-invalid")),
        }
    };
    let agent = agent.filter(|v| !v.is_empty()).ok_or_else(invalid)?;
    let work = work.filter(|v| !v.is_empty()).ok_or_else(invalid)?;
    Ok(AgentQuery {
        agent,
        work,
        logs_after: cursor(logs)?,
        terminal_after: cursor(terminal)?,
    })
}

impl NativeHttpService {
    /// `GET /api/v1/live/overview`: every recorded Work with its agents (light
    /// view, newest first), per-status counts, its planned tasks (`plan`, see
    /// [`plan_view`]), its review jobs and planning runs (`runs`, see
    /// [`swarm_engine::TelemetryRun`]) and budget (`budget`, see
    /// [`swarm_engine::WorkBudgetState`]), plus global totals and the
    /// failure-cause groups of all attempts (`failure_groups`, see
    /// [`swarm_engine::failure_groups`]). One
    /// read-only Root open; any Work/agent read failure refuses the whole page
    /// instead of returning a partial overview that would look complete.
    pub(super) fn live_overview(
        &self,
        request: &Request<Vec<u8>>,
    ) -> Result<RouteBody, HttpServiceError> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(HttpServiceError::Method("GET, HEAD"));
        }
        if request.uri().query().is_some_and(|q| !q.is_empty()) {
            return Err(HttpServiceError::Refused("live-query-invalid"));
        }
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let result = (|| {
            self.check_binding(&store)?;
            let mut totals = Map::new();
            let mut works = Vec::new();
            // L10: operator verdicts, one console-store read for the page.
            let annotated = overview_annotations(&self.root);
            // L11: every recorded attempt, for the failure-cause groups.
            let mut all_agents = Vec::new();
            #[cfg(target_os = "linux")]
            let mut probe = None;
            for work in store.list_works().map_err(|cause| root("read", cause))? {
                let titles: Map<String, Value> = work
                    .tasks
                    .iter()
                    .flatten()
                    .map(|t| (t.id.clone(), Value::String(t.title.clone())))
                    .collect();
                let mut counts = Map::new();
                let mut agents = Vec::new();
                for agent in store
                    .list_agents(&work.id)
                    .map_err(|cause| root("read", cause))?
                {
                    count(&mut counts, &agent.status);
                    count(&mut totals, &agent.status);
                    #[allow(unused_mut)]
                    let mut view = agent_view(&agent)?;
                    if let Some(verdict) = annotated.as_ref().ok().and_then(|m| m.get(&agent.id)) {
                        view["annotation"] = Value::String(verdict.clone());
                    }
                    #[cfg(target_os = "linux")]
                    if ACTIVE.contains(&agent.status.as_str()) {
                        let probe = probe.get_or_insert_with(resource_probe);
                        attach_resources(&mut view, &agent, probe)?;
                    }
                    agents.push(view);
                    all_agents.push(agent);
                }
                works.push(json!({
                    "id": work.id,
                    "title": work.title,
                    "revision": work.revision,
                    "tasks": titles,
                    "plan": work.tasks.iter().flatten().map(plan_view).collect::<Vec<_>>(),
                    "counts": counts,
                    "agents": agents,
                    // L6: scenario results per task (or the console-store failure code).
                    "scenarios": match scenario_summaries(&self.root, &work.id, &all_agents) {
                        Ok(map) => Value::Object(map),
                        Err(code) => json!({"state": "unavailable", "code": code}),
                    },
                    // L7: saved budget, committed estimates and declared cost (Null = no policy).
                    // A malformed budget or declared cost of this Work is shown as a
                    // state with its code; it does not refuse the whole overview.
                    "budget": match store.work_budget_state(&work.id) {
                        Ok(state) => json!(state),
                        Err(cause) => json!({"state": "unavailable", "code": cause.code()}),
                    },
                    // Review jobs and planning runs that may carry telemetry.
                    "runs": store.telemetry_runs(&work.id, None).map_err(|cause| root("read", cause))?,
                }));
            }
            serde_json::to_vec(&json!({
                "schema_version": 1,
                "active_statuses": ACTIVE,
                "totals": totals,
                "works": works,
                "failure_groups": swarm_engine::failure_groups(&all_agents),
                // Console-store read failure for verdicts: shown, not hidden as "none".
                "annotations_state": annotated.as_ref().err().map_or(Value::Null, |code| json!({"state": "unavailable", "code": code})),
            }))
            .map_err(HttpServiceError::ResponseJson)
        })();
        close(result, store.close()).map(RouteBody::plain)
    }

    /// `GET /api/v1/live/fingerprint`: digest of all displayed live state (see
    /// [`swarm_engine::read_live_fingerprint`]), folded with the review/planning
    /// run phases and the telemetry store change marker when present. Used by the SSE probe and by
    /// clients to skip unchanged refreshes. Query refused; no write.
    pub(super) fn live_fingerprint(
        &self,
        request: &Request<Vec<u8>>,
    ) -> Result<RouteBody, HttpServiceError> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(HttpServiceError::Method("GET, HEAD"));
        }
        if request.uri().query().is_some_and(|q| !q.is_empty()) {
            return Err(HttpServiceError::Refused("live-query-invalid"));
        }
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let result = self.check_binding(&store).and_then(|()| {
            let mut fingerprint = store
                .live_fingerprint()
                .map_err(|cause| root("read", cause))?;
            // Review/planning run phases are not in the mission digest: fold
            // them in so a run that starts or ends wakes the console.
            let runs = store.telemetry_runs_marker().map_err(|cause| root("read", cause))?;
            if !runs.is_empty() {
                use sha2::{Digest, Sha256};
                let digest = Sha256::digest(format!("{fingerprint}\nruns:{runs}").as_bytes());
                fingerprint = digest.iter().map(|b| format!("{b:02x}")).collect();
            }
            if let Some(marker) = telemetry_marker(&self.root) {
                use sha2::{Digest, Sha256};
                let digest = Sha256::digest(format!("{fingerprint}\ntelemetry:{marker}").as_bytes());
                fingerprint = digest.iter().map(|b| format!("{b:02x}")).collect();
            }
            serde_json::to_vec(&json!({"schema_version": 1, "fingerprint": fingerprint}))
                .map_err(HttpServiceError::ResponseJson)
        });
        close(result, store.close()).map(RouteBody::plain)
    }

    /// `GET /api/v1/live/agent?agent=&work=&logs_after=&terminal_after=`: one
    /// agent's light view plus the next bounded pages of recorded logs and PTY
    /// chunks, its planned task (`planned_task`, Null if absent from the Work),
    /// the saved cache-health thresholds (`cache_policy`, Null before v31), the
    /// typed transcript of a finished attempt (`transcript`, Null otherwise) and
    /// provider telemetry (`telemetry`, a store failure shown as a state, not a
    /// refusal). Refusals: missing/duplicate/unknown keys (`live-query-invalid`),
    /// negative or non-numeric cursors (`live-cursor-invalid`), unknown agent
    /// (`live-agent-unknown`, 404) and an agent outside `work`
    /// (`live-agent-work-mismatch`). Never opens the terminal control channel.
    pub(super) fn live_agent(
        &self,
        request: &Request<Vec<u8>>,
    ) -> Result<RouteBody, HttpServiceError> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(HttpServiceError::Method("GET, HEAD"));
        }
        let query = agent_query(request.uri().query())?;
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let result = (|| {
            self.check_binding(&store)?;
            let agent = match store.read_agent(&query.agent) {
                Ok(agent) => agent,
                Err(RootStoreError::Agent(swarm_engine::AgentStoreError::Missing)) => {
                    return Err(HttpServiceError::Refused("live-agent-unknown"))
                }
                Err(cause) => return Err(root("read", cause)),
            };
            if agent.work_id != query.work {
                return Err(HttpServiceError::Refused("live-agent-work-mismatch"));
            }
            let logs = store
                .agent_logs_after(&agent.id, query.logs_after)
                .map_err(|cause| root("read", cause))?;
            let terminal = store
                .terminal_events_after(&agent.id, query.terminal_after)
                .map_err(|cause| root("read", cause))?;
            let mut view = agent_view(&agent)?;
            view["reply"] = Value::String(agent.reply.clone());
            #[cfg(target_os = "linux")]
            attach_resources(&mut view, &agent, &resource_probe())?;
            if let Some(memory) = agent.context.as_ref().and_then(|c| c.memory.as_ref()) {
                // Identifiers and versions only; memory content stays in its store.
                view["context_memory"] = Value::Array(
                    memory
                        .iter()
                        .map(|m| json!({"id": m.id, "version": m.version, "work": m.work}))
                        .collect(),
                );
            }
            view["telemetry"] = telemetry_view(&self.root, &agent.id);
            // L4 stored evaluations of this attempt ([] when none or no store).
            view["evaluations"] = evaluations_view(&self.root, &agent.id);
            // L10 operator annotation of this attempt (Null when none or no store).
            view["annotation"] = annotation_view(&self.root, &agent.work_id, &agent.id);
            // Estimated cost of calls without a declared cost (e.g. Codex), from
            // the saved rate catalogue; never mixed with declared costs.
            let rates = store.rate_catalogue().map_err(|cause| root("pricing", cause))?;
            estimate_call_costs(&mut view["telemetry"], &agent.provider, &rates);
            // Saved Work budget (Null = no policy, the unlimited contract), so the
            // harness can show how much of the launch reserve this attempt used.
            view["budget"] = match store.work_budget(&agent.work_id).map_err(|cause| root("budget", cause))? {
                Some(b) => json!({"limit_usd": b.limit_usd.0, "reserve_per_launch_usd": b.reserve_per_launch_usd.0}),
                None => Value::Null,
            };
            view["transcript"] = transcript_view(&agent);
            // Saved v31 cache-health display thresholds; Null before v31.
            let settings = store.settings().map_err(|cause| root("settings", cause))?;
            // Saved v36 note bound for the harness form (`enabled` false before v36).
            view["annotation_policy"] = match settings.values.annotation_note_limit() {
                Some(limit) => json!({"enabled": true, "note_max_chars": limit}),
                None => json!({"enabled": false, "note_max_chars": Value::Null}),
            };
            view["cache_policy"] = match settings.values.cache_health_policy() {
                Ok(Some((ratio, min))) => json!({"miss_ratio_percent": ratio, "min_input_tokens": min}),
                Ok(None) => Value::Null,
                Err(cause) => return Err(root("settings", RootStoreError::Settings(cause))),
            };
            // The task this attempt executes, as planned (Null if the Work no
            // longer lists it); a Work read failure fails the view visibly.
            let work = store.read_work(&agent.work_id).map_err(|cause| root("read", cause))?;
            view["planned_task"] = work
                .tasks
                .iter()
                .flatten()
                .find(|t| t.id == agent.task_id)
                .map_or(Value::Null, plan_view);
            serde_json::to_vec(&json!({
                "schema_version": 1,
                "agent": view,
                "logs": logs,
                "terminal": terminal,
            }))
            .map_err(HttpServiceError::ResponseJson)
        })();
        close(result, store.close()).map(RouteBody::plain)
    }
}

impl NativeHttpService {
    /// `GET /api/v1/live/run?run=<review:id|planning:claim>&work=`: provider
    /// telemetry of one review job or planning run (LIVE-UX), with the run's
    /// kind, target, phase and `active` flag, and the saved cache-health
    /// thresholds (`cache_policy`, Null before v31). No estimated cost: the
    /// provider name of these runs is not persisted (only its digest), so
    /// no rate can be chosen. Refusals: missing/duplicate/unknown keys or a
    /// subject without a known prefix (`live-query-invalid`), a run absent
    /// from `work` (`live-run-unknown`, 404). A telemetry store failure is a
    /// state, as in the agent view. Read only.
    pub(super) fn live_run(
        &self,
        request: &Request<Vec<u8>>,
    ) -> Result<RouteBody, HttpServiceError> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(HttpServiceError::Method("GET, HEAD"));
        }
        let invalid = || HttpServiceError::Refused("live-query-invalid");
        let (mut run, mut work) = (None, None);
        for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
            let slot = match key.as_ref() {
                "run" => &mut run,
                "work" => &mut work,
                _ => return Err(invalid()),
            };
            if slot.replace(value.into_owned()).is_some() {
                return Err(invalid());
            }
        }
        let run = run
            .filter(|r| ["review:", "planning:"].iter().any(|p| r.strip_prefix(p).is_some_and(|id| !id.is_empty())))
            .ok_or_else(invalid)?;
        let work = work.filter(|w| !w.is_empty()).ok_or_else(invalid)?;
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let result = (|| {
            self.check_binding(&store)?;
            let found = store.telemetry_runs(&work, Some(&run)).map_err(|cause| root("read", cause))?;
            let Some(found) = found.into_iter().next() else {
                return Err(HttpServiceError::Refused("live-run-unknown"));
            };
            let settings = store.settings().map_err(|cause| root("settings", cause))?;
            let cache_policy = match settings.values.cache_health_policy() {
                Ok(Some((ratio, min))) => json!({"miss_ratio_percent": ratio, "min_input_tokens": min}),
                Ok(None) => Value::Null,
                Err(cause) => return Err(root("settings", RootStoreError::Settings(cause))),
            };
            serde_json::to_vec(&json!({
                "schema_version": 1,
                "work": work,
                "run": found,
                "telemetry": telemetry_view(&self.root, &run),
                "cache_policy": cache_policy,
            }))
            .map_err(HttpServiceError::ResponseJson)
        })();
        close(result, store.close()).map(RouteBody::plain)
    }

    /// `/api/v1/live/annotations` (LangWatch L10). `GET ?work=`: the Work's
    /// finished attempts (newest `ended` first) each with its operator
    /// annotation or Null, the queue (`queue`: finished attempts without
    /// one) and the saved v36 note bound (`note_max_chars`, Null =
    /// unlimited; `policy` false before v36). `POST` (session + CSRF) body
    /// `{"schema_version":1,"work","agent","verdict","note","expected_revision"}`
    /// creates or changes one annotation (see
    /// [`swarm_engine::TelemetryStore::annotate`]). Refusals: shape
    /// (`annotation-request-invalid`, `live-query-invalid`), unknown agent
    /// (`live-agent-unknown`, 404) or outside the Work
    /// (`live-agent-work-mismatch`), attempt still active
    /// (`annotation-agent-active`), settings before v36
    /// (`annotation-policy-missing`), note longer than the bound
    /// (`annotation-note-too-long`) or holding control characters other than
    /// newline and tab (`annotation-note-invalid`), stale revision
    /// (`annotation-revision-conflict`, 409), busy store (503). The note is
    /// operator text, stored as written; nothing is sent anywhere.
    pub(super) fn live_annotations(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        match *request.method() {
            Method::GET | Method::HEAD => self.read_annotations(request),
            Method::POST => self.write_annotation(request),
            _ => Err(HttpServiceError::Method("GET, HEAD, POST")),
        }
    }

    fn read_annotations(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        let invalid = || HttpServiceError::Refused("live-query-invalid");
        let mut work = None;
        for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
            if key != "work" || work.replace(value.into_owned()).is_some() {
                return Err(invalid());
            }
        }
        let work = work.filter(|w| !w.is_empty()).ok_or_else(invalid)?;
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let read = (|| {
            self.check_binding(&store)?;
            store.read_work(&work).map_err(|cause| root("read", cause))?;
            let agents = store.list_agents(&work).map_err(|cause| root("read", cause))?;
            let settings = store.settings().map_err(|cause| root("settings", cause))?;
            Ok((agents, settings.values.annotation_note_limit()))
        })();
        let (agents, limit) = close(read, store.close())?;
        let annotations = match swarm_engine::TelemetryStore::open(&self.root, false) {
            Ok(store) => {
                let read = store.read_annotations(&work);
                let closed = store.close();
                match (read, closed) {
                    (Ok(v), Ok(())) => v,
                    (Err(e), _) | (Ok(_), Err(e)) => return Err(HttpServiceError::Telemetry(Box::new(e))),
                }
            }
            Err(swarm_engine::TelemetryStoreError::Refused("telemetry-store-missing")) => Default::default(),
            Err(e) => return Err(HttpServiceError::Telemetry(Box::new(e))),
        };
        let ended = |a: &Agent| swarm_wire_json::go_rfc3339::parse(&a.ended).ok();
        let mut finished: Vec<&Agent> = agents.iter().filter(|a| !ACTIVE.contains(&a.status.as_str())).collect();
        finished.sort_by(|x, y| ended(y).cmp(&ended(x)).then_with(|| x.id.cmp(&y.id)));
        let items: Vec<Value> = finished
            .iter()
            .map(|a| json!({"agent_id": a.id, "role": a.role, "task_id": a.task_id, "status": a.status, "ended": a.ended, "annotation": annotations.get(&a.id)}))
            .collect();
        let queue: Vec<&str> = finished.iter().filter(|a| !annotations.contains_key(&a.id)).map(|a| a.id.as_str()).collect();
        serde_json::to_vec(&json!({"schema_version": 1, "work": work, "policy": limit.is_some(), "note_max_chars": limit.flatten(), "attempts": items, "queue": queue}))
            .map(RouteBody::plain)
            .map_err(HttpServiceError::ResponseJson)
    }

    fn write_annotation(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Change {
            schema_version: i64,
            work: String,
            agent: String,
            verdict: String,
            note: String,
            expected_revision: i64,
        }
        if request.uri().query().is_some_and(|q| !q.is_empty()) {
            return Err(HttpServiceError::Refused("live-query-invalid"));
        }
        let change: Change = serde_json::from_slice(request.body()).map_err(|_| HttpServiceError::Refused("annotation-request-invalid"))?;
        if change.schema_version != 1 || change.expected_revision < 0 {
            return Err(HttpServiceError::Refused("annotation-request-invalid"));
        }
        if change.note.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
            return Err(HttpServiceError::Refused("annotation-note-invalid"));
        }
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let read = (|| {
            self.check_binding(&store)?;
            let agent = match store.read_agent(&change.agent) {
                Ok(agent) => agent,
                Err(RootStoreError::Agent(swarm_engine::AgentStoreError::Missing)) => return Err(HttpServiceError::Refused("live-agent-unknown")),
                Err(cause) => return Err(root("read", cause)),
            };
            let settings = store.settings().map_err(|cause| root("settings", cause))?;
            Ok((agent, settings.values.annotation_note_limit()))
        })();
        let (agent, limit) = close(read, store.close())?;
        if agent.work_id != change.work {
            return Err(HttpServiceError::Refused("live-agent-work-mismatch"));
        }
        if ACTIVE.contains(&agent.status.as_str()) {
            return Err(HttpServiceError::Refused("annotation-agent-active"));
        }
        let limit = limit.ok_or(HttpServiceError::Refused("annotation-policy-missing"))?;
        if limit.is_some_and(|max| i64::try_from(change.note.chars().count()).unwrap_or(i64::MAX) > max) {
            return Err(HttpServiceError::Refused("annotation-note-too-long"));
        }
        let now = now_ms().ok_or(HttpServiceError::Refused("live-clock-invalid"))?;
        let mut telemetry = swarm_engine::TelemetryStore::open(&self.root, true).map_err(|e| HttpServiceError::Telemetry(Box::new(e)))?;
        let written = telemetry
            .ensure_current()
            .and_then(|()| telemetry.annotate(&agent.id, &agent.work_id, &change.verdict, &change.note, change.expected_revision, now));
        let closed = telemetry.close();
        let annotation = match (written, closed) {
            (Ok(a), Ok(())) => a,
            (Err(e), _) | (Ok(_), Err(e)) => return Err(HttpServiceError::Telemetry(Box::new(e))),
        };
        serde_json::to_vec(&json!({"schema_version": 1, "annotation": annotation}))
            .map(RouteBody::plain)
            .map_err(HttpServiceError::ResponseJson)
    }

    /// `GET /api/v1/live/dataset?work=&select=failed|bad|all` (LangWatch
    /// L8): regression entries of one Work built from recorded facts only —
    /// planned task (id, title, deliverable, criteria), attempt outcome and
    /// failure cause ([`swarm_engine::failure_cause`]), declared usage,
    /// operator annotation, provider-telemetry summary (calls, tools,
    /// failed tools). `failed`: failed or limit-stopped attempts; `bad`:
    /// attempts annotated `bad`; `all`: every finished attempt. Never
    /// prompts, replies, transcripts or tool content. Refusals: query shape
    /// (`live-query-invalid`), unknown Work (Root error). A telemetry or
    /// annotation store failure refuses the export rather than producing a
    /// silently partial dataset. Read only.
    pub(super) fn live_dataset(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(HttpServiceError::Method("GET, HEAD"));
        }
        let invalid = || HttpServiceError::Refused("live-query-invalid");
        let (mut work, mut select) = (None, None);
        for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
            let slot = match key.as_ref() {
                "work" => &mut work,
                "select" => &mut select,
                _ => return Err(invalid()),
            };
            if slot.replace(value.into_owned()).is_some() {
                return Err(invalid());
            }
        }
        let work = work.filter(|w| !w.is_empty()).ok_or_else(invalid)?;
        let select = select.filter(|s| matches!(s.as_str(), "failed" | "bad" | "all")).ok_or_else(invalid)?;
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let read = (|| {
            self.check_binding(&store)?;
            let recorded = store.read_work(&work).map_err(|cause| root("read", cause))?;
            let agents = store.list_agents(&work).map_err(|cause| root("read", cause))?;
            Ok((recorded, agents))
        })();
        let (recorded, agents) = close(read, store.close())?;
        let telemetry = match swarm_engine::TelemetryStore::open(&self.root, false) {
            Ok(store) => Some(store),
            Err(swarm_engine::TelemetryStoreError::Refused("telemetry-store-missing")) => None,
            Err(e) => return Err(HttpServiceError::Telemetry(Box::new(e))),
        };
        let built = (|| -> Result<Vec<Value>, swarm_engine::TelemetryStoreError> {
            let annotations = match &telemetry {
                Some(t) => t.read_annotations(&work)?,
                None => Default::default(),
            };
            let mut entries = Vec::new();
            for a in agents.iter().filter(|a| !ACTIVE.contains(&a.status.as_str())) {
                let failed = a.status == "failed" || a.diagnostic.limit_reached;
                let annotation = annotations.get(&a.id);
                let keep = match select.as_str() {
                    "failed" => failed,
                    "bad" => annotation.is_some_and(|n| n.verdict == "bad"),
                    _ => true,
                };
                if !keep {
                    continue;
                }
                let task = recorded.tasks.iter().flatten().find(|t| t.id == a.task_id);
                let cause = failed.then(|| swarm_engine::failure_cause(a));
                let summary = match &telemetry {
                    Some(t) => {
                        let v = t.read_agent(&a.id)?;
                        json!({"state": v.state, "calls": v.calls.len(), "calls_more": v.calls_more,
                               "tools": v.tools.iter().map(|x| json!({"name": x.name, "count": x.count, "failures": x.failures})).collect::<Vec<_>>()})
                    }
                    None => json!({"state": "no-token"}),
                };
                entries.push(json!({
                    "agent_id": a.id, "provider": a.provider, "role": a.role, "status": a.status,
                    "started": a.started, "ended": a.ended, "exit_code": a.exit_code,
                    "task": task.map(|t| json!({"id": t.id, "title": t.title, "deliverable": t.deliverable, "criteria": t.criteria, "status": t.status})),
                    "cause": cause.map(|(source, key, label)| json!({"source": source, "key": key, "label": label})),
                    "diagnostic": {"observed_errors": a.diagnostic.observed_errors, "limit_reached": a.diagnostic.limit_reached, "limit_reason": a.diagnostic.limit_reason},
                    "usage": a.usage.as_ref().map(|u| json!({"input_tokens": u.input_tokens, "output_tokens": u.output_tokens, "cost_usd": u.provider_reported_cost_usd.as_deref().map(|c| c.0)})),
                    "annotation": annotation.map(|n| json!({"verdict": n.verdict, "note": n.note, "revision": n.revision})),
                    "telemetry": summary,
                }));
            }
            Ok(entries)
        })();
        let closed = telemetry.map_or(Ok(()), swarm_engine::TelemetryStore::close);
        let entries = match (built, closed) {
            (Ok(e), Ok(())) => e,
            (Err(e), _) | (Ok(_), Err(e)) => return Err(HttpServiceError::Telemetry(Box::new(e))),
        };
        serde_json::to_vec(&json!({"schema_version": 1, "work": work, "select": select, "provenance": "recorded-facts-no-content", "entries": entries}))
            .map(RouteBody::plain)
            .map_err(HttpServiceError::ResponseJson)
    }

    /// `/api/v1/live/scenario` (LangWatch L6). `GET ?work=&task=`: the
    /// task's saved scenario (or Null), its latest attempt and the result of
    /// each expectation judged against recorded facts
    /// ([`swarm_engine::evaluate_scenario`]). `POST` (session + CSRF)
    /// `{"schema_version":1,"work","task","scenario":{…},"expected_revision"}`
    /// saves it. Refusals: shape (`live-query-invalid`,
    /// `scenario-request-invalid`), unknown task (`live-task-unknown`, 404),
    /// invalid specification (`scenario-version-unsupported`,
    /// `scenario-empty`, `scenario-value-invalid`), stale revision
    /// (`scenario-revision-conflict`, 409), busy store (503).
    pub(super) fn live_scenario(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        match *request.method() {
            Method::GET | Method::HEAD => self.read_scenario(request),
            Method::POST => self.write_scenario(request),
            _ => Err(HttpServiceError::Method("GET, HEAD, POST")),
        }
    }

    fn read_scenario(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        let invalid = || HttpServiceError::Refused("live-query-invalid");
        let (mut work, mut task) = (None, None);
        for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
            let slot = match key.as_ref() {
                "work" => &mut work,
                "task" => &mut task,
                _ => return Err(invalid()),
            };
            if slot.replace(value.into_owned()).is_some() {
                return Err(invalid());
            }
        }
        let (work, task) = (work.filter(|w| !w.is_empty()).ok_or_else(invalid)?, task.filter(|t| !t.is_empty()).ok_or_else(invalid)?);
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let read = (|| {
            self.check_binding(&store)?;
            let recorded = store.read_work(&work).map_err(|cause| root("read", cause))?;
            let title = recorded.tasks.iter().flatten().find(|t| t.id == task).map(|t| t.title.clone()).ok_or(HttpServiceError::Refused("live-task-unknown"))?;
            Ok((title, store.list_agents(&work).map_err(|cause| root("read", cause))?))
        })();
        let (title, agents) = close(read, store.close())?;
        let latest = latest_attempt(&agents, &work, &task);
        let telemetry = match swarm_engine::TelemetryStore::open(&self.root, false) {
            Ok(store) => Some(store),
            Err(swarm_engine::TelemetryStoreError::Refused("telemetry-store-missing")) => None,
            Err(e) => return Err(HttpServiceError::Telemetry(Box::new(e))),
        };
        let built = (|| -> Result<Value, swarm_engine::TelemetryStoreError> {
            let Some(store) = &telemetry else { return Ok(json!({"scenario": Value::Null, "results": Value::Null})) };
            let Some((spec, revision, at)) = store.read_scenarios(&work)?.remove(&task) else { return Ok(json!({"scenario": Value::Null, "results": Value::Null})) };
            let view = match latest { Some(a) => Some(store.read_agent(&a.id)?), None => None };
            let results = swarm_engine::evaluate_scenario(&spec, latest, view.as_ref().filter(|v| v.state == "received"));
            Ok(json!({"scenario": {"spec": spec, "revision": revision, "updated_ms": at}, "results": results}))
        })();
        let closed = telemetry.map_or(Ok(()), swarm_engine::TelemetryStore::close);
        let mut body = match (built, closed) {
            (Ok(v), Ok(())) => v,
            (Err(e), _) | (Ok(_), Err(e)) => return Err(HttpServiceError::Telemetry(Box::new(e))),
        };
        body["schema_version"] = json!(1);
        body["task"] = json!({"id": task, "title": title});
        body["attempt"] = latest.map_or(Value::Null, |a| json!({"agent_id": a.id, "status": a.status, "started": a.started}));
        serde_json::to_vec(&body).map(RouteBody::plain).map_err(HttpServiceError::ResponseJson)
    }

    fn write_scenario(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Change {
            schema_version: i64,
            work: String,
            task: String,
            scenario: swarm_engine::ScenarioSpec,
            expected_revision: i64,
        }
        if request.uri().query().is_some_and(|q| !q.is_empty()) {
            return Err(HttpServiceError::Refused("live-query-invalid"));
        }
        let change: Change = serde_json::from_slice(request.body()).map_err(|_| HttpServiceError::Refused("scenario-request-invalid"))?;
        if change.schema_version != 1 || change.expected_revision < 0 {
            return Err(HttpServiceError::Refused("scenario-request-invalid"));
        }
        swarm_engine::validate_scenario(&change.scenario).map_err(HttpServiceError::Refused)?;
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let read = (|| {
            self.check_binding(&store)?;
            let recorded = store.read_work(&change.work).map_err(|cause| root("read", cause))?;
            if !recorded.tasks.iter().flatten().any(|t| t.id == change.task) {
                return Err(HttpServiceError::Refused("live-task-unknown"));
            }
            Ok(())
        })();
        close(read, store.close())?;
        let now = now_ms().ok_or(HttpServiceError::Refused("live-clock-invalid"))?;
        let mut telemetry = swarm_engine::TelemetryStore::open(&self.root, true).map_err(|e| HttpServiceError::Telemetry(Box::new(e)))?;
        let saved = telemetry
            .ensure_current()
            .and_then(|()| telemetry.save_scenario(&change.work, &change.task, &change.scenario, change.expected_revision, now));
        let closed = telemetry.close();
        let revision = match (saved, closed) {
            (Ok(r), Ok(())) => r,
            (Err(e), _) | (Ok(_), Err(e)) => return Err(HttpServiceError::Telemetry(Box::new(e))),
        };
        serde_json::to_vec(&json!({"schema_version": 1, "revision": revision})).map(RouteBody::plain).map_err(HttpServiceError::ResponseJson)
    }

    /// Saved v33 retention applied once at server start, before the telemetry
    /// endpoint is published (no ingest can race it yet), so expired console
    /// rows go even when this Root receives nothing new. Observable in the
    /// purge summary (`retention.last_purge`). Errors (mission store,
    /// settings, console store including busy) refuse the start explicitly.
    pub(super) fn startup_retention(&self) -> Result<swarm_engine::PurgeReport, HttpServiceError> {
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let read = (|| {
            self.check_binding(&store)?;
            store.settings().map_err(|cause| root("settings", cause))
        })();
        let settings = close(read, store.close())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .ok_or(HttpServiceError::Refused("live-clock-invalid"))?;
        swarm_engine::apply_saved_retention(&self.root, &settings.values, now).map_err(|e| HttpServiceError::Telemetry(Box::new(e)))
    }
}

/// Evaluate (L4) every finished attempt that is sampled and not yet
/// evaluated, reading its telemetry from the same store, and record the
/// results in one transaction. Returns the number of attempts recorded.
fn evaluate_finished(
    telemetry: &mut swarm_engine::TelemetryStore,
    agents: &[Agent],
    policy: swarm_engine::EvaluationPolicy,
    percent: i64,
    now: i64,
) -> Result<usize, swarm_engine::TelemetryStoreError> {
    telemetry.ensure_current()?;
    let done = telemetry.evaluated_agents()?;
    let mut batch = Vec::new();
    for a in agents.iter().filter(|a| !ACTIVE.contains(&a.status.as_str()) && !done.contains(&a.id) && swarm_engine::evaluation_sampled(&a.id, percent)) {
        let view = telemetry.read_agent(&a.id)?;
        let measured = (view.state == "received").then_some(&view);
        batch.push((a.id.clone(), a.work_id.clone(), swarm_engine::evaluate_attempt(a, measured, policy)));
    }
    telemetry.record_evaluations(&batch, now)
}

/// In-process serialization of alert evaluations (see `live_alerts_evaluate`).
static ALERT_EVALUATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Last outcome of the server alert loop per Root (settings v35), shown by
/// `GET /api/v1/live/alerts` as `server_evaluation`. In memory only: a
/// restarted server starts again from `starting`.
static SERVER_EVALUATIONS: std::sync::Mutex<std::collections::BTreeMap<std::path::PathBuf, Value>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

fn record_server_evaluation(root: &std::path::Path, state: Value) {
    SERVER_EVALUATIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(root.to_path_buf(), state);
}

fn now_ms() -> Option<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
}

/// Server-side alert evaluation loop (settings v35), owned by the web
/// server: started after the console store retention at server start,
/// stopped and joined before the service closes. See
/// [`NativeHttpService::start_alert_loop`].
pub(crate) struct AlertLoop {
    stop: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    handle: std::thread::JoinHandle<()>,
}
impl AlertLoop {
    /// Ask the loop to stop and wait for it: an evaluation in progress
    /// finishes first (never interrupted mid-transaction). A panicked loop
    /// is reported as `web-alert-loop-panicked`; the recorded state of
    /// `root` becomes `stopped` (`server-stopped`).
    pub(crate) fn stop(self, root: &std::path::Path) -> Result<(), HttpServiceError> {
        let (flag, wake) = &*self.stop;
        *flag.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        wake.notify_all();
        let joined = self.handle.join().map_err(|_| HttpServiceError::Refused("web-alert-loop-panicked"));
        // The view must not keep showing `running` for a server that stopped.
        record_server_evaluation(root, json!({"state": "stopped", "reason": "server-stopped", "last_ms": now_ms()}));
        joined
    }
}

impl NativeHttpService {
    /// Start the server alert loop when the saved cadence (v35
    /// `agent_alert_evaluation_seconds`) is set; `Ok(None)` when it is null
    /// or the document predates v35 (state `off`). Each cycle waits the
    /// cadence, then runs [`Self::evaluate_alerts`]: its outcome (`ok` with
    /// the time, or `error` with the stable code) is recorded for
    /// `GET /api/v1/live/alerts`; a failed cycle is not retried early, the
    /// next one runs at the cadence. The cadence is re-read from each
    /// evaluation, so a new cadence applies after the wait in progress; null
    /// stops the loop (state `stopped`) until restart. A panic inside one
    /// evaluation ends the loop with state `panicked`.
    /// Errors: Root open/binding/settings read at start.
    pub(crate) fn start_alert_loop(self: &std::sync::Arc<Self>) -> Result<Option<AlertLoop>, HttpServiceError> {
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let read = (|| {
            self.check_binding(&store)?;
            store.settings().map_err(|cause| root("settings", cause))
        })();
        let settings = close(read, store.close())?;
        let Some(seconds) = settings.values.alert_evaluation_seconds() else {
            record_server_evaluation(&self.root, json!({"state": "off"}));
            return Ok(None);
        };
        record_server_evaluation(&self.root, json!({"state": "starting", "interval_seconds": seconds}));
        let stop = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        let (service, signal) = (std::sync::Arc::clone(self), std::sync::Arc::clone(&stop));
        let handle = std::thread::Builder::new()
            .name("swarm-alert-loop".into())
            .spawn(move || {
                let mut seconds = seconds;
                loop {
                    let (flag, wake) = &*signal;
                    let guard = flag.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let wait = std::time::Duration::from_secs(u64::try_from(seconds).unwrap_or(u64::MAX));
                    let (guard, _) = wake.wait_timeout_while(guard, wait, |stopped| !*stopped).unwrap_or_else(std::sync::PoisonError::into_inner);
                    if *guard {
                        return;
                    }
                    drop(guard);
                    // A panic in one evaluation is recorded and ends the loop
                    // visibly (state `panicked`), instead of a stale `running`.
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| service.evaluate_alerts()));
                    let Ok(outcome) = outcome else {
                        record_server_evaluation(&service.root, json!({"state": "panicked", "last_ms": now_ms()}));
                        return;
                    };
                    match outcome {
                        Ok(view) => match view["alert_evaluation_seconds"].as_i64() {
                            Some(next) => {
                                seconds = next;
                                record_server_evaluation(&service.root, json!({"state": "running", "interval_seconds": next, "last_ms": now_ms(), "fired": view["sync"]["fired"].as_array().map_or(0, Vec::len)}));
                            }
                            None => {
                                record_server_evaluation(&service.root, json!({"state": "stopped", "reason": "setting-null", "last_ms": now_ms()}));
                                return;
                            }
                        },
                        Err(error) => record_server_evaluation(&service.root, json!({"state": "error", "interval_seconds": seconds, "code": error.code(), "last_ms": now_ms()})),
                    }
                }
            })
            .map_err(|_| HttpServiceError::Refused("web-alert-loop-spawn-failed"))?;
        Ok(Some(AlertLoop { stop, handle }))
    }
}

impl NativeHttpService {
    /// `GET /api/v1/live/alerts`: open supervisor alerts and resolved history
    /// from the console store (empty when no store exists yet, or when it
    /// predates alerts), plus `retention`: the saved v33 policy (Null before
    /// v33) and the cumulative purge summary (Null before any purge). Read
    /// only: an older store is never migrated here.
    pub(super) fn live_alerts(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(HttpServiceError::Method("GET, HEAD"));
        }
        if request.uri().query().is_some_and(|q| !q.is_empty()) {
            return Err(HttpServiceError::Refused("live-query-invalid"));
        }
        // Saved v33 bounds, shown next to the alerts (Null before v33).
        let mission = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let policy = (|| {
            self.check_binding(&mission)?;
            let settings = mission.settings().map_err(|cause| root("settings", cause))?;
            match settings.values.telemetry_retention_policy() {
                Ok(Some((days, cap))) => Ok(json!({"days": days, "max_events_per_attempt": cap})),
                Ok(None) => Ok(Value::Null),
                Err(cause) => Err(root("settings", RootStoreError::Settings(cause))),
            }
        })();
        let policy = close(policy, mission.close())?;
        let (view, last_purge) = match swarm_engine::TelemetryStore::open(&self.root, false) {
            Ok(store) => {
                let read = store.read_alerts().and_then(|v| store.retention_state().map(|r| (v, r)));
                let closed = store.close();
                match (read, closed) {
                    (Ok(v), Ok(())) => v,
                    (Err(e), _) | (Ok(_), Err(e)) => return Err(HttpServiceError::Telemetry(Box::new(e))),
                }
            }
            Err(swarm_engine::TelemetryStoreError::Refused("telemetry-store-missing")) => (swarm_engine::AlertView::default(), None),
            Err(e) => return Err(HttpServiceError::Telemetry(Box::new(e))),
        };
        // v35 server loop outcome (Null when this process runs no web server loop).
        let server = SERVER_EVALUATIONS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(&self.root).cloned().unwrap_or(Value::Null);
        serde_json::to_vec(&json!({"schema_version": 1, "rules": swarm_engine::ALERT_RULES.iter().map(|(r, l)| json!({"rule": r, "level": l})).collect::<Vec<_>>(), "alerts": view, "retention": {"policy": policy, "last_purge": last_purge}, "server_evaluation": server}))
            .map(RouteBody::plain)
            .map_err(HttpServiceError::ResponseJson)
    }

    /// `POST /api/v1/live/alerts/evaluate` (session + CSRF): compute the
    /// conditions true now from the mission store ([`swarm_engine::alert_conditions`])
    /// and reconcile the durable alert history ([`swarm_engine::TelemetryStore::sync_alerts`]),
    /// creating or migrating the console store if needed. Body must be empty
    /// or `{}` (`live-query-invalid`). Returns the fired and resolved
    /// transitions plus the current view. Evaluations of this process are
    /// serialized from the mission read to the reconcile. Errors: mission
    /// store refusals, `telemetry-store-busy` / `-unavailable` (503, nothing
    /// kept), `live-clock-invalid` (500). The saved v33 retention purge runs
    /// after the reconcile (no write when nothing is due) and its counts are
    /// returned as `purged`, with `retention` as in the GET view. The same
    /// evaluation also runs in the server loop when the v35 cadence is set
    /// ([`NativeHttpService::start_alert_loop`]); with the cadence null it
    /// runs only when a client calls this route, and a transition that
    /// happened meanwhile is recorded at the next call (engine `occurred`
    /// kept when known). See [`NativeHttpService::evaluate_alerts`] for the
    /// L4 evaluations and the per-Work budget failures (`budget_errors`).
    pub(super) fn live_alerts_evaluate(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        if request.method() != Method::POST {
            return Err(HttpServiceError::Method("POST"));
        }
        let body = request.body().as_slice();
        if !(body.is_empty() || body.iter().filter(|b| !b.is_ascii_whitespace()).eq(b"{}".iter())) {
            return Err(HttpServiceError::Refused("live-query-invalid"));
        }
        let view = self.evaluate_alerts()?;
        serde_json::to_vec(&view).map(RouteBody::plain).map_err(HttpServiceError::ResponseJson)
    }

    /// One alert evaluation, shared by the console POST and the server loop
    /// (settings v35): read Works, agents, budgets and settings, compute the
    /// conditions (engine marks, v34 usage gap, v35 budget rules), reconcile
    /// the durable history, apply v33 retention, and return the evaluation
    /// view. Errors keep their stable code; nothing is retried here. The
    /// returned `alert_evaluation_seconds` is the saved cadence read in this
    /// same snapshot (`None` = server loop off). A Work whose budget cannot
    /// be read (malformed policy, invalid declared cost) is listed in
    /// `budget_errors` with its code and only its budget rules are skipped;
    /// an open budget alert of that Work stays open (its condition is carried
    /// over from the open-alert page; an alert beyond that page would resolve).
    /// `evaluated` counts attempts newly evaluated (L4, v37).
    pub(crate) fn evaluate_alerts(&self) -> Result<Value, HttpServiceError> {
        // Serialize read + reconcile in this process: an evaluation that read
        // an older mission state must not reconcile after a newer one (flap).
        // Store busy timeout stays zero; another writer (e.g. OTLP ingest)
        // still gets `telemetry-store-busy` (503).
        let _serial = ALERT_EVALUATION.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let read = (|| {
            self.check_binding(&store)?;
            let works = store.list_works().map_err(|cause| root("read", cause))?;
            let mut agents = Vec::new();
            for w in &works {
                agents.extend(store.list_agents(&w.id).map_err(|cause| root("read", cause))?);
            }
            let settings = store.settings().map_err(|cause| root("settings", cause))?;
            let policy = settings.values.telemetry_retention_policy().map_err(|cause| root("settings", RootStoreError::Settings(cause)))?;
            let mismatch = settings.values.usage_mismatch_percent();
            // One Work's unreadable budget (malformed policy or declared cost)
            // skips the budget rules of that Work only, reported with its code;
            // every other rule and Work is still evaluated.
            let (mut budgets, mut budget_errors) = (Vec::new(), Vec::new());
            for w in &works {
                match store.work_budget_state(&w.id) {
                    Ok(state) => budgets.extend(state),
                    Err(cause) => budget_errors.push(json!({"work": w.id, "code": cause.code()})),
                }
            }
            let evaluation = swarm_engine::EvaluationPolicy {
                cache: settings.values.cache_health_policy().map_err(|cause| root("settings", RootStoreError::Settings(cause)))?,
                usage_mismatch_percent: mismatch,
            };
            let sample = settings.values.evaluation_sample_percent();
            Ok((works, agents, policy, mismatch, (budgets, budget_errors), settings.values.budget_warning_percent(), settings.values.alert_evaluation_seconds(), evaluation, sample))
        })();
        let (works, agents, policy, mismatch, (budgets, budget_errors), warning, cadence, evaluation, sample) = close(read, store.close())?;
        let mut conditions = swarm_engine::alert_conditions(&works, &agents);
        // Settings v35 (L7): budget threshold and after-the-fact overspend.
        conditions.extend(swarm_engine::budget_conditions(&budgets, warning));
        let present = swarm_engine::alert_subjects(&works, &agents);
        let days = policy.and_then(|(days, _)| days);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .ok_or(HttpServiceError::Refused("live-clock-invalid"))?;
        let mut telemetry = swarm_engine::TelemetryStore::open(&self.root, true).map_err(|e| HttpServiceError::Telemetry(Box::new(e)))?;
        // Settings v34: declared vs telemetry output-token gap (rule off when null).
        if let Some(percent) = mismatch {
            match telemetry.output_tokens_by_agent() {
                Ok(sums) => conditions.extend(swarm_engine::usage_mismatch_conditions(&works, &agents, &sums, percent)),
                Err(e) => {
                    // Primary cause first, as for every other store failure here.
                    let _closing = telemetry.close();
                    return Err(HttpServiceError::Telemetry(Box::new(e)));
                }
            }
        }
        // Settings v37 (L4): evaluate each sampled finished attempt once.
        let evaluated = match sample {
            Some(percent) => evaluate_finished(&mut telemetry, &agents, evaluation, percent, now),
            None => Ok(0),
        };
        let evaluated = match evaluated {
            Ok(n) => n,
            Err(e) => {
                let _closing = telemetry.close();
                return Err(HttpServiceError::Telemetry(Box::new(e)));
            }
        };
        // An unreadable budget keeps that Work's open budget alert open (its
        // condition is carried over) instead of resolving it as cleared.
        if !budget_errors.is_empty() {
            let open = match telemetry.read_alerts() {
                Ok(view) => view.open,
                Err(e) => {
                    let _closing = telemetry.close();
                    return Err(HttpServiceError::Telemetry(Box::new(e)));
                }
            };
            let unreadable: Vec<String> = budget_errors.iter().filter_map(|e| e["work"].as_str().map(|w| format!("budget:{w}"))).collect();
            for row in open.iter().filter(|r| unreadable.contains(&r.subject)) {
                if let Some(&(rule, level)) = swarm_engine::ALERT_RULES.iter().find(|(rule, _)| *rule == row.rule) {
                    conditions.push(swarm_engine::AlertCondition { rule, level, subject: row.subject.clone(), work_id: row.work_id.clone(), occurred: row.occurred.clone() });
                }
            }
        }
        let result = telemetry
            .sync_alerts(&conditions, &present, now)
            .and_then(|sync| match days {
                Some(days) => telemetry.apply_retention(days, now).map(|purged| (sync, purged)),
                None => Ok((sync, swarm_engine::PurgeReport::default())),
            })
            .and_then(|(sync, purged)| telemetry.read_alerts().map(|view| (sync, purged, view)))
            .and_then(|(sync, purged, view)| telemetry.retention_state().map(|last| (sync, purged, view, last)));
        let closed = telemetry.close();
        let (sync, purged, view, last_purge) = match (result, closed) {
            (Ok(v), Ok(())) => v,
            (Err(e), _) | (Ok(_), Err(e)) => return Err(HttpServiceError::Telemetry(Box::new(e))),
        };
        let policy = policy.map(|(days, cap)| json!({"days": days, "max_events_per_attempt": cap}));
        Ok(json!({"schema_version": 1, "sync": sync, "purged": purged, "alerts": view, "retention": {"policy": policy, "last_purge": last_purge}, "alert_evaluation_seconds": cadence, "evaluated": evaluated, "budget_errors": budget_errors,
            // v35 server loop outcome, as recorded in this process (Null when no loop ran here).
            "server_evaluation": SERVER_EVALUATIONS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(&self.root).cloned().unwrap_or(Value::Null)}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimated_cost_prices_only_undeclared_calls_with_a_matching_rate() {
        let rate = |model: &str, version: &str, updated: &str, input: f64| {
            let mut r = swarm_engine::decode_rate_change(format!(r#"{{"schema_version":1,"rate":{{"provider":"codex","model":"{model}","currency":"USD","input_per_million":{input},"output_per_million":10,"cache_read_per_million":0.5,"source":"synthetic","reference_date":"2026-10-22"}}}}"#).as_bytes()).unwrap().rate;
            r.version = version.into();
            r.updated = updated.into();
            r
        };
        let catalogue = swarm_engine::RateCatalogue { schema_version: 1, digest: String::new(), rates: Some(vec![
            rate("gpt-x", "old", "2026-10-01T00:00:00Z", 100.0), rate("gpt-x", "new", "2026-10-20T00:00:00Z", 2.0)]) };
        let call = |fields: Value| json!({"at_ms": 1, "fields": fields});
        let mut view = json!({"state": "received", "calls": [
            call(json!({"model": "gpt-x", "input_tokens": "1000000", "output_tokens": "100000", "cache_read_tokens": "2000000"})),
            call(json!({"model": "gpt-x", "input_tokens": "5", "cost_usd": 9.0})),
            call(json!({"model": "unknown-model", "input_tokens": "5", "output_tokens": "1"}))]});
        estimate_call_costs(&mut view, "codex", &catalogue);
        let e = &view["estimated_cost"];
        // newest rate: 1M input × 2 + 0.1M output × 10 + 2M cache × 0.5 = 2 + 1 + 1
        assert_eq!((e["amount"].as_f64(), &e["currency"], &e["priced_calls"], &e["unpriced_calls"]), (Some(4.0), &json!("USD"), &json!(1), &json!(1)));
        assert_eq!(e["rate_versions"], json!(["new"]));
        let mut claude = json!({"calls": [call(json!({"model": "m", "cost_usd": 1.0}))]});
        estimate_call_costs(&mut claude, "claude", &catalogue);
        assert!(claude.get("estimated_cost").is_none(), "declared costs are never re-estimated");
    }

    #[test]
    fn plan_view_reports_the_reviewed_candidate_or_null() {
        let mut task = swarm_engine::Task { id: "t1".into(), status: "review".into(), ..Default::default() };
        assert!(plan_view(&task)["delivery"].is_null(), "no review recorded");
        task.independent_review = Some(Box::new(swarm_engine::IndependentReview {
            candidate_commit: "0123456789abcdef0123".into(),
            previous_candidate: "fedcba".into(),
            state: "passed".into(),
            finished: "2026-10-22T10:00:00Z".into(),
            report: "PRIVATE report".into(),
            ..Default::default()
        }));
        task.previous_reviews = Some(vec![swarm_engine::IndependentReview::default()]);
        let v = plan_view(&task);
        assert_eq!(v["delivery"], json!({"commit": "0123456789abcdef0123", "previous": "fedcba", "review_state": "passed", "review_finished": "2026-10-22T10:00:00Z", "reviews": 2}));
        assert!(!v.to_string().contains("PRIVATE"), "review report never served");
    }

    #[test]
    fn agent_query_accepts_a_witness_then_refuses_ambiguity() {
        let ok = agent_query(Some("agent=a&work=w&logs_after=3")).ok().unwrap();
        assert_eq!((ok.agent.as_str(), ok.work.as_str()), ("a", "w"));
        assert_eq!((ok.logs_after, ok.terminal_after), (3, 0));
        for bad in [
            "agent=a&agent=b&work=w",
            "agent=a&work=w&extra=1",
            "agent=&work=w",
            "work=w",
        ] {
            assert!(
                matches!(
                    agent_query(Some(bad)),
                    Err(HttpServiceError::Refused("live-query-invalid"))
                ),
                "{bad}"
            );
        }
        for bad in ["agent=a&work=w&logs_after=-1", "agent=a&work=w&terminal_after=x"] {
            assert!(
                matches!(
                    agent_query(Some(bad)),
                    Err(HttpServiceError::Refused("live-cursor-invalid"))
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn every_owned_asset_is_embedded_and_unknown_paths_are_not_owned() {
        for path in ["/live.html", "/live-session.html", "/live/live.js", "/live/live.css"] {
            assert!(owns(path), "{path}");
        }
        for path in ["/live/../index.html", "/live/", "/live.htm", "/live/x.js"] {
            assert!(!owns(path), "{path}");
        }
    }
}
