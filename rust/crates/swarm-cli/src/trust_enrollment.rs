//! Trusted local two-phase enrollment adapter. No remote source adoption.
use super::{personal_secret_frame, Arguments, CliError};
use std::{
    io::{self, IsTerminal, Read, Write},
    path::Path,
};
use swarm_engine::{RootStore, RootStoreError, TrustEnrollmentRequest};
use swarm_enrollment_authority::trust_action::Operation;
use swarm_web::{HttpServiceError as H, NativeServerError as N, TrustEnrollmentSource};
/// Fixed private framing version, not a policy, credential or authorization.
const HEADER: &[u8] = b"swarm-trust-source-v1\n";

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
/// Preserve original Root cause under the existing HTTP/native wrapper matrix.
fn root(error: RootStoreError) -> N {
    N::Service(H::Root {
        operation: "trust-enrollment",
        cause: Box::new(error),
    })
}
/// Map safe startup/close data without formatting paths, pins or private causes.
fn source(error: swarm_web::TrustEnrollmentSourceError) -> N {
    N::Service(H::EnrollmentSource {
        primary: None,
        cause: Box::new(error),
    })
}
/// Close every acquired Root/request after a setup refusal; no source exists yet.
fn failed(store: RootStore, prepared: Option<TrustEnrollmentRequest>, error: N) -> N {
    let result = finish::<()>(
        Err(error),
        prepared.map(|p| p.close().map_err(root)).unwrap_or(Ok(())),
    );
    finish(result, store.close().map_err(root)).unwrap_err()
}
/// Emit and flush one public JSON line; bytes contain no credential or pin.
fn line(out: &mut dyn Write, value: &impl serde::Serialize) -> Result<(), N> {
    serde_json::to_writer(&mut *out, value).map_err(|cause| N::Service(H::ResponseJson(cause)))?;
    out.write_all(b"\n")
        .and_then(|()| out.flush())
        .map_err(|cause| N::Io {
            phase: "trust-enrollment-output",
            cause,
        })
}
/// Validate all fixed CLI shape before Root/input IO. The declaration is chosen
/// by the local operator, never HTTP. Preparation publishes an exact permission,
/// then stdin supplies an independently acquired declaration pin and (call only)
/// password. No grant/account is automatically issued and no watchdog is added.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let serve = a.pos.get(1).is_some_and(|s| s == "enrollment-serve");
    if !(if serve { 4..=5 } else { 4..=4 }).contains(&a.pos.len())
        || a.server
        || a.input.is_empty()
        || a.input == "-"
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
        || !Path::new(&a.pos[3]).is_absolute()
    {
        return Err(CliError::Usage);
    }
    let operation = Operation::parse(&a.pos[2]).map_err(|_| CliError::Usage)?;
    if io::stdin().is_terminal() {
        return Err(CliError::Usage);
    }
    let raw =
        swarm_engine::read_command_file(Path::new(&a.input)).map_err(CliError::CommandInput)?;
    run(a, out, &mut io::stdin().lock(), operation, &raw, serve).map_err(|error| {
        let cause = CliError::NativeWeb {
            cause: Box::new(error.cause),
            english: a.language == "en",
        };
        match error.publication {
            Some(publication) => CliError::TrustPublication {
                primary: Box::new(cause),
                publication,
            },
            None => cause,
        }
    })
}
/// Keep prepared witness alive across external provisioning. Every early refusal
/// closes it and Root explicitly; input owns a zeroizing guarded allocation.
struct RunError {
    cause: N,
    publication: Option<&'static str>,
}
impl From<N> for RunError {
    fn from(cause: N) -> Self {
        Self {
            cause,
            publication: None,
        }
    }
}
/// Keep the owned Root/nonce alive through out-of-band provisioning, then run
/// the exact action or bind a single-intent server with no mission conductor.
/// Input/output/authority/close errors retain known publication independently.
fn run(
    a: &Arguments,
    out: &mut dyn Write,
    input: &mut dyn Read,
    operation: Operation,
    raw: &[u8],
    serve: bool,
) -> Result<u8, RunError> {
    let store = RootStore::open(Path::new(&a.root), true).map_err(root)?;
    let prepared = match store.trust_enrollment_request(operation, raw) {
        Ok(p) => p,
        Err(e) => return Err(failed(store, None, root(e)).into()),
    };
    let request = prepared.request();
    let event = serde_json::json!({"schema_version":1,"event":"trust-enrollment-prepared","operation":operation.as_str(),"root_digest":request.root_digest(),"request_digest":request.request_digest(),"permission":request.permission(),"authority":false,"restart_reusable":false});
    if let Err(e) = line(out, &event) {
        return Err(failed(store, Some(prepared), e).into());
    }
    let packet = match personal_secret_frame::read_packet(input) {
        Ok(p) => p,
        Err(e) => {
            return Err(failed(
                store,
                Some(prepared),
                N::Io {
                    phase: "trust-enrollment-input",
                    cause: io::Error::other(e),
                },
            )
            .into())
        }
    };
    let cut = HEADER.len() + 64;
    if packet.len() < cut + 1
        || !packet.starts_with(HEADER)
        || packet[cut] != b'\n'
        || (serve && packet.len() != cut + 1)
        || (!serve && packet.len() == cut + 1)
    {
        return Err(failed(store, Some(prepared), N::Refused("trust-enrollment-frame")).into());
    }
    let pin = match std::str::from_utf8(&packet[HEADER.len()..cut]) {
        Ok(p) if swarm_identity_contract::digest(p.as_bytes()) => p,
        _ => return Err(failed(store, Some(prepared), N::Refused("trust-enrollment-pin")).into()),
    };
    let identity = match TrustEnrollmentSource::open(prepared, &a.pos[3], pin) {
        Ok(s) => s,
        Err(e) => return Err(failed(store, None, source(e)).into()),
    };
    if serve {
        drop(packet);
        if let Err(e) = store.close() {
            return finish::<u8>(Err(root(e)), identity.close().map_err(source))
                .map_err(Into::into);
        }
        let server = swarm_web::NativeHttpServer::bind_enrolled(
            Path::new(&a.root),
            a.pos.get(4).map(String::as_str),
            identity,
        )?;
        let address = match server.local_addr() {
            Ok(v) => v,
            Err(e) => return finish(Err(e), server.close()).map_err(Into::into),
        };
        if let Err(e) = line(
            out,
            &serde_json::json!({"schema_version":1,"event":"trust-enrollment-bound","address":address,"runtime_ready":false,"scope":"single_prepared_intent","mission_conductor":false}),
        ) {
            return finish(Err(e), server.close()).map_err(Into::into);
        }
        let mut observer = |outcome: &swarm_web::NativeJobOutcome| {
            line(out, &outcome.record()).map_err(|e| io::Error::other(e))
        };
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
                    phase: "trust-enrollment-signal",
                    cause,
                }),
            ),
        };
        result.map(|_| 0).map_err(Into::into)
    } else {
        let result = identity
            .call(&store, operation.as_str(), raw, &packet[cut + 1..])
            .map_err(root);
        drop(packet);
        let publication = result.as_ref().ok().and_then(|r| r.publication);
        let result = finish(
            finish(result, identity.close().map_err(source)),
            store.close().map_err(root),
        );
        let written=result.and_then(|reply|line(out,&serde_json::json!({"schema_version":1,"event":"trust-enrollment-result","result_json":String::from_utf8(reply.bytes).map_err(|_|N::Refused("trust-enrollment-response-utf8"))?,"publication":reply.publication})).map(|_|0));
        written.map_err(|cause| RunError { cause, publication })
    }
}
