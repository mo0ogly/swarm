//! Explicit task model choice; recorded data is neither identity nor launch permission.
use crate::{AgentLaunchError as E, Launch, TaskModel, Work, WorkEnvironment};
use serde::{Deserialize, Serialize};
use std::path::Path;
/// Inherited Go task-model request. Missing scalar fields have Go zero values,
/// then semantic validation applies; inherit is an explicit return to task defaults.
/// No actor/time, arbitrary route or provider availability assertion is accepted.
#[derive(Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TaskModelRequest {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub task_id: String,
    pub provider: String,
    pub level: String,
    pub model_policy_hash: String,
    pub inherit: bool,
}
/// Strict command boundary with inherited Go null/case/duplicate merge behavior.
/// Unknown/type/JSON fields and invalid identifiers/revision/selection refuse
/// with safe codes and retained causes.
/// No IO, defaults adoption, permission or execution occurs.
pub fn decode_task_model_request(raw: &[u8]) -> Result<TaskModelRequest, E> {
    let r: TaskModelRequest = crate::work::decode_mode(raw, request_shape(), true)
        .map_err(|e| E::caused("task-model-request-invalid", e))?;
    if r.schema_version != 1
        || r.expected_revision < 1
        || [&r.event_id, &r.task_id]
            .iter()
            .any(|v| !swarm_identity_contract::name(v.as_bytes()))
        || (!r.inherit
            && (!swarm_identity_contract::name(r.provider.as_bytes())
                || !matches!(
                    r.level.as_str(),
                    "" | "auto" | "simple" | "standard" | "exigeant"
                )
                || !crate::provider_configuration::configuration_digest_valid(
                    &r.model_policy_hash,
                )))
    {
        return Err(E::Refused("task-model-request-invalid"));
    }
    Ok(r)
}
/// Inherited TaskModelRequest wire fields; operator choice remains data. The
/// common projector preserves Go null/scalar/case/duplicate semantics while
/// refusing unknown fields and invalid types; this creates no new defaults.
fn request_shape() -> crate::model_wire::Fields {
    use crate::model_wire::Kind;
    &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("task_id", Kind::String),
        ("provider", Kind::String),
        ("level", Kind::String),
        ("model_policy_hash", Kind::String),
        ("inherit", Kind::Bool),
    ]
}

/// Read-only proposed choice. The local actor/time are observations, not
/// authenticated identity; no provider process or reservation is created.
#[derive(Serialize)]
pub struct TaskModelPreview {
    pub before: Option<Box<TaskModel>>,
    pub after: Option<Box<TaskModel>>,
    pub revision: i64,
    pub scope: &'static str,
}
/// Unique task index: duplicate persisted identities never select an arbitrary row.
fn index(w: &Work, task: &str) -> Result<usize, E> {
    let mut ids = w
        .tasks
        .iter()
        .flatten()
        .enumerate()
        .filter(|(_, t)| t.id == task)
        .map(|(i, _)| i);
    let i = ids.next().ok_or(E::Refused("task-model-task-missing"))?;
    if ids.next().is_some() {
        return Err(E::Refused("task-model-task-duplicate"));
    }
    Ok(i)
}
/// Require no active native attempt or running task/review. SQL failures retain
/// their causes; no state is changed or active attempt silently replaced.
fn idle(db: &rusqlite::Connection, w: &Work, i: usize) -> Result<(), E> {
    let t = &w
        .tasks
        .as_ref()
        .ok_or(E::Refused("task-model-task-missing"))?[i];
    let mut statement=db.prepare("SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')").map_err(|e|E::caused("task-model-storage",e))?;
    let result = statement
        .query_row([&w.id, &t.id], |row| row.get::<_, i64>(0))
        .map_err(|e| E::caused("task-model-storage", e));
    let count = crate::agent_launch_error::finish(
        result,
        statement
            .finalize()
            .map_err(|e| E::caused("task-model-statement-close", e)),
    )?;
    if count != 0
        || t.status == "running"
        || t.independent_review
            .as_ref()
            .is_some_and(|r| r.state == "running")
    {
        return Err(E::Refused("task-model-active-attempt"));
    }
    Ok(())
}
/// Resolve through the same current provider/rules/catalogue adapter as admission.
/// Reads under caller's settings reservation; closes its checked descriptor with
/// joined causes. No executable, model identity or account availability is proved.
#[cfg(target_os = "linux")]
fn selection(
    root: &Path,
    r: &TaskModelRequest,
    settings: &crate::EngineSettingsDocument,
) -> Result<crate::ModelRoute, E> {
    let directory = crate::local_storage::LocalDirectory::open(&root.join(".swarm"))
        .map_err(|e| E::caused("task-model-root-open", e))?;
    let result = (|| {
        let policy = settings
            .values
            .agent_launch_policy()
            .map_err(|e| E::caused(e.reason(), e))?;
        let launch = Launch {
            provider: r.provider.clone(),
            level: r.level.clone(),
            model_policy_hash: r.model_policy_hash.clone(),
            ..Default::default()
        };
        let (provider, _) = crate::agent_launch::provider(&directory, &launch, &policy)?;
        let model =
            crate::provider_model_route::resolve(&provider, &launch, &settings.values, &policy)?;
        model
            .route()
            .cloned()
            .ok_or(E::Refused("task-model-adapter-unavailable"))
    })();
    crate::agent_launch_error::finish(
        result,
        directory
            .close()
            .map_err(|e| E::caused("task-model-root-close", e)),
    )
}
/// Stage the choice on a private candidate; caller owns transaction/rollback and
/// replay. Validates idle state and current sources; inherit clears only choice.
/// Actor/time come from the engine environment and are not authentication. No
/// attempts, provider files, consumption or reservation are rewritten.
pub(crate) fn apply<Ev: WorkEnvironment>(
    db: &rusqlite::Connection,
    root: &Path,
    w: &mut Work,
    r: &TaskModelRequest,
    settings: &crate::EngineSettingsDocument,
    env: &mut Ev,
) -> Result<(), E> {
    let i = index(w, &r.task_id)?;
    idle(db, w, i)?;
    let selected = if r.inherit {
        None
    } else {
        Some(Box::new(TaskModel {
            provider: r.provider.clone(),
            route: selection(root, r, settings)?,
            actor: env
                .operator()
                .map_err(|e| E::caused("task-model-operator", e))?,
            at: env
                .timestamp()
                .map_err(|e| E::caused("task-model-time", e))?,
        }))
    };
    w.tasks
        .as_mut()
        .ok_or(E::Refused("task-model-task-missing"))?[i]
        .model_selection = selected;
    Ok(())
}
/// Read/clone actual Work at the requested revision, then compute a proposal.
/// No SQL writes occur; source changes before apply are independently rechecked.
/// This observation is not a freshness lease or actor authorization.
pub(crate) fn preview<Ev: WorkEnvironment>(
    db: &rusqlite::Connection,
    root: &Path,
    work: &str,
    r: &TaskModelRequest,
    settings: &crate::EngineSettingsDocument,
    env: &mut Ev,
) -> Result<TaskModelPreview, E> {
    let mut w =
        crate::read_recorded_work(db, work).map_err(|e| E::caused("task-model-work-invalid", e))?;
    if w.revision != r.expected_revision {
        return Err(E::Refused("revision_conflict"));
    }
    let i = index(&w, &r.task_id)?;
    let before = w
        .tasks
        .as_ref()
        .ok_or(E::Refused("task-model-task-missing"))?[i]
        .model_selection
        .clone();
    apply(db, root, &mut w, r, settings, env)?;
    Ok(TaskModelPreview {
        before,
        after: w
            .tasks
            .as_ref()
            .ok_or(E::Refused("task-model-task-missing"))?[i]
            .model_selection
            .clone(),
        revision: w.revision,
        scope: "local_selection_only_no_launch_permission",
    })
}
/// Normalize the requested launch against the actual task choice. Provider and
/// explicit level conflicts refuse; selected policy hash always overrides client
/// omission and is checked against current provider policy by the common resolver.
/// Task metadata alone authenticates neither actor nor provider/model identity.
pub(crate) fn bind_launch(w: &Work, r: &mut Launch) -> Result<(), E> {
    let i = index(w, &r.task_id)?;
    if let Some(s) = &w
        .tasks
        .as_ref()
        .ok_or(E::Refused("task-model-task-missing"))?[i]
        .model_selection
    {
        if r.provider != s.provider
            || (!matches!(r.level.as_str(), "" | "auto") && r.level != s.route.level)
            || !crate::model_routing_policy::model_level(&s.route.level)
            || !crate::provider_configuration::configuration_digest_valid(&s.route.policy_hash)
            || (!r.model_policy_hash.is_empty() && r.model_policy_hash != s.route.policy_hash)
        {
            return Err(E::Refused("task-model-launch-mismatch"));
        }
        r.level = s.route.level.clone();
        r.model_policy_hash = s.route.policy_hash.clone();
    }
    Ok(())
}
/// Non-Linux native provider resolution remains explicitly unavailable; no
/// platform fallback or fabricated selection is returned.
#[cfg(not(target_os = "linux"))]
fn selection(
    _root: &Path,
    _r: &TaskModelRequest,
    _settings: &crate::EngineSettingsDocument,
) -> Result<crate::ModelRoute, E> {
    Err(E::Refused("task-model-platform-unported"))
}
/// Compare the retained choice with a freshly resolved actual candidate. Reason
/// text is explanatory; level/model/effort/billing/policy must agree. No trust in
/// a serialized selected route can replace authoritative provider resolution.
pub(crate) fn check_resolved(
    w: &Work,
    task: &str,
    route: Option<&crate::ModelRoute>,
) -> Result<(), E> {
    let i = index(w, task)?;
    if let Some(s) = &w
        .tasks
        .as_ref()
        .ok_or(E::Refused("task-model-task-missing"))?[i]
        .model_selection
    {
        let r = route.ok_or(E::Refused("task-model-route-mismatch"))?;
        let old = &s.route;
        if (
            old.level.as_str(),
            old.model.as_str(),
            old.effort.as_str(),
            old.billing.as_str(),
            old.policy_hash.as_str(),
        ) != (
            r.level.as_str(),
            r.model.as_str(),
            r.effort.as_str(),
            r.billing.as_str(),
            r.policy_hash.as_str(),
        ) {
            return Err(E::Refused("task-model-route-mismatch"));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Valid controls precede malformed requests; no invented default selection.
    #[test]
    fn task_model_request_strict_and_explicit() {
        let raw=br#"{"schema_version":1,"event_id":"select","expected_revision":3,"task_id":"task","inherit":true}"#;
        assert!(decode_task_model_request(raw).unwrap().inherit);
        for compatible in [br#"{"schema_version":1,"event_id":"inherit","expected_revision":3,"task_id":"task","inherit":true,"provider":null,"level":null,"model_policy_hash":null}"#.as_slice(),br#"{"SCHEMA_VERSION":1,"event_id":"first","EVENT_ID":"second","expected_revision":3,"task_id":"task","INHERIT":true}"#]{let r=decode_task_model_request(compatible).unwrap();assert!(r.inherit);assert_eq!(r.provider,"");assert!(matches!(r.event_id.as_str(),"inherit"|"second"));}
        for bad in [br#"{}"#.as_slice(),br#"{"schema_version":1,"event_id":"select","expected_revision":3,"task_id":"task","inherit":"true"}"#,br#"{"schema_version":1,"event_id":"select","expected_revision":3,"task_id":"task","inherit":true,"actor":"forged"}"#,br#"{"schema_version":1,"event_id":"select","expected_revision":3,"task_id":"task","provider":"fixture","level":"standard","model_policy_hash":"bad"}"#]{assert!(decode_task_model_request(bad).is_err());}
    }
    /// A retained choice binds level/hash without altering other launch fields.
    #[test]
    fn task_model_launch_choice_is_not_client_override() {
        let mut w = Work {
            tasks: Some(vec![crate::Task {
                id: "task".into(),
                model_selection: Some(Box::new(TaskModel {
                    provider: "fixture".into(),
                    route: crate::ModelRoute {
                        level: "standard".into(),
                        policy_hash: "a".repeat(64),
                        ..Default::default()
                    },
                    ..Default::default()
                })),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let good = Launch {
            task_id: "task".into(),
            provider: "fixture".into(),
            level: "auto".into(),
            ..Default::default()
        };
        let mut r = good.clone();
        bind_launch(&w, &mut r).unwrap();
        assert_eq!(r.level, "standard");
        assert_eq!(r.model_policy_hash, "a".repeat(64));
        for bad in [
            Launch {
                provider: "other".into(),
                ..good.clone()
            },
            Launch {
                level: "exigeant".into(),
                ..good.clone()
            },
            Launch {
                model_policy_hash: "b".repeat(64),
                ..good.clone()
            },
        ] {
            assert!(bind_launch(&w, &mut bad.clone()).is_err());
        }
        let chosen = w.tasks.as_ref().unwrap()[0]
            .model_selection
            .as_ref()
            .unwrap()
            .route
            .clone();
        check_resolved(&w, "task", Some(&chosen)).unwrap();
        let mut explain = chosen.clone();
        explain.reason = "explanation only".into();
        check_resolved(&w, "task", Some(&explain)).unwrap();
        for route in [
            crate::ModelRoute {
                model: "other".into(),
                ..chosen.clone()
            },
            crate::ModelRoute {
                effort: "other".into(),
                ..chosen.clone()
            },
            crate::ModelRoute {
                billing: "other".into(),
                ..chosen.clone()
            },
            crate::ModelRoute {
                level: "exigeant".into(),
                ..chosen.clone()
            },
            crate::ModelRoute {
                policy_hash: "b".repeat(64),
                ..chosen.clone()
            },
        ] {
            assert!(check_resolved(&w, "task", Some(&route)).is_err());
        }
        assert!(check_resolved(&w, "task", None).is_err());
        let duplicate = w.tasks.as_ref().unwrap()[0].clone();
        w.tasks.as_mut().unwrap().push(duplicate);
        assert!(bind_launch(&w, &mut good.clone()).is_err());
    }
}
