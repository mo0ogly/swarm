//! Guarded two-phase CLI consumer of the engine's PAEA Git transfer call.
//! The prepared call keeps both physical root witnesses alive inside this
//! process while the operator provisions its exact permission out of band;
//! the private stdin frame then supplies the protected declaration pin and
//! personal password. No auto-grant, ref effect, remote adoption or accepted
//! service exists here: the prepared operation is always Reconcile (pure
//! native observation, no ref write), because `paea_git_transfer_call` binds
//! Operation::Reconcile at preparation time and consumption recompares it.
use super::{personal_secret_frame, Arguments, CliError};
use std::{
    io::{self, IsTerminal, Read, Write},
    path::Path,
};
use swarm_engine::RootStore;
use swarm_enrollment_authority::{
    authenticated_control::Context as UseContext, authenticated_startup,
};
use swarm_enrollment_state::Context;

/// Fixed private framing version shared with the Trust enrollment adapter. The
/// packet layout is `swarm-trust-source-v1\n<pin64>\n<password>`; the pin names
/// the protected declaration of this call's operator, never the Git journal pin
/// passed as a public argument. The password stays inside the Zeroizing packet
/// and is dropped before any result line is emitted.
const HEADER: &[u8] = b"swarm-trust-source-v1\n";

/// Dispatch the guarded `paea transfer-call SERVICE PROJECT WORK CHILD PIN
/// STAGING DECLARATION --input REQUESTFILE` command. Both roots are opened
/// writable (the separated-root check is contractual and runs at preparation),
/// the request file is a strict `{proposal, context}` document, DECLARATION is
/// the operator's provisioned protected source (validated only at startup,
/// after the prepared event), and stdin must not be a terminal: the private
/// frame arrives after the prepared event, mirroring the Trust enrollment
/// handshake. Every refusal closes all acquired owners and retains their close
/// causes; observation proves neither acceptance nor authority and grants no
/// permission.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 9
        || a.pos[1] != "transfer-call"
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
    run(a, out, &mut io::stdin().lock(), &raw)
}

/// Keep the owned prepared call alive through external permission provisioning,
/// then consume the one-shot native preparation under actual held authority.
/// Every early refusal closes the acquired roots and, once prepared, the call
/// itself; all close failures stay typed beside the primary cause, never
/// dropped and never invented into a separate success.
fn run(a: &Arguments, out: &mut dyn Write, input: &mut dyn Read, raw: &[u8]) -> Result<u8, CliError> {
    let root = |e| CliError::Root(Box::new(e));
    let project = RootStore::open(Path::new(&a.pos[3]), true).map_err(root)?;
    let service = match RootStore::open(Path::new(&a.pos[2]), true) {
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
            .paea_git_transfer_call(
                &project,
                &document.proposal,
                &document.context,
                &a.pos[4],
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
            "event":"paea-transfer-prepared",
            "service":"repository.promote",
            "operation":"reconcile",
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
                    "paea-transfer-call-input",
                ))),
                closes,
            ));
        }
    };
    // The frame is HEADER + pin64 + '\n' + nonempty personal password (the
    // transfer call always carries a password, unlike the serve-only
    // enrollment frame): a bare pin or missing separator refuses here, before
    // any declaration is opened and before any preparation is consumed.
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
                    "paea-transfer-call-frame",
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
                    "paea-transfer-call-startup",
                ))),
                closes,
            ));
        }
    };
    let result = service
        .call_paea_git_transfer(
            &project,
            &prepared,
            &app,
            &UseContext::new(),
            &head,
            &packet[cut + 1..],
        )
        .map_err(|e| CliError::PaeaTransfer(Box::new(e)));
    drop(packet);
    // The receipt escapes only as the final JSON line; every retained owner is
    // checked-closed on every outcome and each close failure stays retained.
    // The authority close cause is a distinct typed kernel error, retained as
    // its own Refused witness rather than a dropped or invented success.
    if let Err(_) = app.close() {
        closes.push(swarm_engine::RootStoreError::Refused(
            "paea-transfer-call-authority-close",
        ));
    }
    closes.extend(prepared.close().err());
    closes.extend(project.close().err());
    closes.extend(service.close().err());
    let result = match result {
        Ok(result) => result,
        Err(primary) => return Err(failed(Ok(()), Ok(()), primary, closes)),
    };
    let written = line(
        out,
        &serde_json::json!({
            "schema_version":1,
            "event":"paea-transfer-result",
            "candidate":result.candidate,
            "pack_bytes":result.pack_bytes,
            "pack_sha256":result.pack_sha256
        }),
    )
    .map_err(root);
    match written {
        Ok(()) => Ok(0),
        Err(primary) => Err(failed(Ok(()), Ok(()), primary, closes)),
    }
}

/// Emit and flush one public JSON line; bytes contain no credential or pin.
fn line(out: &mut dyn Write, value: &impl serde::Serialize) -> Result<(), swarm_engine::RootStoreError> {
    serde_json::to_writer(&mut *out, value)
        .map_err(|_| swarm_engine::RootStoreError::Refused("paea-transfer-call-output"))?;
    out.write_all(b"\n")
        .and_then(|()| out.flush())
        .map_err(|_| swarm_engine::RootStoreError::Refused("paea-transfer-call-output"))
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
