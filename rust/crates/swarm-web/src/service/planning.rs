//! Go-compatible protected planning reads and already-ported operator controls.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{RootStore, SystemEnvironment};

/// Exact URL data, never a filesystem root, fallback mission or permission grant.
#[derive(Debug, PartialEq, Eq)]
struct Query {
    work: String,
    action: Option<String>,
}

/// Decode one safe Work and at most one explicit action before Root IO.
/// Duplicate/unknown keys, empty values and traversal-shaped Work refuse. A read
/// without action means show; an explicit unsupported action never becomes show.
fn query(raw: Option<&str>) -> Result<Query, E> {
    let (mut work, mut action) = (None, None);
    for (key, value) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        match key.as_ref() {
            "work" if work.is_none() => work = Some(value.into_owned()),
            "action" if action.is_none() && !value.is_empty() => action = Some(value.into_owned()),
            _ => return Err(E::Refused("planning-query-invalid")),
        }
    }
    let work = work
        .filter(|v| swarm_identity_contract::name(v.as_bytes()))
        .ok_or(E::Refused("planning-query-invalid"))?;
    Ok(Query { work, action })
}

/// Static protocol routing to supported Root commands. No provider/process
/// operation, managed preview or arbitrary engine kind is admitted by user text.
fn kind(action: Option<&str>) -> Result<&'static str, E> {
    match action {
        Some("enable") => Ok("planning.enable"),
        Some("claim") => Ok("planning.claim"),
        Some("lease-renew") => Ok("planning.lease-renew"),
        Some("decide") => Ok("planning.decide"),
        Some("judge") => Ok("planning.judge"),
        Some("renew") => Ok("planning.renew"),
        Some("pause") => Ok("planning.pause"),
        Some("resume") => Ok("planning.resume"),
        Some("reject-instructions") => Ok("planning.reject-instructions"),
        _ => Err(E::Refused("web-operation-unported")),
    }
}

/// Advertise the implemented planning route entrypoints, not admission for a
/// particular Work or platform. Root still validates policy, ownership, schema
/// and current state; no model launch or managed preview is promised.
/// Allocation is proportional to this fixed protocol vocabulary, with no IO.
pub(super) fn capabilities() -> [&'static str; 10] {
    [
        "show",
        "enable",
        "claim",
        "lease-renew",
        "decide",
        "judge",
        "renew",
        "pause",
        "resume",
        "reject-instructions",
    ]
}

impl NativeHttpService {
    /// GET/HEAD retained decisions for one existing Work; strict query/body and
    /// common admission/binding before Root's readonly snapshot. Stored operations
    /// are never executed or rejudged as current authority. Preserve all typed
    /// read/response-encoding/close causes, with no empty fallback or provider.
    pub(super) fn planning_history(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E>{
        if !matches!(*request.method(),Method::GET|Method::HEAD){return Err(E::Method("GET, HEAD"))}
        if !request.body().is_empty(){return Err(E::Refused("planning-history-body-invalid"))}
        let q=query(request.uri().query())?;if q.action.is_some(){return Err(E::Refused("planning-history-query-invalid"))}
        let store=RootStore::open(&self.root,false).map_err(|e|root("open",e))?;
        let result=(||{self.check_binding(&store)?;let value=store.planning_history(&q.work).map_err(|e|root("planning-history-read",e))?;serde_json::to_vec(&value).map(RouteBody::plain).map_err(E::ResponseJson)})();
        close(result,store.close())
    }

    /// Handle the existing planning endpoint behind shared Host/session/CSRF,
    /// transport policy and Root lane. GET/HEAD return actual Work only for show;
    /// POST forwards bounded bytes to Root's strict PlanningRequest decoder and
    /// saved revision/replay/policy/physical-owner checks. No default Work,
    /// synthetic claim, model call, identity grant or new execution policy.
    /// Recheck retained datastore binding before business access. Preserve cause,
    /// committed Work metadata and close failures, even if response encoding fails.
    pub(super) fn planning(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        let read = matches!(*request.method(), Method::GET | Method::HEAD);
        if !read && request.method() != Method::POST {
            return Err(E::Method("GET, HEAD, POST"));
        }
        let query = query(request.uri().query())?;
        let operation = if read {
            if query.action.as_deref().is_some_and(|v| v != "show") {
                return Err(E::Refused("web-operation-unported"));
            }
            if !request.body().is_empty() {
                return Err(E::Refused("planning-read-body-invalid"));
            }
            None
        } else {
            let kind = kind(query.action.as_deref())?;
            swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
            Some(kind)
        };
        let mut store = RootStore::open(&self.root, !read).map_err(|e| root("open", e))?;
        let mut recorded = None;
        let result = (|| {
            self.check_binding(&store)?;
            let work = if let Some(kind) = operation {
                let outcome = store
                    .execute_external(&query.work, kind, request.body(), &mut SystemEnvironment)
                    .map_err(|e| root("mutate", e))?;
                let work = outcome.result.work;
                recorded = Some((work.id.clone(), work.revision));
                work
            } else {
                store.read_work(&query.work).map_err(|e| root("read", e))?
            };
            let bytes = swarm_engine::encode_model(&work).map_err(E::Model)?;
            Ok(RouteBody { auxiliary_effects:Vec::new(), provider_probe:None, preparation_receipt:None, trust_publication:None, observation_causes: Vec::new(),
                bytes,
                recorded_work: recorded.clone(),
                configuration_recorded: false,
                recorded_configuration: None,
            })
        })();
        match (close(result, store.close()), recorded) {
            (Err(primary), Some((work, revision))) => Err(E::Recorded {
                primary: Box::new(primary),
                work,
                revision,
            }),
            (result, _) => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Valid actual URL shape precedes ambiguity, missing and traversal cases.
    #[test]
    fn strict_query_has_no_mission_or_action_fallback() {
        assert_eq!(
            query(Some("work=w%2Done&action=judge")).unwrap(),
            Query {
                work: "w-one".into(),
                action: Some("judge".into())
            }
        );
        assert_eq!(query(Some("work=w-one")).unwrap().action, None);
        for value in [
            None,
            Some(""),
            Some("work="),
            Some("action=judge"),
            Some("work=w&work=x"),
            Some("work=w&action=judge&action=renew"),
            Some("work=w&actor=engine"),
            Some("work=%2E%2E%2Fx"),
            Some("work=w&action="),
        ] {
            assert_eq!(query(value).unwrap_err().code(), "planning-query-invalid");
        }
    }
    /// Every supported choice is an exact protocol mapping; unsupported reads
    /// and process/managed operations have no implicit execution or show fallback.
    #[test]
    fn supported_control_kinds_are_explicit() {
        assert_eq!(capabilities()[0], "show");
        for action in capabilities().into_iter().skip(1) {
            assert!(kind(Some(action)).is_ok());
        }
        for action in [
            "enable",
            "claim",
            "lease-renew",
            "decide",
            "judge",
            "renew",
            "pause",
            "resume",
        ] {
            assert_eq!(kind(Some(action)).unwrap(), format!("planning.{action}"));
        }
        for action in [
            None,
            Some("show"),
            Some("step"),
            Some("run"),
            Some("scope-patch"),
            Some("task.update"),
            Some("planning.judge"),
        ] {
            assert_eq!(kind(action).unwrap_err().code(), "web-operation-unported");
        }
    }
}
