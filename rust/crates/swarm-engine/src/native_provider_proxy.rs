//! Native adapter inside the unchanged Linux boundary; no Root access or grant.
use std::{
    ffi::OsString,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
};

/// Reserved line framing; distinguishes RPC from ordinary provider JSON.
pub(crate) const FRAME_PREFIX: &[u8] = b"SWARM_NATIVE_TRUST_V1 ";

/// Fixed protocol instructions appended by the trusted gate only for an
/// explicitly mapped native execution. These describe transport, not authority.
pub(crate) const INSTRUCTIONS: &[u8] = b"\n\nNative protected actions: you have no direct access to Swarm authority. To propose one final Trust operation, emit a single assistant text consisting exactly of SWARM_TRUST_REQUEST followed by a space and a JSON object: {\"type\":\"swarm.trust.request\",\"schema_version\":1,\"id\":\"unique-call-id\",\"operation\":\"commit\",\"input\":{\"schema_version\":1,\"event_id\":\"unique-event-id\",\"action_id\":\"operator-supplied-action-id\"}}. Use only actual supplied action identifiers; never invent an authorization. Supported operations: request, commit, reconcile. This is only an untrusted intent: the supervisor separately checks current identity, exact Root/work/task/attempt, mandate, policy, approvals, budget and Biba before effect. No configuration or approval authority is granted. The native adapter finalizes after your process succeeds; its reply belongs to the supervisor audit and does not mean task acceptance. Ordinary I0 workspace tools remain confined by Biba. Write authority does not grant object read access: replies contain only a closed status acknowledgment. A configured BLP policy without admitted native egress refuses this transport before effect.\n";

/// Run the exact prepared backend as a descendant, preserving stdin/stdio and
/// the inherited Biba/seccomp boundary. After successful backend exit, keep the
/// registered native leader alive while the foreground host mediates the final
/// intent over the already inherited bidirectional stdout descriptor. No new
/// socket, Root open, bearer or descriptor exception is used. Direct native
/// protocol backends can also exchange frames on stdout before exiting.
///
/// Called by the private executable dispatch with absolute backend argv. Calling
/// it alone confers no authority: only the genuine parent channel can perform
/// a broker action. IO/spawn/wait/protocol/close failures return typed causes;
/// an error after publication never establishes rollback. Exit 1 denotes a
/// refused final operation, not successful task completion. No watchdog/retry.
#[cfg(target_os = "linux")]
pub fn execute_native_provider_proxy(args: &[OsString]) -> Result<u8, crate::RootStoreError> {
    let failure = |code, error| {
        crate::RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused(code, error)))
    };
    let Some(image) = args.first() else {
        return Err(crate::RootStoreError::Refused(
            "native-proxy-backend-missing",
        ));
    };
    if !Path::new(image).is_absolute() {
        return Err(crate::RootStoreError::Refused(
            "native-proxy-backend-relative",
        ));
    }
    let status = Command::new(image)
        .args(&args[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| failure("native-proxy-spawn", e))?
        .wait()
        .map_err(|e| failure("native-proxy-wait", e))?;
    if !status.success() {
        return Ok(1);
    }
    // stdout is the original full-duplex socket made by the foreground runner,
    // not a pipe opened by this adapter. A standalone ordinary stdout refuses.
    let file = File::from(
        rustix::io::dup(rustix::stdio::stdout())
            .map_err(|e| failure("native-proxy-channel", io::Error::from(e)))?,
    );
    let mut channel = BufReader::new(file);
    let operation = (|| {
        let nonce = crate::generate_identifier("finalize-").map_err(io::Error::other)?;
        let mut frame = serde_json::to_vec(&serde_json::json!({"type":"swarm.trust.finalize",
            "schema_version":1,"id":nonce}))
        .map_err(io::Error::other)?;
        frame.push(b'\n');
        channel.get_mut().write_all(FRAME_PREFIX)?;
        channel.get_mut().write_all(&frame)?;
        channel.get_mut().flush()?;
        let mut raw = Vec::new();
        if channel.read_until(b'\n', &mut raw)? == 0 {
            return Err(io::Error::other("native-proxy-reply-missing"));
        }
        let reply: serde_json::Value = swarm_wire_json::decode(&raw).map_err(io::Error::other)?;
        let allowed = final_reply(&reply, &nonce)?;
        Ok(if allowed { 0 } else { 1 })
    })();
    let close = nix::unistd::close(channel.into_inner())
        .map_err(|e| failure("native-proxy-close", io::Error::from(e)));
    crate::root_store::finished(
        operation.map_err(|e| failure("native-proxy-operation", e)),
        close,
    )
}

/// Require the exact fresh finalization nonce. An unread older business reply
/// cannot acknowledge a later final action. This verifies transport correlation,
/// not the truth of an untrusted standalone peer or task acceptance.
fn final_reply(reply: &serde_json::Value, nonce: &str) -> io::Result<bool> {
    if reply.get("type").and_then(|v| v.as_str()) != Some("swarm.trust.reply")
        || reply.get("schema_version").and_then(|v| v.as_i64()) != Some(1)
        || reply.get("id").and_then(|v| v.as_str()) != Some(nonce)
    {
        return Err(io::Error::other("native-proxy-reply-invalid"));
    }
    let ok = reply
        .get("ok")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| io::Error::other("native-proxy-reply-invalid"))?;
    if !ok {
        return Ok(false);
    }
    let result = reply
        .get("result")
        .and_then(|v| v.as_object())
        .ok_or_else(|| io::Error::other("native-proxy-reply-invalid"))?;
    if result.get("finished").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(true);
    }
    let state = result
        .get("state")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| io::Error::other("native-proxy-reply-invalid"))?;
    Ok(state != "denied")
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Start with a valid synthetic acknowledgment, then check stale/missing
    /// nonce errors, explicit refusal and missing-result errors. Assertions panic
    /// on mismatch; this proves no real peer identity, delivery or native effect.
    #[test]
    fn native_proxy_final_reply_rejects_stale_acknowledgment() {
        let mut reply = serde_json::json!({"type":"swarm.trust.reply","schema_version":1,
            "id":"fresh-nonce","ok":true,"result":{"finished":true}});
        assert!(final_reply(&reply, "fresh-nonce").unwrap());
        assert!(final_reply(&reply, "other-nonce").is_err());
        reply.as_object_mut().unwrap().remove("id");
        assert!(final_reply(&reply, "fresh-nonce").is_err());
        reply["id"] = "fresh-nonce".into();
        reply["ok"] = false.into();
        assert!(!final_reply(&reply, "fresh-nonce").unwrap());
        reply["ok"] = true.into();
        reply.as_object_mut().unwrap().remove("result");
        assert!(final_reply(&reply, "fresh-nonce").is_err());
    }
}

/// Safe projection for a failed proxy process. This is a declared diagnostic,
/// not pre-exec evidence or an authority source. Every known compound branch is
/// traversed by the shared projector; arbitrary error text/argv is never emitted.
pub fn native_proxy_diagnostic(error: &crate::RootStoreError) -> serde_json::Value {
    serde_json::json!({"type":"swarm.native.proxy-failure","schema_version":1,
        "causes_scope":"known-native-compound-v1","causes":crate::native_diagnostics::project(error)})
}
