//! Guarded single-intent CLI bootstrap that binds one frozen approval command
//! to the right PROJECT root and the existing native HTTP server. The command
//! reuses the established preparation chain: it opens both physical roots,
//! prepares the selected PAEA Git call (transfer or promotion variant), freezes
//! the exact business Trust command from the provided file through
//! `PaeaGitCallSource::prepare_approval`, and publishes BOTH exact permissions
//! in one prepared event before any external provisioning. The private stdin
//! frame then supplies the protected declaration pin (pin-only, mirroring
//! `trust enrollment-serve`); the declaration is opened, the CLI's own held
//! roots are closed before binding, and `bind_git_call_with_approval` starts
//! the existing server with the effect source and the frozen approval command
//! bound to the clean PROJECT root. No request selects a root, command or
//! grant; no account, Git effect, journal write or approval is created by this
//! startup; absence or refusal of the command source keeps its cause and never
//! publishes a bound or ready state.
//!
//! Closure ownership on refusal paths: every held witness has an explicit
//! public close — `RootStore::close` for the two roots,
//! `PaeaGitCallSource::close_prepared` for the prepared call and
//! `PaeaGitCallApprovalSource::close_prepared` for the frozen approval — and
//! this module calls each exactly once on every refusal path, retaining all
//! typed causes. No `Drop` impl, no process-exit reclamation and no empty
//! success stands in for a close: a close failure is composed under the typed
//! `Cleanup` join and reported beside the primary refusal. This matches the
//! engine's own contract that an implicit drop is never evidence of a checked
//! close. After `bind_git_call_with_approval` the server owns every witness
//! and closes them through `serve_until` by existing contract.
use super::{personal_secret_frame, Arguments, CliError};
use std::{
    io::{self, IsTerminal, Read, Write},
    path::{Path, PathBuf},
};
use swarm_engine::RootStore;
use swarm_enrollment_authority::{
    paea_action::Operation as PromoteOperation, trust_action::Operation as ApprovalOperation,
};
use swarm_web::{HttpServiceError as H, NativeServerError as N};

/// Fixed private framing version shared with the Trust enrollment adapter. The
/// serve packet layout is `swarm-trust-source-v1\n<pin64>\n` exactly: pin only,
/// no password (the approval consumption happens over HTTP with the operator's
/// own Enrollment header, never through this frame).
const HEADER: &[u8] = b"swarm-trust-source-v1\n";

/// Selected engine call kind. Transfer keeps a staging input; commit and
/// reconcile are the promotion variants and take no staging — the uniform CLI
/// position then carries the explicit `-` marker, never a silently ignored
/// value.
#[derive(Clone, Copy)]
enum Call {
    Transfer,
    Promote(PromoteOperation),
}

/// Preserve the original Root cause under the existing HTTP/native wrapper.
fn root(error: swarm_engine::RootStoreError) -> N {
    N::Service(H::Root {
        operation: "paea-approval-web",
        cause: Box::new(error),
    })
}

/// Map a typed git-call source refusal (startup/close branches retained by the
/// producer) without formatting paths, pins or private causes.
fn source_refusal(error: swarm_web::PaeaGitCallSourceError) -> N {
    N::Service(H::GitCallSource {
        primary: None,
        cause: Box::new(error),
    })
}

/// Join original and independent close failures; no success/rollback/retry is
/// inferred. Both typed branches remain available through native diagnostics.
fn finish<T>(result: Result<T, N>, cleanup: Result<(), N>) -> Result<T, N> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(N::Cleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}

/// Close the acquired roots after a setup refusal that predates the prepared
/// call source; both close causes are retained beside the primary failure,
/// never dropped. Refusals after the source exists use the caller-site closes
/// that also release the prepared call and approval witnesses explicitly.
fn failed(service: Option<RootStore>, project: Option<RootStore>, error: N) -> N {
    let result = finish::<()>(
        Err(error),
        project.map(|p| p.close().map_err(root)).unwrap_or(Ok(())),
    );
    finish(result, service.map(|s| s.close().map_err(root)).unwrap_or(Ok(())))
        .unwrap_err()
}

/// Close both held roots independently, joining both close causes beside the
/// primary error; used on refusal paths where the source or approval witness
/// is closed separately at the same call site. Consumes the roots: each
/// branch that calls this returns immediately after, so each owner is closed
/// exactly once.
fn failed_inner(service: RootStore, project: RootStore) -> Result<(), N> {
    let project_close = project.close().map_err(root);
    let service_close = service.close().map_err(root);
    finish(project_close, service_close)
}

/// Emit and flush one public JSON line; bytes contain no credential or pin.
fn line(out: &mut dyn Write, value: &impl serde::Serialize) -> Result<(), N> {
    serde_json::to_writer(&mut *out, value)
        .map_err(|cause| N::Service(H::ResponseJson(cause)))?;
    out.write_all(b"\n")
        .and_then(|()| out.flush())
        .map_err(|cause| N::Io {
            phase: "paea-approval-web-output",
            cause,
        })
}

/// Validate the fixed CLI shape and read both startup files before any Root is
/// opened: the request document and the frozen approval command are pure reads,
/// so their refusals close nothing. DECLARATION is chosen by the local
/// operator and must be an absolute path, mirroring the serve sibling; stdin
/// must not be a terminal. No grant, account or Git effect exists here.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let call = match a.pos.get(2).map(String::as_str) {
        Some("transfer") => Call::Transfer,
        Some("commit") => Call::Promote(PromoteOperation::Commit),
        Some("reconcile") => Call::Promote(PromoteOperation::Reconcile),
        _ => return Err(CliError::Usage),
    };
    let approval_operation = match a.pos.get(3).map(String::as_str) {
        Some(spelling) => ApprovalOperation::parse(spelling).map_err(|_| CliError::Usage)?,
        None => return Err(CliError::Usage),
    };
    // The promotion engine takes no staging input: the uniform position must
    // carry the explicit `-` marker rather than a value that would be ignored.
    if matches!(call, Call::Promote(_)) && a.pos.get(9).map(String::as_str) != Some("-") {
        return Err(CliError::Usage);
    }
    if (a.pos.len() != 12 && a.pos.len() != 13)
        || a.server
        || a.input.is_empty()
        || a.input == "-"
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
        || a.pos.get(10).map(String::as_str) == Some("-")
        || !Path::new(&a.pos[11]).is_absolute()
        || io::stdin().is_terminal()
    {
        return Err(CliError::Usage);
    }
    let raw = swarm_engine::read_command_file(Path::new(&a.input))
        .map_err(CliError::CommandInput)?;
    let document = swarm_protection_contract::decode_document(&raw)
        .map_err(CliError::Protection)?;
    let command = swarm_engine::read_command_file(Path::new(&a.pos[10]))
        .map_err(CliError::CommandInput)?;
    run(
        a,
        out,
        &mut io::stdin().lock(),
        call,
        approval_operation,
        &document,
        &command,
    )
    .map_err(|error| CliError::NativeWeb {
        cause: Box::new(error.cause),
        english: a.language == "en",
    })
}

/// Keep the owned prepared call and frozen approval command alive through
/// external permission provisioning, then open the independently provisioned
/// declaration, close the held roots and bind the existing server with the
/// effect source and approval attached. Every refusal closes every held
/// witness explicitly — roots through their own `RootStore::close`, the
/// prepared call through `PaeaGitCallSource::close_prepared`, the frozen
/// approval through `PaeaGitCallApprovalSource::close_prepared` — and composes
/// all typed causes; a close failure is reported under the `Cleanup` join, not
/// swallowed or replaced by an empty success. On an `open()` startup refusal
/// the source has already released the prepared call itself; this module
/// closes the approval witness beside it. After bind the server owns every
/// witness: `serve_until` closes the service, which closes the authority, the
/// approval and the prepared call by existing contract — this module closes
/// nothing after `serve_until` returns.
fn run(
    a: &Arguments,
    out: &mut dyn Write,
    input: &mut dyn Read,
    call: Call,
    approval_operation: ApprovalOperation,
    document: &swarm_protection_contract::Document,
    command: &[u8],
) -> Result<u8, RunError> {
    let project = RootStore::open(Path::new(&a.pos[5]), true).map_err(root)?;
    let service = match RootStore::open(Path::new(&a.pos[4]), true) {
        Ok(service) => service,
        Err(cause) => return Err(failed(None, Some(project), root(cause)).into()),
    };
    // Prepare the selected engine call; the exact staging input is revalidated
    // by the job against the journal pin and managed definition.
    let source = match call {
        Call::Transfer => {
            let prepared = match service.paea_git_transfer_call(
                &project,
                &document.proposal,
                &document.context,
                &a.pos[6],
                &a.pos[7],
                &a.pos[8],
                &a.pos[9],
            ) {
                Ok(prepared) => prepared,
                Err(cause) => {
                    return Err(failed(Some(service), Some(project), root(cause)).into())
                }
            };
            swarm_web::PaeaGitCallSource::new_transfer(
                prepared,
                PathBuf::from(&a.pos[4]),
                PathBuf::from(&a.pos[5]),
            )
        }
        Call::Promote(operation) => {
            let prepared = match service.paea_git_promote_call(
                &project,
                &document.proposal,
                &document.context,
                operation,
                &a.pos[6],
                &a.pos[7],
                &a.pos[8],
            ) {
                Ok(prepared) => prepared,
                Err(cause) => {
                    return Err(failed(Some(service), Some(project), root(cause)).into())
                }
            };
            swarm_web::PaeaGitCallSource::new_promote(
                prepared,
                PathBuf::from(&a.pos[4]),
                PathBuf::from(&a.pos[5]),
            )
        }
    };
    // From here the prepared call witness lives inside the source until an
    // explicit `close_prepared` release on a refusal path below, or the bind;
    // the engine forbids counting an implicit drop as a checked close.
    // Freeze the exact business Trust command against the same project root
    // BEFORE the prepared event: the approval permission derived from these
    // exact bytes is what the operator provisions into the signed declaration,
    // so the held grant can never authorize a different operation or command.
    let approval = match source.prepare_approval(approval_operation, command) {
        Ok(approval) => approval,
        Err(cause) => {
            // The source's prepared witness is released explicitly beside the
            // roots; every typed cause is joined, none dropped.
            let result = finish::<()>(
                Err(root(cause)),
                source.close_prepared().map_err(root),
            );
            return Err(
                finish(result, failed_inner(service, project)).unwrap_err().into()
            );
        }
    };
    // Public prepared event with BOTH exact permissions: the operator
    // provisions one declaration carrying both out of band before answering
    // on private stdin. No authority is established and this process must
    // stay alive; the witnesses are memory-only and restart-destroyed.
    let call_request = source.request();
    let approval_request = approval.request();
    let event = serde_json::json!({
        "schema_version":1,
        "event":"paea-approval-web-prepared",
        "call":source.kind(),
        "service":"repository.promote",
        "call_operation": match call {
            Call::Transfer => "reconcile",
            Call::Promote(operation) => operation.as_str(),
        },
        "approval_operation": approval_operation.as_str(),
        "root_digest":call_request.root_digest(),
        "action_digest":call_request.action_digest(),
        "call_permission":call_request.permission(),
        "approval_root_digest":approval_request.root_digest(),
        "approval_request_digest":approval_request.request_digest(),
        "approval_permission":approval_request.permission(),
        "authority":false,
        "restart_reusable":false
    });
    if let Err(e) = line(out, &event) {
        // Explicit release of all three witnesses; every typed cause joined.
        let result = finish::<()>(Err(e), source.close_prepared().map_err(root));
        let result = finish(result, approval.close_prepared().map_err(root));
        return Err(finish(result, failed_inner(service, project))
            .unwrap_err()
            .into());
    }
    // Pin-only serve frame, mirroring `trust enrollment-serve`: a wrong
    // header, length or separator refuses before any declaration is opened
    // and before any preparation is consumed.
    let packet = match personal_secret_frame::read_packet(input) {
        Ok(packet) => packet,
        Err(e) => {
            let result = finish::<()>(
                Err(N::Io {
                    phase: "paea-approval-web-input",
                    cause: io::Error::other(e),
                }),
                source.close_prepared().map_err(root),
            );
            let result = finish(result, approval.close_prepared().map_err(root));
            return Err(finish(result, failed_inner(service, project))
                .unwrap_err()
                .into());
        }
    };
    let cut = HEADER.len() + 64;
    if packet.len() != cut + 1 || !packet.starts_with(HEADER) || packet[cut] != b'\n' {
        let result =
            finish::<()>(Err(N::Refused("paea-approval-web-frame")), source.close_prepared().map_err(root));
        let result = finish(result, approval.close_prepared().map_err(root));
        return Err(finish(result, failed_inner(service, project))
            .unwrap_err()
            .into());
    }
    let pin = match std::str::from_utf8(&packet[HEADER.len()..cut]) {
        Ok(p) if swarm_identity_contract::digest(p.as_bytes()) => p,
        _ => {
            let result =
                finish::<()>(Err(N::Refused("paea-approval-web-pin")), source.close_prepared().map_err(root));
            let result = finish(result, approval.close_prepared().map_err(root));
            return Err(finish(result, failed_inner(service, project))
                .unwrap_err()
                .into());
        }
    };
    // Independently provisioned protected declaration for this exact operator:
    // the CLI creates no account, adopts no remote source and mints no grant.
    // On a startup refusal `open` has already released the prepared call
    // witness itself; this module closes the approval witness beside it.
    let opened = match source.open(&a.pos[11], pin) {
        Ok(opened) => opened,
        Err(e) => {
            let result = finish::<()>(Err(source_refusal(e)), approval.close_prepared().map_err(root));
            return Err(finish(result, failed_inner(service, project))
                .unwrap_err()
                .into());
        }
    };
    drop(packet);
    // Close the CLI's own held roots BEFORE binding: the bind opens fresh
    // roots from the trusted startup paths and rechecks both prepared
    // witnesses against them. Both roots close independently — a service-root
    // close failure never skips the project close — and every close cause is
    // joined beside the other. On this branch the approval witness is NOT yet
    // attached to the authority, so it is released explicitly beside
    // `opened.close()`, which owns the authority and the prepared call.
    let project_close = project.close().map_err(root);
    let service_close = service.close().map_err(root);
    if let Err(cause) = finish(project_close, service_close) {
        let result = finish::<()>(Err(cause), opened.close().map_err(source_refusal));
        let result = finish(result, approval.close_prepared().map_err(root));
        return Err(result.unwrap_err().into());
    }
    // Bind refusal closes the supplied authority (approval witness included)
    // inside the bind contract, retaining both typed causes; nothing extra is
    // closed here.
    let server = swarm_web::NativeHttpServer::bind_git_call_with_approval(
        Path::new(&a.pos[4]),
        a.pos.get(12).map(String::as_str),
        opened,
        Some(approval),
    )?;
    let address = match server.local_addr() {
        Ok(address) => address,
        Err(e) => return finish(Err(e), server.close()).map_err(Into::into),
    };
    if let Err(e) = line(
        out,
        &serde_json::json!({"schema_version":1,"event":"paea-approval-web-bound","address":address,"runtime_ready":false,"scope":"single_prepared_git_call","mission_conductor":false}),
    ) {
        return finish(Err(e), server.close()).map_err(Into::into);
    }
    // Every observed outcome escapes as a public JSON line; the server owns
    // all witnesses and closes them through `serve_until` by existing contract.
    let mut observer =
        |outcome: &swarm_web::NativeJobOutcome| line(out, &outcome.record()).map_err(io::Error::other);
    let mut signal = None;
    let result = server.serve_until(
        async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                signal = Some(e);
            }
        },
        &mut observer,
    );
    let result = match signal {
        None => result,
        Some(cause) => finish(
            result,
            Err(N::Io {
                phase: "paea-approval-web-signal",
                cause,
            }),
        ),
    };
    result.map(|_| 0).map_err(Into::into)
}

/// Typed run failure carrying only the native cause; no Trust publication
/// exists on this startup path.
struct RunError {
    cause: N,
}
impl From<N> for RunError {
    fn from(cause: N) -> Self {
        Self { cause }
    }
}
