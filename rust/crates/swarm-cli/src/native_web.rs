//! Foreground native API command. Output is an explicit observation stream, not a
//! durable journal or proof of human/agent identity. No Go process is restarted.
use super::{Arguments, CliError};
mod conductor;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
};
use swarm_web::{NativeHttpServer, NativeJobOutcome, NativeServerError};

/// Bootstrap CLI validates arguments before Root/session creation. Address override
/// uses the persisted-policy validator without saving it. The bound announcement
/// precedes runtime initialization and explicitly is not readiness; caller must
/// probe the protected API. Init is never implicit. The foreground process installs
/// Tokio's SIGINT handler (which may remain installed for the process lifetime).
/// Each outcome is serialized without body/credential and explicitly flushed before
/// acknowledgement. The caller-owned synchronous Write must return/progress; blocked
/// sinks are not made cancellable by this command. No output watchdog/retry is added.
/// Native errors preserve pending effects/causes; owned server closes on bootstrap
/// output failure and drains started Root jobs on output failure/SIGINT. No other
/// process/Root is stopped or adopted. The server-owned coordinator may invoke
/// saved, consented mission roles/workers through their existing guards; HTTP
/// requests never own a mission loop. Local login and native
/// settings Admin accompany the partial API; other UI/Admin routes remain unported.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if !(1..=2).contains(&a.pos.len())
        || !a.input.is_empty()
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
        || a.server
    {
        return Err(CliError::Usage);
    }
    let english = a.language == "en";
    let server = NativeHttpServer::bind(Path::new(&a.root), a.pos.get(1).map(String::as_str))
        .map_err(|cause| native(cause, english))?;
    let address = match server.local_addr() {
        Ok(address) => address,
        Err(error) => return Err(failed_bound(server, error, english)),
    };
    let announcement = if a.json {
        line(
            out,
            &Bound {
                schema_version: 1,
                event: "web-bound",
                address,
                settings_revision: server.settings_revision(),
                runtime_ready: false,
            },
        )
    } else {
        let text = if english {
            format!("Native development HTTP API bound at http://{address}; runtime startup pending. Live agents: http://{address}/live.html. Outcome stream follows.
")
        } else {
            format!("API HTTP native de développement liée à http://{address} ; démarrage runtime en attente. Agents en direct : http://{address}/live.html. Flux d’observation ensuite.
")
        };
        out.write_all(text.as_bytes()).and_then(|()| out.flush())
    };
    if let Err(cause) = announcement {
        return Err(failed_bound(
            server,
            NativeServerError::Io {
                phase: "web-bound-output",
                cause,
            },
            english,
        ));
    }
    let mut signal_error = None;
    let result = {
        let mut observer = |outcome: &NativeJobOutcome| {
            line(
                out,
                &OutcomeEvent {
                    schema_version: 1,
                    event: "web-outcome",
                    outcome: outcome.record(),
                },
            )
        };
        conductor::serve(
            server, std::path::PathBuf::from(&a.root),
            async {
                if let Err(error) = tokio::signal::ctrl_c().await {
                    signal_error = Some(error);
                }
            },
            &mut observer,
        )
    };
    let result = match signal_error {
        None => result,
        Some(cause) => Err(NativeServerError::SessionState {
            primary: Box::new(match result {
                Ok(report) => NativeServerError::StoppedOutput {
                    phase: "web-shutdown-signal",
                    cause,
                    report: Box::new(report),
                },
                Err(primary) => NativeServerError::Cleanup {
                    primary: Box::new(primary),
                    cleanup: Box::new(NativeServerError::Io {
                        phase: "web-shutdown-signal",
                        cause,
                    }),
                },
            }),
        }),
    };
    let result = match result {
        Ok(report) => match line(out, &stop_record(&report)) {
            Ok(()) => Ok(report),
            Err(cause) => Err(NativeServerError::SessionState {
                primary: Box::new(NativeServerError::StoppedOutput {
                    phase: "web-summary-output",
                    cause,
                    report: Box::new(report),
                }),
            }),
        },
        Err(error) => Err(error),
    };
    result.map(|_already_reported| 0).map_err(|cause| {
        let work = cause
            .recorded_work()
            .map(|(id, revision)| (id.to_owned(), revision));
        let error = native(cause, english);
        match work {
            Some((work, revision)) => CliError::StatePreserved {
                primary: Box::new(error),
                work,
                revision,
                english,
            },
            None => error,
        }
    })
}
/// Owned bound socket, not runtime readiness. No secret/Root path is published.
#[derive(serde::Serialize)]
struct Bound {
    schema_version: u8,
    event: &'static str,
    address: std::net::SocketAddr,
    settings_revision: swarm_engine::SettingCount,
    runtime_ready: bool,
}
/// End-of-loop summary includes handled refusal/parser families without formatting
/// untrusted diagnostics. Clean server shutdown does not turn refused operations
/// into successful business results. Detailed acknowledged outcomes precede it.
#[derive(serde::Serialize)]
struct StopRecord<'a> {
    schema_version: u8,
    event: &'static str,
    accepted_connections: u64,
    completed_jobs: u64,
    rejected_requests: u64,
    rejected_connections: u64,
    rejected_queued_jobs: u64,
    connection_failures: u64,
    disconnected_replies: u64,
    observed_outcomes: u64,
    configuration_outcomes: u64,
    work_outcomes: u64,
    first_request_failure: Option<String>,
    first_service_failure: Option<&'static str>,
    first_connection_failure: Option<ConnectionFailure>,
    last_recorded_work: Option<&'a (String, i64)>,
    last_recorded_configuration: Option<&'a swarm_web::RecordedConfiguration>,
}
/// Facts about a retained Hyper failure, not its arbitrary Display text.
#[derive(serde::Serialize)]
struct ConnectionFailure {
    parse: bool,
    timeout: bool,
    causes: Vec<String>,
}
/// Borrow final counters/known values and classify first causes; no IO or retry.
/// A last known configuration entry is not a claim about later uncertain writes.
fn stop_record(report: &swarm_web::NativeServerReport) -> StopRecord<'_> {
    StopRecord {
        schema_version: 1,
        event: "web-stopped",
        accepted_connections: report.accepted_connections,
        completed_jobs: report.completed_jobs,
        rejected_requests: report.rejected_requests,
        rejected_connections: report.rejected_connections,
        rejected_queued_jobs: report.rejected_queued_jobs,
        connection_failures: report.connection_failures,
        disconnected_replies: report.disconnected_replies,
        observed_outcomes: report.observed_outcomes,
        configuration_outcomes: report.configuration_outcomes,
        work_outcomes: report.work_outcomes,
        first_request_failure: report
            .first_request_failure
            .as_ref()
            .map(ToString::to_string),
        first_service_failure: report
            .first_service_failure
            .as_ref()
            .map(swarm_web::HttpServiceError::code),
        first_connection_failure: report.first_connection_failure.as_ref().map(|error| {
            ConnectionFailure {
                parse: error.is_parse(),
                timeout: error.is_timeout(),
                causes: super::safe_causes(error),
            }
        }),
        last_recorded_work: report.last_recorded_work.as_ref(),
        last_recorded_configuration: report.last_recorded_configuration.as_ref(),
    }
}
/// Explicit protocol envelope for a safe borrowed outcome projection. Ack/flush
/// does not establish downstream receipt, disk sync or business completion.
#[derive(serde::Serialize)]
struct OutcomeEvent<'a> {
    schema_version: u8,
    event: &'static str,
    outcome: swarm_web::NativeOutcomeRecord<'a>,
}
/// Compact NDJSON framing, followed by checked write and flush. Original serde/IO
/// causes remain typed. No lossy conversion, hidden size cap or retry is performed.
fn line(out: &mut dyn Write, value: &impl serde::Serialize) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    bytes.push(b'\n');
    out.write_all(&bytes)?;
    out.flush()
}
/// Preserve native typed error and language; no raw cause/path/body is rendered.
fn native(cause: NativeServerError, english: bool) -> CliError {
    CliError::NativeWeb {
        cause: Box::new(cause),
        english,
    }
}
/// Bootstrap already read/published a session. Close binding/listener explicitly
/// and retain both output/setup and cleanup failures, without removing credentials.
/// Close a bound server after a conductor startup failure, retaining both causes.
fn failed_conductor_start(server: NativeHttpServer, cause: swarm_engine::RootStoreError) -> NativeServerError {
    let primary = NativeServerError::Service(swarm_web::HttpServiceError::Root { operation: "web-conductor-start", cause: Box::new(cause) });
    match server.close() { Ok(()) => primary, Err(cleanup) => NativeServerError::Cleanup { primary: Box::new(primary), cleanup: Box::new(cleanup) } }
}
fn failed_bound(server: NativeHttpServer, primary: NativeServerError, english: bool) -> CliError {
    let error = match server.close() {
        Ok(()) => primary,
        Err(cleanup) => NativeServerError::Cleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        },
    };
    native(
        NativeServerError::SessionState {
            primary: Box::new(error),
        },
        english,
    )
}
/// Additional non-linear native error branches for the CLI's existing typed,
/// cycle-checked cause walkers. Standard source() still supplies each primary.
/// No IO, formatting of arbitrary data, authority or retry follows this traversal.
pub(super) fn branches<'a>(error: &'a (dyn Error + 'static)) -> Vec<&'a (dyn Error + 'static)> {
    let mut out: Vec<&'a (dyn Error + 'static)> = Vec::new();
    if let Some(error) = error.downcast_ref::<NativeServerError>() {
        match error {
            NativeServerError::Cleanup { cleanup, .. } => out.push(cleanup.as_ref()),
            NativeServerError::StoppedOutput { report, .. } => {
                if let Some(error) = report.first_request_failure.as_ref() {
                    out.push(error);
                }
                if let Some(error) = report.first_service_failure.as_ref() {
                    out.push(error);
                }
                if let Some(error) = report.first_connection_failure.as_ref() {
                    out.push(error);
                }
            }
            NativeServerError::OutcomeEncoding { outcome, .. } => {
                if let Some(error) = outcome.failure.as_ref() {
                    out.push(error);
                }
                out.extend(outcome.observation_causes.iter().map(|cause| cause.as_ref() as &(dyn Error + 'static)));
            }
            _ => (),
        }
    }
    if let Some(error) = error.downcast_ref::<swarm_web::NativeObservationFailure>() {
        for outcome in &error.pending {
            if let Some(error) = outcome.failure.as_ref() {
                out.push(error);
            }
            out.extend(outcome.observation_causes.iter().map(|cause| cause.as_ref() as &(dyn Error + 'static)));
        }
    }
    if let Some(error) = error.downcast_ref::<swarm_web::HttpServiceError>() {
        match error {
            swarm_web::HttpServiceError::Close { cleanup, .. } => out.push(cleanup.as_ref()),
            swarm_web::HttpServiceError::ResponseEncoding { cause, .. } => out.push(cause),
            _ => (),
        }
    }
    out
}
/// Add only classified native families to safe CLI diagnostics. Metadata values,
/// panic payloads, raw request/headers, paths and arbitrary cause messages stay out.
pub(super) fn family(error: &(dyn Error + 'static)) -> Option<&'static str> {
    if let Some(error) = error.downcast_ref::<NativeServerError>() {
        Some(error.code())
    } else if let Some(error) = error.downcast_ref::<swarm_web::HttpServiceError>() {
        Some(error.code())
    } else if let Some(error) = error.downcast_ref::<swarm_web::NativeObservationFailure>() {
        Some(error.code())
    } else if let Some(error) = error.downcast_ref::<swarm_web::NativeObservationCause>() {
        Some(match error {
            swarm_web::NativeObservationCause::Io(_) => "web-observer-io",
            swarm_web::NativeObservationCause::Panic(_) => "web-observer-panic",
        })
    } else {
        None
    }
}
/// Extend a native CLI failure with known session/effects and every pending safe
/// outcome projection. This neither invents precision for an unknown Root error
/// nor serializes original diagnostic payloads; all typed causes stay in CliError.
pub(super) fn details(error: &CliError, document: &mut serde_json::Value) {
    if let Some(native) = error
        .typed_causes()
        .into_iter()
        .find_map(|error| error.downcast_ref::<NativeServerError>())
    {
        document["web_session_may_be_recorded"] = native.session_state_may_be_recorded().into();
        document["configuration_may_be_recorded"] = native.configuration_may_be_recorded().into();
        if let Some((id, revision)) = native.recorded_work() {
            document["recorded_work"] = serde_json::json!({"id":id,"revision":revision});
        }
        if let Some(configuration) = native.recorded_configuration() {
            document["recorded_configuration"] = serde_json::json!(configuration);
        }
        if let Some(report) = native.stopped_report() {
            document["stopped_report"] = serde_json::json!(stop_record(report));
        }
        if let Some(failure) = native.observation_failure() {
            document["pending_outcomes"] = serde_json::json!(failure
                .pending
                .iter()
                .map(NativeJobOutcome::record)
                .collect::<Vec<_>>());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Typed aggregation model: a valid summary precedes a late IO failure, whose
    /// original cause/report/known effects survive CLI normalization and JSON output.
    #[test]
    fn late_web_summary_failure_retains_report_effects_and_safe_causes() {
        let report = swarm_web::NativeServerReport {
            configuration_outcomes: 1,
            work_outcomes: 1,
            last_recorded_work: Some(("owned-late-work".into(), 3)),
            last_recorded_configuration: Some(swarm_web::RecordedConfiguration {
                event_id: "owned-late-config".into(),
                event_revision: swarm_engine::SettingCount(2),
                current_revision: swarm_engine::SettingCount(2),
                replayed: false,
            }),
            first_request_failure: Some(swarm_web::NativeRequestError::Refused(
                "owned-client-refusal",
            )),
            ..Default::default()
        };
        let mut witness = Vec::new();
        line(&mut witness, &stop_record(&report)).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&witness).unwrap()["event"],
            "web-stopped"
        );
        let cause = NativeServerError::SessionState {
            primary: Box::new(NativeServerError::StoppedOutput {
                phase: "web-summary-output",
                cause: io::Error::new(io::ErrorKind::BrokenPipe, "owned diagnostic private text"),
                report: Box::new(report),
            }),
        };
        let cli = native(cause, true);
        assert!(cli.configuration_may_be_recorded());
        assert_eq!(cli.recorded_work(), Some(("owned-late-work".into(), 3)));
        let mut diagnostic = Vec::new();
        let outcome = super::super::report(cli, true, true, &mut diagnostic);
        assert_eq!(outcome.code, 2);
        assert!(outcome.diagnostic_error.is_none());
        let json: serde_json::Value = serde_json::from_slice(&diagnostic).unwrap();
        assert_eq!(json["failure"]["code"], "web-summary-output");
        assert_eq!(json["configuration_may_be_recorded"], true);
        assert_eq!(json["recorded_work"]["id"], "owned-late-work");
        assert_eq!(
            json["stopped_report"]["first_request_failure"],
            "owned-client-refusal"
        );
        assert!(!String::from_utf8(diagnostic)
            .unwrap()
            .contains("owned diagnostic private text"));
    }
}

#[cfg(test)]
mod ancillary_tests {
    use super::*;
    /// Typed fixture causes cross both late native wrappers. This tests actual
    /// owner/branch traversal, not a real HTTP write fault or Git execution.
    #[test]
    fn ancillary_native_causes_reach_cli_after_observation_and_encoding_failure() {
        let mut outcome = NativeJobOutcome::from_error(swarm_web::HttpServiceError::Refused("fixture-initial"));
        outcome.failure = None;
        outcome.observation_causes.push(std::sync::Arc::new(swarm_engine::RootStoreError::Refused("fixture-ancillary")));
        let outcome = outcome.observe(&mut |_| Ok(())).unwrap();
        let mut error = outcome.observe(&mut |_| Err(io::Error::new(io::ErrorKind::BrokenPipe,"fixture-observer"))).unwrap_err();
        assert!(branches(&error).iter().any(|cause| matches!(cause.downcast_ref::<swarm_engine::RootStoreError>(),Some(swarm_engine::RootStoreError::Refused("fixture-ancillary")))));
        let outcome=error.pending.remove(0);
        let encoding=serde_json::from_slice::<bool>(b"invalid").unwrap_err();
        let error=NativeServerError::OutcomeEncoding { cause: encoding, outcome: Box::new(outcome) };
        assert!(branches(&error).iter().any(|cause| matches!(cause.downcast_ref::<swarm_engine::RootStoreError>(),Some(swarm_engine::RootStoreError::Refused("fixture-ancillary")))));
    }
}
