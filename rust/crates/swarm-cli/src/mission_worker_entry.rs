//! Private Linux session bootstrap for the EXISTING native `agent run` route.
//! This does not start a conductor, prepare an Agent or grant provider execution.
use super::{agent, Arguments, CliError};
use std::{io::Write, path::Path};

/// Private protocol operation, deliberately absent from public help. Its name
/// carries no capability: the normal Agent claim/gate remain authoritative.
pub(super) const OPERATION: &str = "_mission_worker_entry";

/// Validate only this bootstrap's transport before any syscall or Root open,
/// then normalize to the actual public Agent consumer. Preserve the Agent ID
/// exactly; its canonical validity and existence belong to that consumer.
/// Absolute Root is an internal caller precondition preventing an autonomous
/// child from silently selecting the default cwd Root. It proves no ownership.
/// NUL is impossible in OS argv, not a new identity or filesystem policy.
fn agent_arguments(a: &Arguments) -> Result<Arguments, CliError> {
    if a.pos.len() != 2
        || a.pos[0] != OPERATION
        || !a.json
        || a.help
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
        || !Path::new(&a.root).is_absolute()
        || a.root.as_bytes().contains(&0)
        || a.pos[1].as_bytes().contains(&0)
    {
        return Err(CliError::Usage);
    }
    Ok(Arguments {
        pos: vec!["agent".into(), "run".into(), a.pos[1].clone()],
        input: String::new(),
        root: a.root.clone(),
        output: String::new(),
        task: String::new(),
        phase: a.phase.clone(),
        server: false,
        json: true,
        help: false,
        language: a.language.clone(),
    })
}

/// Preserve session phase/observed SID with the original typed CLI cause. A
/// successful setsid is not rolled back if later Root/run/close/output fails.
/// No PID is signalled and no absence of durable Agent effects is inferred.
fn failure(primary: CliError, session_id: Option<i32>, a: &Arguments) -> CliError {
    CliError::MissionWorkerEntry {
        primary: Box::new(primary),
        session_id,
        english: a.language == "en",
    }
}

/// Sequence pure validation, one session syscall and the real Agent consumer.
/// Injected closures are private seams for pure refusal/error tests; production
/// fixes them below. No retry/fallback/session grant or synthetic Agent is used.
fn execute_with(
    a: &Arguments,
    out: &mut dyn Write,
    enter_session: impl FnOnce() -> Result<rustix::process::Pid, rustix::io::Errno>,
    run_agent: impl FnOnce(&Arguments, &mut dyn Write) -> Result<u8, CliError>,
) -> Result<u8, CliError> {
    let normalized = agent_arguments(a)?;
    let session = enter_session().map_err(|cause| {
        failure(
            CliError::Root(Box::new(swarm_engine::RootStoreError::Runtime(
                swarm_engine::RuntimeError::new(swarm_engine::RuntimeOperation::Platform, cause),
            ))),
            None,
            a,
        )
    })?;
    run_agent(&normalized, out)
        .map_err(|primary| failure(primary, Some(session.as_raw_nonzero().get()), a))
}

/// Enter a new session in this already exec'd CLI, then reuse `agent::execute`
/// with exact `agent run ID` arguments. No unsafe pre_exec or shell is involved.
/// The launcher MUST use ordinary spawn inheriting its PGID: process_group(0)
/// would make this child a group leader and setsid would refuse with EPERM.
/// Call only in the isolated child, before threads/Root work; never in a host
/// test runner. Existing Root/Agent ownership and private gate check every grant.
/// This owns no loop, lease, child-provider implementation or shutdown policy.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    execute_with(a, out, rustix::process::setsid, agent::execute)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, error::Error, io};

    /// Parse the same argv format as the CLI without opening a Root or syscall.
    fn arguments() -> Result<Arguments, CliError> {
        crate::parse(
            &[
                OPERATION.into(),
                "agent-original".into(),
                "--root".into(),
                "/owned-fixture".into(),
                "--json".into(),
            ],
            "en",
        )
    }

    /// Valid transport normalizes only the operation; ID bytes remain those of
    /// the real Agent route. This is not a positive session/runtime execution.
    #[test]
    fn bootstrap_normalizes_without_inventing_identity_or_authority() -> Result<(), CliError> {
        let mut input = arguments()?;
        input.pos[1] = " supplied legacy id ".into();
        let normalized = agent_arguments(&input)?;
        assert_eq!(normalized.pos, ["agent", "run", " supplied legacy id "]);
        assert_eq!(normalized.root, input.root);
        assert_eq!(normalized.language, "en");
        assert!(normalized.json);
        assert!(!normalized.help);
        Ok(())
    }

    /// Every malformed transport must refuse before either injected syscall or
    /// Agent consumer. No setsid or Root is executed by this test.
    #[test]
    fn bootstrap_refuses_invalid_transport_before_session_or_root() -> Result<(), CliError> {
        let mutations: Vec<fn(&mut Arguments)> = vec![
            |a| a.pos.clear(),
            |a| {
                a.pos.pop();
            },
            |a| a.pos.push("extra".into()),
            |a| a.pos[0] = "_agent_gate".into(),
            |a| a.json = false,
            |a| a.help = true,
            |a| a.input = "input.json".into(),
            |a| a.output = "output.json".into(),
            |a| a.server = true,
            |a| a.task = "task".into(),
            |a| a.phase = "entry".into(),
            |a| a.root = ".".into(),
            |a| a.root = String::new(),
            |a| a.root.push('\0'),
            |a| a.pos[1].push('\0'),
        ];
        for mutation in mutations {
            let mut input = arguments()?;
            mutation(&mut input);
            let session_called = Cell::new(false);
            let runtime_called = Cell::new(false);
            let result = execute_with(
                &input,
                &mut Vec::new(),
                || {
                    session_called.set(true);
                    Err(rustix::io::Errno::PERM)
                },
                |_, _| {
                    runtime_called.set(true);
                    Ok(0)
                },
            );
            assert!(matches!(result, Err(CliError::Usage)));
            assert!(!session_called.get());
            assert!(!runtime_called.get());
        }
        Ok(())
    }

    /// A failed session syscall is retained as typed errno and never calls the
    /// Agent path. The injected EPERM is not a real process-session observation.
    #[test]
    fn bootstrap_session_refusal_preserves_errno_without_runtime() -> Result<(), CliError> {
        let runtime_called = Cell::new(false);
        let result = execute_with(
            &arguments()?,
            &mut Vec::new(),
            || Err(rustix::io::Errno::PERM),
            |_, _| {
                runtime_called.set(true);
                Ok(0)
            },
        );
        let error = match result {
            Err(error) => error,
            Ok(_) => return Err(CliError::Usage),
        };
        assert_eq!(error.code(), "mission-worker-session-unavailable");
        assert!(matches!(
            error,
            CliError::MissionWorkerEntry {
                session_id: None,
                ..
            }
        ));
        assert!(!runtime_called.get());
        assert!(error.typed_causes().iter().any(
            |cause| cause.downcast_ref::<rustix::io::Errno>() == Some(&rustix::io::Errno::PERM)
        ));
        assert!(error.source().is_some());
        Ok(())
    }

    /// The real CLI dispatcher must not return successful public help for a
    /// malformed private entry. Its help flag refuses before the real syscall;
    /// this is intentionally an invalid-only in-process route test.
    #[test]
    fn bootstrap_dispatch_rejects_help_before_public_help() -> Result<(), CliError> {
        let mut input = arguments()?;
        input.help = true;
        let mut output = Vec::new();
        assert!(matches!(
            crate::execute(&input, &mut output),
            Err(CliError::Usage)
        ));
        assert!(output.is_empty());
        Ok(())
    }

    /// A mocked successful session followed by a known lifecycle/output failure
    /// keeps the observed SID and both CLI error branches, without executing
    /// setsid, opening Root or claiming that this fixture recorded a real Work.
    #[test]
    fn bootstrap_after_session_error_keeps_state_cleanup_and_diagnostics() -> Result<(), CliError> {
        let session = rustix::process::Pid::from_raw(4321).ok_or(CliError::Usage)?;
        let result = execute_with(
            &arguments()?,
            &mut Vec::new(),
            || Ok(session),
            |a, _| {
                assert_eq!(a.pos, ["agent", "run", "agent-original"]);
                Err(CliError::StatePreserved {
                    primary: Box::new(CliError::RootCleanup {
                        primary: Box::new(CliError::Output(io::Error::from_raw_os_error(32))),
                        cleanup: Box::new(swarm_engine::RootStoreError::Refused("fixture-close")),
                    }),
                    work: "fixture-work".into(),
                    revision: 7,
                    english: true,
                })
            },
        );
        let error = match result {
            Err(error) => error,
            Ok(_) => return Err(CliError::Usage),
        };
        assert!(matches!(
            error,
            CliError::MissionWorkerEntry {
                session_id: Some(4321),
                ..
            }
        ));
        assert_eq!(error.recorded_work(), Some(("fixture-work".into(), 7)));
        let labels = crate::safe_causes(&error);
        assert!(labels.iter().any(|label| label == "fixture-close"));
        assert!(labels.iter().any(|label| label == "os-errno:32"));
        assert!(error.message().contains("4321"));
        assert!(error.message().contains("does not prove worker entry"));
        Ok(())
    }
}
