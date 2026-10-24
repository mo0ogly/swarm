//! Explicit native mission HTTP bridge; not the historical Go snapshot route.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{RootStore, RootStoreError};

/// Exact query data, not a Root path, default mission or permission.
#[derive(Debug, PartialEq, Eq)]
struct Query { work: String, action: Option<String> }

/// Parse one safe Work and at most one nonempty action before Root access.
/// Unknown/duplicate keys and unsafe names refuse, never fall back to a Work.
/// This is protocol validation only; it performs no IO or policy change.
fn query(raw: Option<&str>) -> Result<Query, E> {
    let (mut work, mut action) = (None, None);
    for (key, value) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        match key.as_ref() {
            "work" if work.is_none() => work = Some(value.into_owned()),
            "action" if action.is_none() && !value.is_empty() => action = Some(value.into_owned()),
            _ => return Err(E::Refused("mission-query-invalid")),
        }
    }
    let work = work.filter(|v| swarm_identity_contract::name(v.as_bytes()))
        .ok_or(E::Refused("mission-query-invalid"))?;
    Ok(Query { work, action })
}

/// Fixed implemented read/write vocabulary, not admission or a running conductor.
/// Default GET show returns MissionControlView; historical status may still refuse
/// unsupported branches. This bridge acquires no lease and launches no agent.
pub(super) fn capabilities() -> [&'static str; 10] {
    ["show", "order", "queue", "status", "start", "pause", "unpause", "stop", "resume", "priority"]
}

/// Select a fixed write command and validate its actual CLI envelope before IO.
/// No profile, slot, reason, revision or event is invented. Decoders preserve
/// typed causes under a pre-Root input phase; ordinary transport size rules apply.
/// Successful validation is not consent, execution, a lease or future admission.
fn write_input(action: Option<&str>, raw: &[u8]) -> Result<&'static str, E> {
    let kind = match action {
        Some("start") => "mission.start", Some("pause") => "mission.pause",
        Some("unpause") => "mission.unpause", Some("stop") => "mission.stop",
        Some("resume") => "mission.resume", Some("priority") => "mission.priority",
        _ => return Err(E::Refused("web-operation-unported")),
    };
    swarm_engine::check_command_input_size(raw).map_err(E::Input)?;
    let result = match kind {
        "mission.start" => swarm_engine::decode_mission_start_request(raw),
        "mission.priority" => swarm_engine::decode_mission_priority_request(raw),
        _ => swarm_engine::decode_mission_control_request(raw),
    };
    result.map_err(|c| root("mission-input", RootStoreError::Mutation(swarm_engine::WorkEngineError::Mission(Box::new(c)))))?;
    Ok(kind)
}

/// Reuse the CLI's typed mission error wrapper; retain nested SQL/cleanup causes.
/// No diagnostic data is formatted and no failure becomes an empty success.
fn mission_error(cause: swarm_engine::MissionSupervisionError) -> E {
    root("read", RootStoreError::Mutation(swarm_engine::WorkEngineError::Mission(Box::new(cause))))
}

/// Attach every ancillary read cause to a later actual encode/close failure.
/// Successful degraded observations are valid DTOs, as in historical CLI; this
/// helper runs only after a real failure and retains all original typed causes.
fn retain_observation_error(mut primary: E, causes: Vec<RootStoreError>) -> E {
    for cause in causes { primary = E::Close { primary: Some(Box::new(primary)), cleanup: Box::new(cause) }; }
    primary
}

impl NativeHttpService {
    /// Read existing mission facts behind shared Host/session and Root lane.
    /// GET/HEAD accept show/order/queue/status without a body. POST validates the
    /// selected CLI mission document before opening the writable bound Root.
    /// Unknown operations refuse; no implicit Work, envelope or consent is supplied.
    /// A bound existing-Work read preserves its canonical not-found failure before
    /// the mission producer adds diagnostic wrappers; it grants no operation.
    /// Historical status preserves valid degraded DTOs. Ancillary diagnostic causes
    /// survive in the owner outcome and join any later encode/close/output failure.
    /// Writes use the same engine transaction/replay/guards and preserve known Work
    /// before encode/close. Start only records local configuration/consent; it
    /// never starts a conductor, agent or model. No new policy or visit is added.
    /// Explicit Root close preserves both failures; HEAD suppresses body
    /// in the shared response layer after applying the same access/read checks.
    pub(super) fn mission(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        let read = matches!(*request.method(), Method::GET | Method::HEAD);
        if !read && request.method() != Method::POST { return Err(E::Method("GET, HEAD, POST")); }
        let query = query(request.uri().query())?;
        let action = query.action.as_deref().unwrap_or("show");
        let operation = if read {
            if !matches!(action, "show" | "order" | "queue" | "status") { return Err(E::Refused("web-operation-unported")); }
            if !request.body().is_empty() { return Err(E::Refused("mission-read-body-invalid")); }
            None
        } else { Some(write_input(query.action.as_deref(), request.body())?) };
        let mut store = RootStore::open(&self.root, !read).map_err(|e| root("open", e))?;
        let mut recorded = None;
        let mut observation_causes = Vec::new();
        let result = (|| {
            self.check_binding(&store)?;
            // Preserve the canonical missing-Work error before mission wrappers
            // add their own diagnostic family. This read grants no operation.
            store.read_work(&query.work).map_err(|cause| root("read", cause))?;
            if let Some(kind) = operation {
                let work = store.execute_external(&query.work, kind, request.body(), &mut swarm_engine::SystemEnvironment)
                    .map_err(|c| root("mutate", c))?.result.work;
                recorded = Some((work.id.clone(), work.revision));
                let bytes = swarm_engine::encode_model(&work).map_err(E::Model)?;
                return Ok(RouteBody { observation_causes: Vec::new(), bytes, recorded_work: recorded.clone(), configuration_recorded: false, recorded_configuration: None });
            }
            let bytes = match action {
                "show" => serde_json::to_vec(&store.mission_control_view(&query.work).map_err(mission_error)?),
                "order" => serde_json::to_vec(&store.mission_task_order(&query.work).map_err(mission_error)?),
                "queue" => serde_json::to_vec(&store.mission_workspace_queue(&query.work).map_err(mission_error)?),
                "status" => {
                    let operator = swarm_engine::operator_identity().map_err(|c| root("read", RootStoreError::Runtime(c)))?;
                    let observation = store.mission_status_historical(&query.work, &operator).map_err(|c| root("read", c))?;
                    observation_causes = observation.causes;
                    serde_json::to_vec(&observation.status)
                }
                _ => return Err(E::Refused("web-operation-unported")),
            }.map_err(E::Json)?;
            Ok(RouteBody { observation_causes: Vec::new(), bytes, recorded_work: None, configuration_recorded: false, recorded_configuration: None })
        })();
        match close(result, store.close()) {
            Ok(mut body) => { body.observation_causes = observation_causes; Ok(body) }
            Err(primary) => {
                let primary = retain_observation_error(primary, observation_causes);
                match recorded {
                    Some((work, revision)) => Err(E::Recorded { primary: Box::new(primary), work, revision }),
                    None => Err(primary),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Valid witness precedes duplicate, unsafe and unsupported query data.
    #[test]
    fn mission_query_has_no_ambiguous_work_or_action_fallback() {
        assert_eq!(query(Some("work=w%2Done&action=queue")).unwrap(), Query { work:"w-one".into(), action:Some("queue".into()) });
        assert_eq!(query(Some("work=w-one")).unwrap().action, None);
        for raw in [None,Some(""),Some("action=show"),Some("work="),Some("work=../w"),Some("work=w&work=w"),Some("work=w&action="),Some("work=w&action=show&action=show"),Some("work=w&root=x")] {
            assert!(matches!(query(raw), Err(E::Refused("mission-query-invalid"))));
        }
        assert!(query(Some("work=w-one&action=unknown")).is_ok()); // operation layer owns refusal
    }
    /// Real CLI decoders separate action envelopes and preserve safe input errors.
    #[test]
    fn mission_write_admission_uses_actual_action_specific_cli_decoders() {
        let valid = br#"{"schema_version":1,"event_id":"fixture","expected_revision":1,"reason":"Explicit fixture"}"#;
        assert_eq!(write_input(Some("pause"), valid).unwrap(), "mission.pause");
        assert_eq!(write_input(Some("start"), valid).unwrap(), "mission.start");
        let priority = br#"{"schema_version":1,"event_id":"fixture","expected_revision":1,"reason":"Explicit fixture","task_id":"task","priority":"-9223372036854775808"}"#;
        assert_eq!(write_input(Some("priority"), priority).unwrap(), "mission.priority");
        for (action, raw) in [(Some("priority"),valid.as_slice()),(Some("pause"),priority.as_slice()),(Some("resume"),b"{}".as_slice()),(None,valid.as_slice()),(Some("show"),valid.as_slice())] {
            assert!(write_input(action, raw).is_err());
        }
        let error = match write_input(Some("priority"), valid) { Err(e) => e, Ok(_) => panic!("priority input must refuse") };
        assert_eq!(error.status(), http::StatusCode::BAD_REQUEST);
    }
    /// Ancillary observations join an actual failure, never invent a failed DTO.
    #[test]
    fn mission_observation_causes_retain_primary_and_cleanup() {
        let primary = E::Refused("fixture-primary");
        assert!(matches!(retain_observation_error(primary,Vec::new()),E::Refused("fixture-primary")));
        let error = retain_observation_error(E::Refused("fixture-primary"), vec![RootStoreError::Refused("fixture-first"),RootStoreError::Refused("fixture-second")]);
        match error { E::Close { primary:Some(primary), cleanup } => {
            assert!(matches!(*cleanup,RootStoreError::Refused("fixture-second")));
            match *primary { E::Close { primary:Some(primary), cleanup } => {
                assert!(matches!(*primary,E::Refused("fixture-primary")));
                assert!(matches!(*cleanup,RootStoreError::Refused("fixture-first")));
            }, _ => panic!("first observation cause required") }
        }, _ => panic!("both observation causes required") }
    }
}
