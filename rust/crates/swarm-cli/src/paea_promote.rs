//! Guarded two-phase CLI consumer of the engine's PAEA Git promote call.
//! The prepared call keeps both physical root witnesses alive inside this
//! process while the operator provisions its exact permission out of band;
//! the private stdin frame then supplies the protected declaration pin and
//! personal password. Operation is an explicit argument: Commit performs one
//! OLD-fenced reference transaction after a durable intent reservation
//! (recoverable through Reconcile), Reconcile is a pure journal read plus one
//! read-only current direct-ref observation that never publishes, reserves or
//! writes the journal; a missing effect row never proves an absent past
//! effect. No
//! auto-grant, UID separation or independent review exists here; the prepared
//! permission is provisioned out of band, never issued by this process.
use super::{personal_secret_frame, Arguments, CliError};
use std::{
    io::{self, IsTerminal, Read, Write},
    path::Path,
};
use swarm_engine::RootStore;
use swarm_enrollment_authority::{
    authenticated_control::Context as UseContext, authenticated_startup, paea_action::Operation,
};
use swarm_enrollment_state::Context;

/// Fixed private framing version shared with the Trust enrollment adapter. The
/// packet layout is `swarm-trust-source-v1\n<pin64>\n<password>`; the pin names
/// the protected declaration of this call's operator, never the Git journal pin
/// passed as a public argument. The password stays inside the Zeroizing packet
/// and is dropped before any result line is emitted.
const HEADER: &[u8] = b"swarm-trust-source-v1\n";

/// Project the first attributed promotion error through the existing CLI
/// branch matrix. No independent calls are merged and absence of an envelope
/// establishes nothing about effects. This performs no IO or retry.
pub(super) fn error_observation(error: &CliError) -> Option<serde_json::Value> {
    error.typed_causes().into_iter()
        .find_map(|cause| cause.downcast_ref::<swarm_engine::PaeaGitPromoteCallError>())
        .map(swarm_engine::PaeaGitPromoteCallError::public_effect_observation)
}

/// Find the first attributed business-effect state through the complete CLI
/// branch matrix, for text rendering even behind late cleanup wrappers. No
/// error text is parsed and no effect is inferred from a missing envelope.
pub(super) fn error_effect_status(error: &CliError) -> Option<&'static str> {
    error.typed_causes().into_iter()
        .find_map(|cause| cause.downcast_ref::<swarm_engine::PaeaGitPromoteCallError>())
        .map(swarm_engine::PaeaGitPromoteCallError::effect_status)
}

/// Parse the explicit promotion operation argument. Reconciliation never
/// grants a new commitment; both spellings are stable protocol strings.
fn operation(spelling: &str) -> Option<Operation> {
    match spelling {
        "commit" => Some(Operation::Commit),
        "reconcile" => Some(Operation::Reconcile),
        _ => None,
    }
}

/// Dispatch the guarded `paea promote-call OPERATION SERVICE PROJECT WORK CHILD
/// PIN DECLARATION --input REQUESTFILE` command. Both roots are opened writable
/// (the separated-root check is contractual and runs at preparation), the
/// request file is a strict `{proposal, context}` document, DECLARATION is the
/// operator's provisioned protected source (validated only at startup, after
/// the prepared event), and stdin must not be a terminal: the private frame
/// arrives after the prepared event, mirroring the Trust enrollment handshake.
/// Every refusal closes all acquired owners and retains their close causes;
/// observation proves neither acceptance nor authority and grants no permission.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let operation = if a.pos.len() == 9 {
        operation(&a.pos[2])
    } else {
        None
    };
    if operation.is_none()
        || a.input.is_empty()
        || a.input == "-"
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
        || io::stdin().is_terminal()
    {
        return Err(CliError::Usage);
    }
    let raw = swarm_engine::read_command_file(Path::new(&a.input))
        .map_err(CliError::CommandInput)?;
    run(a, out, &mut io::stdin().lock(), operation.unwrap(), &raw)
}

/// Keep the owned prepared call alive through external permission provisioning,
/// then consume the one-shot guarded call under actual held authority. Every
/// early refusal closes the acquired roots and, once prepared, the call
/// itself; all close failures stay typed beside the primary cause, never
/// dropped and never invented into a separate success.
fn run(
    a: &Arguments,
    out: &mut dyn Write,
    input: &mut dyn Read,
    operation: Operation,
    raw: &[u8],
) -> Result<u8, CliError> {
    let root = |e| CliError::Root(Box::new(e));
    let project = RootStore::open(Path::new(&a.pos[4]), true).map_err(root)?;
    let service = match RootStore::open(Path::new(&a.pos[3]), true) {
        Ok(service) => service,
        Err(cause) => {
            // The project root is already opened: its close outcome joins the
            // primary failure whether it succeeds or refuses (never dropped).
            return Err(failed(
                Ok(()),
                project.close(),
                CliError::Root(Box::new(cause)),
                Vec::new(),
            ));
        }
    };
    let prepare = (|| {
        let document = swarm_protection_contract::decode_document(raw)
            .map_err(|e| CliError::Protection(e))?;
        let prepared = service
            .paea_git_promote_call(
                &project,
                &document.proposal,
                &document.context,
                operation,
                &a.pos[5],
                &a.pos[6],
                &a.pos[7],
            )
            .map_err(root)?;
        let request = prepared.request();
        // Public preparation event: the operator provisions exactly this
        // permission out of band before answering on private stdin. No
        // authority is established and this process must stay alive; the
        // prepared witnesses are memory-only and restart-destroyed.
        let event = serde_json::json!({
            "schema_version":1,
            "event":"paea-promote-prepared",
            "service":"repository.promote",
            "operation":operation.as_str(),
            "root_digest":request.root_digest(),
            "action_digest":request.action_digest(),
            "permission":request.permission(),
            "authority":false,
            "restart_reusable":false
        });
        line(out, &event).map_err(root)?;
        Ok(prepared)
    })();
    let mut closes: Vec<swarm_engine::RootStoreError> = Vec::new();
    let prepared = match prepare {
        Ok(prepared) => prepared,
        Err(primary) => {
            return Err(failed(service.close(), project.close(), primary, closes));
        }
    };
    let packet = match personal_secret_frame::read_packet(input) {
        Ok(packet) => packet,
        Err(_) => {
            closes.extend(prepared.close().err());
            return Err(failed(
                service.close(),
                project.close(),
                CliError::Root(Box::new(swarm_engine::RootStoreError::Refused(
                    "paea-promote-call-input",
                ))),
                closes,
            ));
        }
    };
    // The frame is HEADER + pin64 + '\n' + nonempty personal password: the
    // call always carries a password. A bare pin, wrong header or missing
    // separator refuses here, before any declaration is opened and before any
    // preparation is consumed.
    let cut = HEADER.len() + 64;
    let pin = if packet.len() > cut + 1
        && packet.starts_with(HEADER)
        && packet[cut] == b'\n'
    {
        std::str::from_utf8(&packet[HEADER.len()..cut])
            .ok()
            .filter(|pin| swarm_identity_contract::digest(pin.as_bytes()))
    } else {
        None
    };
    let pin = match pin {
        Some(pin) => pin,
        None => {
            closes.extend(prepared.close().err());
            return Err(failed(
                service.close(),
                project.close(),
                CliError::Root(Box::new(swarm_engine::RootStoreError::Refused(
                    "paea-promote-call-frame",
                ))),
                closes,
            ));
        }
    };
    // Independently provisioned protected declaration for this exact operator:
    // the CLI creates no account, adopts no remote source and mints no grant.
    let opened = authenticated_startup::open(&Context::new(), &a.pos[8], pin);
    let (app, head) = match opened {
        Ok(opened) => opened,
        Err(_) => {
            closes.extend(prepared.close().err());
            return Err(failed(
                service.close(),
                project.close(),
                CliError::Root(Box::new(swarm_engine::RootStoreError::Refused(
                    "paea-promote-call-startup",
                ))),
                closes,
            ));
        }
    };
    let result = service
        .call_paea_git_promote(
            &project,
            &prepared,
            &app,
            &UseContext::new(),
            &head,
            &packet[cut + 1..],
        )
        .map_err(|e| CliError::PaeaPromote(Box::new(e)));
    drop(packet);
    // The outcome escapes only as the final JSON line; every retained owner is
    // checked-closed on every outcome and each close failure stays retained.
    // Preserve the actual authority close envelope, including independent
    // personal/business causes. Closing cannot undo the observed Git effect.
    if let Err(cause) = app.close() {
        closes.push(retained_cause("paea-promote-call-authority-close", cause));
    }
    closes.extend(prepared.close().err());
    closes.extend(project.close().err());
    closes.extend(service.close().err());
    let result = match result {
        Ok(result) => result,
        Err(primary) => return Err(failed(Ok(()), Ok(()), primary, closes)),
    };
    // Recorded means a durable effect row exists for this exact canonical
    // intent (replayed=true performed no new publication). Reconciled is the
    // explicit reconciliation observation: the historical journal effect (if
    // any) plus the actual current direct Git ref; observed_oid is null only
    // when Git successfully listed no exact direct ref. Never an absence
    // claim, a negative result, an empty success or a grant.
    // FR : la réconciliation retourne l'effet journalisé historique et l'oid
    // Git direct actuel ; l'absence de ligne ne prouve jamais l'absence d'effet.
    let outcome = match &result {
        swarm_engine::PaeaGitPromoteOutcome::Recorded {
            intent_id,
            action_digest,
            target_ref,
            new_oid,
            replayed,
        } => serde_json::json!({
            "schema_version":1,
            "event":"paea-promote-result",
            "outcome":"recorded",
            "intent_id":intent_id,
            "action_digest":action_digest,
            "target_ref":target_ref,
            "new_oid":new_oid,
            "replayed":replayed
        }),
        swarm_engine::PaeaGitPromoteOutcome::Reconciled {
            intent_id,
            target_ref,
            observed_oid,
            recorded_effect,
        } => serde_json::json!({
            "schema_version":1,
            "event":"paea-promote-result",
            "outcome":"reconciled",
            "intent_id":intent_id,
            "target_ref":target_ref,
            "observed_oid":observed_oid,
            "recorded_effect":recorded_effect
        }),
    };
    let written = line(out, &outcome).map_err(|cause| {
        // A failed stdout write/newline/flush must not erase the effect
        // metadata: the known engine outcome is retained in the typed
        // observation envelope beside the fixed output refusal.
        CliError::PaeaPromote(Box::new(
            swarm_engine::PaeaGitPromoteCallError::Effect {
                outcome: Some(result.clone()),
                cause,
            },
        ))
    });
    match written {
        Ok(()) => match close_tail(closes, &result) {
            Some(error) => Err(error),
            None => Ok(0),
        },
        Err(primary) => Err(failed(Ok(()), Ok(()), primary, closes)),
    }
}

/// Retain an original typed cause behind a fixed safe diagnostic code.
/// This pure wrapper grants no retry or authority and never formats the cause.
fn retained_cause(
    code: &'static str,
    cause: impl std::error::Error + Send + Sync + 'static,
) -> swarm_engine::RootStoreError {
    swarm_engine::RootStoreError::Launch(Box::new(swarm_engine::AgentLaunchError::Cause {
        code, cause: Box::new(cause),
    }))
}

/// Encode before writing, then write the JSON, newline and flush separately.
/// Original serialization/IO causes survive under fixed stage codes. No bytes
/// are written on encoding failure; later errors may leave partial output and
/// never establish absence of the already-observed effect. Caller retains it.
fn line(out: &mut dyn Write, value: &impl serde::Serialize) -> Result<(), swarm_engine::RootStoreError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|cause| retained_cause("paea-promote-call-output-encode", cause))?;
    out.write_all(&bytes)
        .map_err(|cause| retained_cause("paea-promote-call-output-write", cause))?;
    out.write_all(b"\n")
        .map_err(|cause| retained_cause("paea-promote-call-output-newline", cause))?;
    out.flush()
        .map_err(|cause| retained_cause("paea-promote-call-output-flush", cause))
}

/// Build the refusal for close causes retained after a successful operation:
/// the first close becomes the typed primary and the rest stay chained
/// cleanups, so success with failed closes still exits nonzero and no cause
/// is dropped or invented into a separate success.
fn close_tail(closes: Vec<swarm_engine::RootStoreError>, outcome: &swarm_engine::PaeaGitPromoteOutcome) -> Option<CliError> {
    let mut closes = closes.into_iter();
    let first = closes.next()?;
    let mut tail = CliError::PaeaPromote(Box::new(
        swarm_engine::PaeaGitPromoteCallError::Effect {
            outcome: Some(outcome.clone()), cause: first,
        },
    ));
    for close in closes {
        tail = CliError::RootCleanup {
            primary: Box::new(tail),
            cleanup: Box::new(close),
        };
    }
    Some(tail)
}

/// Join a primary failure with two pending root closes, then every retained
/// close cause; a failed close never downgrades or hides the primary cause.
fn failed(
    service: Result<(), swarm_engine::RootStoreError>,
    project: Result<(), swarm_engine::RootStoreError>,
    primary: CliError,
    mut closes: Vec<swarm_engine::RootStoreError>,
) -> CliError {
    closes.extend(project.err());
    closes.extend(service.err());
    let mut tail = primary;
    for close in closes {
        tail = CliError::RootCleanup {
            primary: Box::new(tail),
            cleanup: Box::new(close),
        };
    }
    tail
}

#[cfg(test)]
mod output_cause_tests {
    use super::*;

    #[derive(Clone, Copy)]
    enum Stage { Write, Newline, Flush }

    /// Deterministic local output fault, with no Root, authority or Git effect.
    struct FailingWriter(Stage);
    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if matches!(self.0, Stage::Write)
                || matches!(self.0, Stage::Newline) && bytes == b"\n"
            {
                return Err(io::Error::from_raw_os_error(nix::errno::Errno::EPIPE as i32));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            if matches!(self.0, Stage::Flush) {
                Err(io::Error::from_raw_os_error(nix::errno::Errno::EPIPE as i32))
            } else { Ok(()) }
        }
    }

    /// A healthy output precedes each fault. Require the actual original errno
    /// through the CLI's production branch walker, not an error string.
    fn check_stage(stage: Stage) {
        let value = serde_json::json!({"state":"recorded"});
        let mut healthy = Vec::new();
        line(&mut healthy, &value).unwrap();
        assert_eq!(healthy, b"{\"state\":\"recorded\"}\n");
        let cause = line(&mut FailingWriter(stage), &value).expect_err("output must fail");
        let error = CliError::Root(Box::new(cause));
        assert!(error.typed_causes().iter().any(|cause| {
            cause.downcast_ref::<io::Error>().is_some_and(|io| {
                io.raw_os_error() == Some(nix::errno::Errno::EPIPE as i32)
            })
        }), "the output failure lost its original EPIPE cause");
    }

    #[test]
    fn write_retains_original_errno() { check_stage(Stage::Write); }
    #[test]
    fn newline_retains_original_errno() { check_stage(Stage::Newline); }
    #[test]
    fn flush_retains_original_errno() { check_stage(Stage::Flush); }
    /// The real close-error type has two independent branches. The CLI walker
    /// and the late-close consumer must retain both beside the known outcome.
    #[test]
    fn authority_close_retains_branches_and_known_effect() {
        let outcome = swarm_engine::PaeaGitPromoteOutcome::Recorded {
            intent_id: "intent".into(), action_digest: "digest".into(),
            target_ref: "refs/heads/release".into(), new_oid: "candidate".into(), replayed: false,
        };
        assert!(close_tail(Vec::new(), &outcome).is_none());
        let cause = swarm_enrollment_authority::authenticated_control::CloseError {
            personal: Some(swarm_enrollment_authority::personal::coordinator::Error::Interrupted),
            control: Some(swarm_enrollment_authority::control::Error::Binding),
        };
        let error = close_tail(vec![retained_cause("paea-promote-call-authority-close", cause)], &outcome)
            .expect("authority close failure must not become success");
        let causes = error.typed_causes();
        assert!(causes.iter().any(|e| e.downcast_ref::<swarm_enrollment_authority::authenticated_control::CloseError>().is_some()));
        assert!(causes.iter().any(|e| matches!(e.downcast_ref::<swarm_enrollment_authority::personal::coordinator::Error>(), Some(swarm_enrollment_authority::personal::coordinator::Error::Interrupted))));
        assert!(causes.iter().any(|e| matches!(e.downcast_ref::<swarm_enrollment_authority::control::Error>(), Some(swarm_enrollment_authority::control::Error::Binding))));
        let observed = causes.iter().find_map(|cause| swarm_engine::PaeaGitPromoteCallError::find(*cause))
            .expect("effect envelope retained");
        assert_eq!(observed.known_outcome(), Some(&outcome));
    }

    /// A serializer refusal keeps its original typed cause and writes nothing;
    /// private nested messages must not appear in ordinary safe CLI diagnostics.
    #[test]
    fn serialization_retains_typed_cause_before_output() {
        struct Invalid;
        impl serde::Serialize for Invalid {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("PRIVATE encode witness"))
            }
        }
        let mut out = Vec::new();
        let error = CliError::Root(Box::new(line(&mut out, &Invalid).expect_err("serializer refused")));
        assert!(out.is_empty());
        assert!(error.typed_causes().iter().any(|e| e.downcast_ref::<serde_json::Error>().is_some()));
        assert!(!error.message().contains("PRIVATE"));
        assert!(!super::super::safe_causes(&error).iter().any(|cause| cause.contains("PRIVATE")));
    }

    /// Exercise the actual public CLI failure renderer, not just the getter.
    #[test]
    fn public_failure_json_retains_recorded_effect() {
        let outcome = swarm_engine::PaeaGitPromoteOutcome::Recorded {
            intent_id: "intent".into(), action_digest: "digest".into(),
            target_ref: "refs/heads/release".into(), new_oid: "candidate".into(), replayed: true,
        };
        let error = close_tail(vec![swarm_engine::RootStoreError::Refused("fixture-close")], &outcome).unwrap();
        let mut stderr = Vec::new();
        let result = super::super::report(error, true, false, &mut stderr);
        assert_ne!(result.code, 0);
        let value: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(value["paea_git_effect"]["state"], "recorded");
        assert_eq!(value["paea_git_effect"]["intent_id"], "intent");
        assert_eq!(value["paea_git_effect"]["replayed"], true);
    }

    /// Neither a late publication error nor a pre-effect authority refusal
    /// proves that the business intention has no historical effect.
    #[test]
    fn public_failure_json_keeps_effect_history_unknown() {
        for error in [
            swarm_engine::PaeaGitPromoteCallError::Effect {
                outcome: None, cause: swarm_engine::RootStoreError::Refused("fixture-post-effect"),
            },
            swarm_engine::PaeaGitPromoteCallError::Action(
                swarm_engine::RootStoreError::Refused("fixture-authority-refusal")),
        ] {
            let mut stderr = Vec::new();
            let result = super::super::report(CliError::PaeaPromote(Box::new(error)), true, false, &mut stderr);
            assert_ne!(result.code, 0);
            let value: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
            assert_eq!(value["paea_git_effect"]["state"], "unknown");
            assert!(value["paea_git_effect"].get("new_oid").is_none());
        }
    }

    /// Render the same typed observation directly and behind independent close
    /// failure. Exercise both CLI languages and preserve safe primary cause.
    fn check_text_observation(known: bool) {
        fn error(known: bool) -> CliError {
            CliError::PaeaPromote(Box::new(swarm_engine::PaeaGitPromoteCallError::Effect {
                outcome: known.then(|| swarm_engine::PaeaGitPromoteOutcome::Recorded {
                    intent_id: "intent".into(), action_digest: "digest".into(),
                    target_ref: "refs/heads/release".into(), new_oid: "candidate".into(), replayed: false,
                }),
                cause: retained_cause("fixture-primary-close", io::Error::other("PRIVATE nested cause")),
            }))
        }
        let state = if known { "recorded" } else { "unknown" };
        for english in [false, true] {
            let mut control = Vec::new();
            let result = super::super::report(error(known), false, english, &mut control);
            assert_ne!(result.code, 0);
            assert!(String::from_utf8(control).unwrap().contains(&format!("effect_state={state}")));
            let nested = CliError::RootCleanup {
                primary: Box::new(error(known)),
                cleanup: Box::new(swarm_engine::RootStoreError::Refused("fixture-secondary-close")),
            };
            let mut output = Vec::new();
            let result = super::super::report(nested, false, english, &mut output);
            assert_ne!(result.code, 0);
            let text = String::from_utf8(output).unwrap();
            assert!(text.contains(&format!("effect_state={state}")), "close wrapper hid effect observation");
            assert!(text.contains("fixture-primary-close"), "close wrapper hid safe primary cause");
            assert!(!text.contains("PRIVATE"));
        }
    }

    #[test]
    fn public_failure_text_retains_recorded_through_cleanup() { check_text_observation(true); }
    #[test]
    fn public_failure_text_retains_unknown_through_cleanup() { check_text_observation(false); }

}
