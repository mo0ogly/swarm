//! Public historical status JSON for supported ordinary missions, and distinct
//! native conductor facts. Unsupported historical branches refuse explicitly;
//! the smaller native document is never substituted under the historical command.
use crate::{json_bytes, write_output, Arguments, CliError};
use serde::Serialize;
use std::io::Write;
use swarm_engine::{
    Agent, MissionControlView, MissionSupervisionObservation, MissionSupervisionState, RootStore,
    RootStoreError,
};

/// Selected stored registration fields for this trusted local CLI only. A PID
/// is an observation, not signal permission or proof of process/provider life.
/// Private prompts, environment, context, host and process stamps are excluded.
#[derive(Serialize)]
struct AgentRecord {
    id: String,
    task: String,
    status: String,
    desired: String,
    supervisor_pid: i64,
    child_pid: i64,
}
/// Existing native lease-clock classification and recorded times, never a grant.
/// Private source/error strings and conductor identity are deliberately omitted.
#[derive(Serialize)]
struct SupervisionRecord {
    state: &'static str,
    generation: Option<i64>,
    last_check_at: String,
    heartbeat_observed_at: String,
    last_check_failed: bool,
}
/// Version1 is a new local observation protocol, not the Go MissionStatus DTO.
/// Reads are sequential snapshots, so differing revisions remain visible. No
/// launch decision, liveness, acceptance or complete cockpit projection follows.
#[derive(Serialize)]
struct NativeStatus {
    schema_version: i64,
    work: String,
    revision: i64,
    controls: MissionControlView,
    supervision: Option<SupervisionRecord>,
    agents: Vec<AgentRecord>,
    limitations: Vec<&'static str>,
}

/// Preserve the native state vocabulary without mapping a present lease to a
/// live process or an active automatic mission. Unknown legacy stays unverified.
fn supervision_state(state: MissionSupervisionState) -> &'static str {
    match state {
        MissionSupervisionState::LegacyUnverified => "legacy_unverified",
        MissionSupervisionState::Stopped => "stopped",
        MissionSupervisionState::Present => "present",
        MissionSupervisionState::CycleError => "cycle_error",
        MissionSupervisionState::Stale => "stale",
        MissionSupervisionState::Future => "future",
    }
}

/// Project only actual reader outputs, preserving absence and retained fields.
/// This pure conversion performs no IO, inference, mutation or process action.
fn project(
    work: String,
    revision: i64,
    controls: MissionControlView,
    supervision: Option<MissionSupervisionObservation>,
    agents: Vec<Agent>,
) -> NativeStatus {
    NativeStatus {
        schema_version: 1,
        work,
        revision,
        controls,
        supervision: supervision.map(|value| SupervisionRecord {
            state: supervision_state(value.state),
            generation: value.generation,
            last_check_at: value.last_check_at,
            heartbeat_observed_at: value.observed_at,
            last_check_failed: !value.last_error.is_empty(),
        }),
        agents: agents
            .into_iter()
            .map(|agent| AgentRecord {
                id: agent.id,
                task: agent.task_id,
                status: agent.status,
                desired: agent.desired,
                supervisor_pid: agent.supervisor_pid,
                child_pid: agent.child_pid,
            })
            .collect(),
        limitations: vec![
            "native-status-is-not-historical-json",
            "sequential-native-read-observations",
            "stored-registration-is-not-process-liveness",
            "no-task-acceptance-derived",
            "read-only-no-conductor-action",
        ],
    }
}

/// Escape terminal controls in retained strings without changing the machine
/// document. Formatting private data never changes the underlying observation.
fn visible(raw: &str) -> String {
    raw.escape_debug().to_string()
}

/// Render selected facts in FR/EN, preserving missing controls as unknown.
/// No aggregate named active/enabled/accepted is synthesized. This is a human
/// reading surface; machine consumers must request status-native JSON explicitly.
fn render(status: &NativeStatus, english: bool) -> Vec<u8> {
    let unknown = if english { "unknown" } else { "inconnu" };
    let boolean = |value: Option<bool>| match value {
        Some(true) => {
            if english {
                "yes"
            } else {
                "oui"
            }
        }
        Some(false) => {
            if english {
                "no"
            } else {
                "non"
            }
        }
        None => unknown,
    };
    let mut text = format!(
        "Mission {} — {} {}\n{}: {} · {}: {} · {}: {} · {}: {}\n",
        visible(&status.work),
        if english { "revision" } else { "révision" },
        status.revision,
        if english {
            "Recorded consent"
        } else {
            "Consentement enregistré"
        },
        boolean(status.controls.consent_enabled),
        if english { "Autonomy" } else { "Autonomie" },
        status.controls.autonomy.unwrap_or(unknown),
        if english { "Paused" } else { "Pause" },
        boolean(status.controls.paused),
        if english { "Archived" } else { "Archivée" },
        boolean(Some(status.controls.archived)),
    );
    let slots = status
        .controls
        .slots
        .map(|n| n.to_string())
        .unwrap_or_else(|| unknown.into());
    text.push_str(&format!(
        "{}: {} · {}: {}\n",
        if english {
            "Recorded slots"
        } else {
            "Créneaux enregistrés"
        },
        slots,
        if english {
            "Controls revision"
        } else {
            "Révision des contrôles"
        },
        status.controls.revision,
    ));
    match &status.supervision {
        Some(supervision) => text.push_str(&format!(
            "{}: {} · {}: {} · {}: {}\n{}: {} · {}: {}\n",
            if english {
                "Supervision observation"
            } else {
                "Observation de supervision"
            },
            supervision.state,
            if english {
                "generation"
            } else {
                "génération"
            },
            supervision
                .generation
                .map(|n| n.to_string())
                .unwrap_or_else(|| unknown.into()),
            if english {
                "last check"
            } else {
                "dernière vérification"
            },
            visible(&supervision.last_check_at),
            if english {
                "Recorded heartbeat"
            } else {
                "Heartbeat enregistré"
            },
            visible(&supervision.heartbeat_observed_at),
            if english {
                "Recorded check error"
            } else {
                "Erreur de vérification enregistrée"
            },
            boolean(Some(supervision.last_check_failed)),
        )),
        None => text.push_str(if english {
            "Supervision: no stored record.\n"
        } else {
            "Supervision : aucun enregistrement.\n"
        }),
    }
    text.push_str(&format!(
        "{}: {}\n",
        if english {
            "Agent records"
        } else {
            "Enregistrements d’agents"
        },
        status.agents.len(),
    ));
    for agent in &status.agents {
        text.push_str(&format!(
            "- {} · {} {} · {} · desired={} · supervisor_pid={} · child_pid={}\n",
            visible(&agent.id),
            if english { "task" } else { "tâche" },
            visible(&agent.task),
            visible(&agent.status),
            visible(&agent.desired),
            agent.supervisor_pid,
            agent.child_pid,
        ));
    }
    text.push_str(if english {
        "Limits: sequential native observations; no process liveness, launch permission or task acceptance is established. Historical JSON supports ordinary unconfigured missions only.\n"
    } else {
        "Limites : observations natives séquentielles ; aucune vie de processus, autorisation de départ ou acceptation de tâche établie. Le JSON historique couvre les missions ordinaires sans configuration.\n"
    });
    text.into_bytes()
}

/// Close before publishing bytes and preserve read/encode plus close failures.
/// Output/flush failures propagate only after successful Root close. A read error
/// never publishes an empty status; no cleanup failure becomes a successful exit.
fn finish(
    result: Result<Vec<u8>, CliError>,
    close: Result<(), RootStoreError>,
    out: &mut dyn Write,
) -> Result<u8, CliError> {
    let root = |cause| CliError::Root(Box::new(cause));
    match (result, close) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cause)) => Err(root(cause)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}

/// Keep typed semantic observation causes alive through the final output flush.
/// A late failure keeps each cause as a secondary branch; the existing cleanup
/// envelope represents combined errors and does not assert that IO cleanup ran.
/// Success returns its original exit code and never exposes raw diagnostic data.
pub(super) fn retain_observation_causes(
    result: Result<u8, CliError>,
    causes: Vec<RootStoreError>,
) -> Result<u8, CliError> {
    result.map_err(|mut primary| {
        for cause in causes {
            primary = CliError::RootCleanup {
                primary: Box::new(primary),
                cleanup: Box::new(cause),
            };
        }
        primary
    })
}

/// Read historical status/changes/spending JSON, native status text, or the explicit
/// `status-native WORK --json` extension. Historical unsupported branches refuse
/// without substituting native facts. Operator identity comes from the local OS;
/// it selects a visit window and does not authenticate authority. Root is opened
/// read-only and closed before output; observation causes survive encode, close,
/// write and flush failures. No SQL mutation, provider, signal or lease claim occurs.
/// Changes/spending without --json render a localized heading then the complete
/// pretty JSON subdocument, preserving unknown measurements and stored text.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.pos[0] != "mission"
        || !matches!(a.pos[1].as_str(), "status" | "status-native" | "changes" | "spending")
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    if a.pos[1] == "status-native" && !a.json {
        return Err(CliError::Usage);
    }
    let root = |cause| CliError::Root(Box::new(cause));
    let mission = |cause| {
        root(RootStoreError::Mutation(
            swarm_engine::WorkEngineError::Mission(Box::new(cause)),
        ))
    };
    let store = RootStore::open(std::path::Path::new(&a.root), false).map_err(root)?;
    if (a.pos[1] == "status" && a.json) || matches!(a.pos[1].as_str(), "changes" | "spending") {
        let mut causes = Vec::new();
        let result = (|| {
            let operator = swarm_engine::operator_identity()
                .map_err(|cause| root(RootStoreError::Runtime(cause)))?;
            let mut observation = store.mission_status_historical(&a.pos[2], &operator)
                .map_err(root)?;
            causes.append(&mut observation.causes);
            match a.pos[1].as_str() {
                "changes" => insight_bytes(&observation.status.changes, a, "Depuis votre dernière visite", "Since your last visit"),
                "spending" => insight_bytes(&observation.status.spending, a, "Appels et dépenses observés", "Observed calls and spending"),
                _ => json_bytes(&observation.status).map_err(CliError::Output),
            }
        })();
        return retain_observation_causes(finish(result, store.close(), out), causes);
    }
    let result = (|| {
        let work = store.read_work(&a.pos[2]).map_err(root)?;
        let controls = store.mission_control_view(&a.pos[2]).map_err(mission)?;
        let now = swarm_engine::system_timestamp()
            .map_err(|cause| root(RootStoreError::Runtime(cause)))?;
        let supervision = store
            .read_mission_supervision(&a.pos[2], &now)
            .map_err(mission)?;
        let agents = store.list_agents(&a.pos[2]).map_err(root)?;
        let status = project(work.id, work.revision, controls, supervision, agents);
        if a.json {
            json_bytes(&status).map_err(CliError::Output)
        } else {
            Ok(render(&status, a.language == "en"))
        }
    })();
    finish(result, store.close(), out)
}

/// Serialize one historical subdocument without flattening absent/unknown fields.
/// Human output adds only a localized heading; JSON escaping prevents retained
/// terminal controls from executing. No IO, policy default or mutation occurs;
/// serialization failures propagate through the caller's close/cause handling.
fn insight_bytes<T: Serialize>(value: &T, a: &Arguments, french: &str, english: &str) -> Result<Vec<u8>, CliError> {
    let bytes = json_bytes(value).map_err(CliError::Output)?;
    if a.json { return Ok(bytes); }
    let mut text = if a.language == "en" { english } else { french }.as_bytes().to_vec();
    text.push(b'\n');
    text.extend_from_slice(&bytes);
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    /// Both new operations reject invalid transport before Root IO. Human
    /// serialization keeps nulls and terminal controls escaped in either locale.
    #[test]
    fn insight_transport_and_safe_rendering() -> Result<(), CliError> {
        for operation in ["changes", "spending"] {
            for language in ["fr", "en"] {
                let mut a = crate::parse(&["mission".into(), operation.into(), "owned-work".into(),
                    "--root".into(), "/missing-insight-fixture".into(), "--json".into()], language)?;
                let value = serde_json::json!({"unknown":null,"literal":"\u{1b}[31m"});
                let machine = insight_bytes(&value, &a, "Titre", "Title")?;
                assert_eq!(serde_json::from_slice::<serde_json::Value>(&machine).unwrap(), value);
                a.json = false;
                let text = insight_bytes(&value, &a, "Titre", "Title")?;
                assert!(!text.contains(&27));
                let rendered = String::from_utf8(text).unwrap();
                let (heading, body) = rendered.split_once('\n').unwrap();
                assert_eq!(heading, if language == "en" { "Title" } else { "Titre" });
                assert_eq!(serde_json::from_str::<serde_json::Value>(body).unwrap(), value);
                a.input = "must-refuse-before-open".into();
                let mut out = Vec::new();
                assert!(matches!(execute(&a, &mut out), Err(CliError::Usage)));
                assert!(out.is_empty());
            }
        }
        Ok(())
    }

    /// Actual absence remains null; this helper creates no Root or permission.
    fn controls() -> MissionControlView {
        MissionControlView {
            revision: 4,
            paused: Some(false),
            autonomy: Some("autonome"),
            slots: Some(1),
            consent_enabled: Some(true),
            archived: false,
        }
    }

    /// Only selected fields are serialized; private raw Agent data never leaks.
    #[test]
    fn native_status_keeps_absence_and_excludes_private_agent_fields() -> Result<(), CliError> {
        let mut agent = Agent::default();
        agent.id = "owned-agent".into();
        agent.task_id = "owned-task".into();
        agent.status = "queued".into();
        agent.prompt = "PRIVATE-PROMPT".into();
        agent.supervisor_identity = "PRIVATE-STAMP".into();
        agent.host = "PRIVATE-HOST".into();
        let status = project("owned-work".into(), 3, controls(), None, vec![agent]);
        let raw = json_bytes(&status).map_err(CliError::Output)?;
        let value: serde_json::Value = serde_json::from_slice(&raw)
            .map_err(|cause| CliError::Output(io::Error::other(cause)))?;
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["revision"], 3);
        assert_eq!(value["controls"]["revision"], 4);
        assert!(value["supervision"].is_null());
        assert_eq!(value["agents"][0]["status"], "queued");
        assert_eq!(value["agents"][0]["supervisor_pid"], 0);
        assert_eq!(value["agents"][0].as_object().map(|m| m.len()), Some(6));
        assert_eq!(value["limitations"].as_array().map(|v| v.len()), Some(5));
        assert!(!String::from_utf8_lossy(&raw).contains("PRIVATE"));
        Ok(())
    }

    /// Stable native names never translate presence into process liveness.
    #[test]
    fn native_status_supervision_vocabulary_and_text_limits_are_explicit() {
        for (state, name) in [
            (
                MissionSupervisionState::LegacyUnverified,
                "legacy_unverified",
            ),
            (MissionSupervisionState::Stopped, "stopped"),
            (MissionSupervisionState::Present, "present"),
            (MissionSupervisionState::CycleError, "cycle_error"),
            (MissionSupervisionState::Stale, "stale"),
            (MissionSupervisionState::Future, "future"),
        ] {
            assert_eq!(supervision_state(state), name);
        }
        let status = project("owned\nwork".into(), 3, controls(), None, vec![]);
        let english = String::from_utf8_lossy(&render(&status, true)).into_owned();
        let french = String::from_utf8_lossy(&render(&status, false)).into_owned();
        assert!(english.contains("owned\\nwork"));
        assert!(english.contains("Agent records: 0"));
        assert!(english.contains("no process liveness"));
        assert!(
            french.contains("Enregistrements d’agents : 0")
                || french.contains("Enregistrements d’agents: 0")
        );
        assert!(french.contains("missions ordinaires sans configuration"));
    }

    /// Historical JSON reaches the real Root reader; bad native transport is rejected.
    #[test]
    fn status_historical_uses_root_and_rejects_invalid_native_transport() -> Result<(), CliError> {
        let arguments = crate::parse(
            &[
                "mission".into(),
                "status".into(),
                "owned-work".into(),
                "--root".into(),
                "/missing-status-test-root".into(),
                "--json".into(),
            ],
            "en",
        )?;
        let mut output = Vec::new();
        assert!(matches!(
            execute(&arguments, &mut output),
            Err(CliError::Root(_))
        ));
        assert!(output.is_empty());
        let mut invalid = arguments;
        invalid.pos[1] = "status-native".into();
        invalid.json = false;
        assert!(matches!(
            execute(&invalid, &mut output),
            Err(CliError::Usage)
        ));
        invalid.json = true;
        invalid.input = "private-input".into();
        assert!(matches!(
            execute(&invalid, &mut output),
            Err(CliError::Usage)
        ));
        assert!(output.is_empty());
        Ok(())
    }

    /// A late output refusal keeps actual typed observation causes as secondary
    /// branches; success never becomes an error merely because diagnosis exists.
    #[test]
    fn historical_observation_causes_survive_output_failure() {
        let error = retain_observation_causes(
            Err(CliError::Output(io::Error::from_raw_os_error(28))),
            vec![RootStoreError::Refused("status-observation-witness")],
        );
        match error {
            Err(CliError::RootCleanup { primary, cleanup }) => {
                assert!(matches!(*primary, CliError::Output(ref cause) if cause.raw_os_error() == Some(28)));
                assert!(matches!(*cleanup, RootStoreError::Refused("status-observation-witness")));
            }
            _ => panic!("late failure lost original observation cause"),
        }
        assert!(matches!(retain_observation_causes(Ok(0), vec![RootStoreError::Refused("status-observation-witness")]), Ok(0)));
    }

    /// Primary/close failures remain distinct; no partial document is emitted.
    #[test]
    fn status_read_close_and_flush_failures_do_not_become_empty_success() {
        let mut output = Vec::new();
        let error = finish(
            Err(CliError::Root(Box::new(RootStoreError::Refused(
                "status-read-witness",
            )))),
            Err(RootStoreError::Refused("status-close-witness")),
            &mut output,
        );
        assert!(matches!(error, Err(CliError::RootCleanup { .. })));
        assert!(output.is_empty());
        assert!(matches!(
            finish(
                Ok(b"valid".to_vec()),
                Err(RootStoreError::Refused("status-close-witness")),
                &mut output
            ),
            Err(CliError::Root(_))
        ));
        assert!(output.is_empty());
        struct FlushRefusal;
        impl Write for FlushRefusal {
            /// Accept the fixture bytes without touching a real descriptor.
            fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
                Ok(raw.len())
            }
            /// Preserve a concrete flush cause after the valid output witness.
            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::from_raw_os_error(5))
            }
        }
        assert!(finish(Ok(b"valid".to_vec()), Ok(()), &mut output).is_ok());
        assert_eq!(output, b"valid");
        assert!(matches!(
            finish(Ok(b"valid".to_vec()), Ok(()), &mut FlushRefusal),
            Err(CliError::Output(_))
        ));
    }
}
