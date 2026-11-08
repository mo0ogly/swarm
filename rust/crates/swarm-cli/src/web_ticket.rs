//! `swarm-rust web-ticket ADDRESS [PATH] --root ROOT --json`: issue one
//! single-use sign-in link for the cockpit already serving ADDRESS, so the
//! local launcher can open the browser without the owner key being copied.
//! `swarm-rust web-rotate ADDRESS --root ROOT --json`: replace that address's
//! owner key while its server is stopped.
use crate::{json_bytes, write_output, Arguments, CliError};
use serde::Serialize;
use std::{io::Write, net::SocketAddr, path::Path};

#[derive(Serialize)]
struct Response<'a> {
    schema_version: i64,
    url: String,
    expires_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<&'a str>,
}

/// Issue a ticket and print `{schema_version, url, expires_at[, next]}`.
///
/// Preconditions: exactly ADDRESS (a loopback `IP:PORT` the server listens
/// on) and an optional same-origin PATH starting with one `/`; `--json` is
/// required and no `--input`/`--output`/`--task`/`--server` is accepted.
/// Effects: publishes one private expiry file in ROOT's `.swarm` (writable
/// Root required); nothing else in the Root changes and no server is
/// contacted, so a ticket for a stopped server simply expires.
/// Errors: usage; `web-ticket-address-invalid` for a non-loopback or
/// malformed address; Root open/close and `WebTicketError` causes (including
/// `web-ticket-settings` when the saved v50 lifetime is absent).
/// Limits: stdout carries a single-use secret; callers must hand it to the
/// browser opener only and never log it. Grants only the owner key's access.
pub(crate) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if !(2..=3).contains(&a.pos.len())
        || !a.json
        || !a.input.is_empty()
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.server
    {
        return Err(CliError::Usage);
    }
    let address: SocketAddr = a.pos[1]
        .parse()
        .ok()
        .filter(|s: &SocketAddr| s.ip().is_loopback())
        .ok_or(CliError::WebTicket(Box::new(swarm_engine::WebTicketError::Session(
            swarm_engine::WebSessionError::Refused("web-ticket-address-invalid"),
        ))))?;
    let next = a.pos.get(2).map(String::as_str);
    if next.is_some_and(|p| {
        !p.starts_with('/') || p.starts_with("//") || p.contains('\\') || p.chars().any(char::is_control)
    }) {
        return Err(CliError::Usage);
    }
    let store = swarm_engine::RootStore::open(Path::new(&a.root), true)
        .map_err(|e| CliError::Root(Box::new(e)))?;
    let issued = store.issue_web_login_ticket(address);
    let closed = store.close();
    let ticket = issued.map_err(|e| CliError::WebTicket(Box::new(e)))?;
    closed.map_err(|e| CliError::Root(Box::new(e)))?;
    let mut url = format!("http://{address}/login?ticket={}", ticket.token());
    if let Some(next) = next {
        url.push_str("&next=");
        for b in next.bytes() {
            // RFC 3986 unreserved bytes and '/' stay literal; all others are %XX.
            if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
                url.push(char::from(b));
            } else {
                url.push_str(&format!("%{b:02X}"));
            }
        }
    }
    let response = Response {
        schema_version: 1,
        url,
        expires_at: ticket.expires_at(),
        next,
    };
    write_output(out, &json_bytes(&response).map_err(CliError::Output)?)?;
    Ok(0)
}

/// Rotate the owner key of ADDRESS and print
/// `{schema_version, address, rotated: true, restart_required: true}`.
///
/// Preconditions: exactly ADDRESS (loopback `IP:PORT`), `--json`, no other
/// option; the server for ADDRESS must be stopped (a bound listener refuses
/// with `web-rotate-server-running` before any write).
/// Effects: atomically replaces `.swarm/web-session-<hash>`; after the next
/// start every old cookie and unconsumed ticket is refused. The new key is
/// never printed. Errors: usage, address, Root open/close and storage causes.
pub(crate) fn execute_rotate(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 2 || !a.json || !a.input.is_empty() || !a.output.is_empty() || !a.task.is_empty() || a.server {
        return Err(CliError::Usage);
    }
    let address: SocketAddr = a.pos[1]
        .parse()
        .ok()
        .filter(|s: &SocketAddr| s.ip().is_loopback())
        .ok_or(CliError::WebTicket(Box::new(swarm_engine::WebTicketError::Session(
            swarm_engine::WebSessionError::Refused("web-ticket-address-invalid"),
        ))))?;
    let store = swarm_engine::RootStore::open(Path::new(&a.root), true)
        .map_err(|e| CliError::Root(Box::new(e)))?;
    let rotated = store.rotate_web_session(address);
    let closed = store.close();
    rotated.map_err(|e| CliError::WebTicket(Box::new(swarm_engine::WebTicketError::Session(e))))?;
    closed.map_err(|e| CliError::Root(Box::new(e)))?;
    let body = serde_json::json!({"schema_version": 1, "address": address.to_string(), "rotated": true, "restart_required": true});
    write_output(out, &json_bytes(&body).map_err(CliError::Output)?)?;
    Ok(0)
}

/// `web-authority-witness reset --json`: after a deliberate restore of the
/// authority daemon state, remove the Root rollback witness (EP-06) so the
/// next assertion re-establishes it; the reset is recorded in the Root
/// attribution ledger under the local operator identity. Run with the
/// engine stopped (`./swarm.sh authority accept-restore` does). Errors keep
/// their stable code (`web-assertion-witness-io`, `web-attribution-unavailable`).
pub(crate) fn execute_witness_reset(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 2 || a.pos[1] != "reset" || !a.json || !a.input.is_empty() || !a.output.is_empty() || !a.task.is_empty() || a.server {
        return Err(CliError::Usage);
    }
    let refused = |code: &'static str| CliError::WebTicket(Box::new(swarm_engine::WebTicketError::Session(swarm_engine::WebSessionError::Refused(code))));
    let actor = swarm_engine::operator_identity().map_err(|_| refused("web-assertion-witness-io"))?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let existed = swarm_web::reset_witness(Path::new(&a.root), &actor, now).map_err(refused)?;
    let body = serde_json::json!({"schema_version": 1, "witness_reset": true, "existed": existed, "restart_required": true});
    write_output(out, &json_bytes(&body).map_err(CliError::Output)?)?;
    Ok(0)
}
