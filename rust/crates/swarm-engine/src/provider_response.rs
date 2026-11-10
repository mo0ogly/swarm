//! Complete JSONL provider response transport shared by native domains.
//! Legacy planning-output diagnostic codes remain stable; no domain/exec authority.
use crate::model_wire::Boundaries;
use crate::{AgentLaunchError as E, EngineSettingsValues};
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

/// Exact terminal reply plus unverified provider announcements. Bytes may be
/// non-JSON or invalid for a domain: callers must strictly validate before use.
/// This is neither a process/identity receipt nor measured invoice/authority.
pub struct ProviderResponse {
    pub response: Vec<u8>,
    pub usage: Option<crate::Usage>,
    pub reported_model: Option<crate::ReportedModel>,
    pub provider_facts: crate::GoRaw,
}
/// Incremental JSONL input under explicit response and saved event policies.
/// No raw capture, truncation, retry, global duration or default is inferred.
/// Only a terminal result/turn completion can finalize complete response bytes.
/// Private state prevents constructing a successful terminal marker from JSON.
pub struct ProviderResponseStream {
    line: Vec<u8>,
    reply: Option<Vec<u8>>,
    reply_limit: Option<usize>,
    event_limit: Option<usize>,
    facts: crate::agent_provider_facts::ProviderFacts,
    terminal: bool,
    failed: bool,
    finished: bool,
    tool_free: bool,
}
/// Borrow complete immediate members without erasing duplicate keys or nested
/// proposal bytes. Duplicate/shape errors refuse with safe original causes.
fn fields<'a>(raw: &'a [u8], b: &Boundaries) -> Result<BTreeMap<String, &'a [u8]>, E> {
    let mut out = BTreeMap::new();
    for (key, value) in crate::model_wire::object(raw.trim_ascii(), b)
        .map_err(|e| E::caused("planning-output-object", e))?
    {
        if out.insert(key, value).is_some() {
            return Err(E::Refused("planning-output-duplicate-field"));
        }
    }
    Ok(out)
}
/// Strict required textual transport member. Null, missing and other types do
/// not become empty strings or authority; original codec causes stay attached.
fn text(m: &BTreeMap<String, &[u8]>, key: &str) -> Result<String, E> {
    let raw = m
        .get(key)
        .ok_or(E::Refused("planning-output-field-required"))?
        .trim_ascii();
    if raw.first() != Some(&b'"') {
        return Err(E::Refused("planning-output-field-type"));
    }
    swarm_wire_json::decode(raw).map_err(|e| E::caused("planning-output-field-format", e))
}
impl ProviderResponseStream {
    /// Finalize once after the owner observed EOF. Return exact complete response
    /// bytes and unverified facts; no domain/JSON validation, process completion,
    /// cost settlement or permission is implied. Missing/failed/ambiguous terminal
    /// and prior IO/limit failure refuse without a truncated successful response.
    pub fn finish(&mut self) -> Result<ProviderResponse, E> {
        self.finish_decoded(|_| Ok(()))
            .map(|(response, ())| response)
    }
    /// Freeze the validated saved event cap and an explicitly supplied response
    /// cap. Tool declarations are permitted metadata here; tool-free roles must
    /// use new_tool_free. The caller binds the cap to its persisted intention;
    /// None is explicit unlimited, not an inferred default. Refuse invalid values
    /// and event-cap representation. No IO, adoption, process or permission.
    pub fn new(values: &EngineSettingsValues, reply_limit: Option<usize>) -> Result<Self, E> {
        values.validate().map_err(|e| E::caused(e.reason(), e))?;
        let execution = values
            .agent_execution_policy()
            .map_err(|e| E::caused(e.reason(), e))?;
        let event_limit = execution
            .event_max_bytes
            .0
            .map(|n| {
                usize::try_from(n.0)
                    .map_err(|e| E::caused("planning-output-event-cap-representation", e))
            })
            .transpose()?;
        Ok(Self {
            line: Vec::new(),
            reply: None,
            reply_limit,
            event_limit,
            facts: Default::default(),
            terminal: false,
            failed: false,
            finished: false,
            tool_free: false,
        })
    }
    /// Require a tool-free response contract under the same saved framing caps.
    /// Known Claude/Codex tool declarations refuse and latch before a later reply
    /// can succeed. This is a role invariant, not operator policy or OS confinement:
    /// missing/unknown events cannot establish that no external tool executed.
    /// Validation/IO failure semantics are identical to new; no process is started.
    pub fn new_tool_free(
        values: &EngineSettingsValues,
        reply_limit: Option<usize>,
    ) -> Result<Self, E> {
        let mut stream = Self::new(values, reply_limit)?;
        stream.tool_free = true;
        Ok(stream)
    }
    /// Reject known tool declarations without parsing or displaying private input.
    /// Only tool-free callers use this gate. Malformed observed envelopes retain
    /// original framing causes; unknown kinds establish neither tools nor absence.
    fn require_tool_free(
        &self,
        top: &BTreeMap<String, &[u8]>,
        kind: &str,
        b: &Boundaries,
    ) -> Result<(), E> {
        if !self.tool_free {
            return Ok(());
        }
        let forbidden = match kind {
            "item.started" | "item.completed" => {
                let item = fields(
                    top.get("item")
                        .ok_or(E::Refused("planning-output-item-required"))?,
                    b,
                )?;
                matches!(
                    text(&item, "type")?.as_str(),
                    "command_execution" | "file_change" | "web_search" | "mcp_tool_call"
                )
            }
            "assistant" | "user" => {
                let message = fields(
                    top.get("message")
                        .ok_or(E::Refused("planning-output-message-required"))?,
                    b,
                )?;
                let content = message
                    .get("content")
                    .ok_or(E::Refused("planning-output-content-required"))?;
                let mut found = false;
                for raw in crate::model_wire::array(content, b)
                    .map_err(|e| E::caused("planning-output-content-array", e))?
                {
                    let block = fields(raw, b)?;
                    found |= matches!(text(&block, "type")?.as_str(), "tool_use" | "tool_result");
                }
                found
            }
            _ => false,
        };
        if forbidden {
            return Err(E::Refused("planning-output-tool-event-forbidden"));
        }
        Ok(())
    }
    /// Retain complete response bytes only within the explicit cap. Replacement
    /// is a newer message snapshot before terminal, never concatenated inference
    /// retries or truncation. A refusal leaves no successful finish path.
    fn candidate(&mut self, raw: &[u8]) -> Result<(), E> {
        if self.reply_limit.is_some_and(|cap| raw.len() > cap) {
            return Err(E::Refused("planning-response-byte-limit-exceeded"));
        }
        self.reply = Some(raw.to_vec());
        Ok(())
    }
    /// Process one complete strictly UTF8 event, retaining raw proposal bytes through the
    /// strict decoder later. Metadata is observed with an engine clock, not a
    /// provider timestamp. Known terminal failures refuse; unknown event kinds
    /// grant no response/completion. Every post-terminal event is ambiguous.
    fn event(&mut self, raw: &[u8]) -> Result<(), E> {
        // Provider protocol bytes must not be normalized into a valid proposal
        // by Go-compatible string replacement before the strict inner decoder.
        std::str::from_utf8(raw).map_err(|e| E::caused("planning-output-utf8", e))?;
        let raw = raw.trim_ascii();
        if raw.is_empty() {
            return Ok(());
        }
        if self.terminal {
            return Err(E::Refused("planning-output-after-terminal"));
        }
        let b = crate::agent_tool_guard::validated_json(raw)
            .map_err(|e| E::caused("planning-output-json", e))?;
        let top = fields(raw, &b)?;
        let kind = text(&top, "type")?;
        let at = crate::system_timestamp().map_err(|e| E::caused("planning-output-clock", e))?;
        self.facts
            .observe(&top, &kind, &b, &at)
            .map_err(|e| E::caused("planning-output-facts", e))?;
        if let Some(error) = top.get("error") {
            if error.trim_ascii() != b"null" {
                return Err(E::Refused("planning-output-provider-failed"));
            }
        }
        if let Some(raw) = top.get("is_error") {
            let failed: bool = serde_json::from_slice(raw)
                .map_err(|e| E::caused("planning-output-error-flag", e))?;
            if failed {
                return Err(E::Refused("planning-output-provider-failed"));
            }
        }
        self.require_tool_free(&top, &kind, &b)?;
        match kind.as_str() {
            "turn.failed" | "error" => return Err(E::Refused("planning-output-provider-failed")),
            "result" => {
                if top.contains_key("subtype")
                    && !matches!(text(&top, "subtype")?.as_str(), "" | "success")
                {
                    return Err(E::Refused("planning-output-provider-failed"));
                }
                if let Some(raw) = top.get("structured_output") {
                    if raw.trim_ascii().first() != Some(&b'{') {
                        return Err(E::Refused("planning-output-structured-object-required"));
                    }
                    self.candidate(raw)?;
                } else if top.contains_key("result") {
                    self.candidate(text(&top, "result")?.as_bytes())?;
                }
                self.terminal = true;
            }
            "turn.completed" => self.terminal = true,
            "item.completed" => {
                let item = fields(
                    top.get("item")
                        .ok_or(E::Refused("planning-output-item-required"))?,
                    &b,
                )?;
                if text(&item, "type")? == "agent_message" {
                    self.candidate(text(&item, "text")?.as_bytes())?;
                }
            }
            "assistant" => {
                let message = fields(
                    top.get("message")
                        .ok_or(E::Refused("planning-output-message-required"))?,
                    &b,
                )?;
                let content = message
                    .get("content")
                    .ok_or(E::Refused("planning-output-content-required"))?;
                let mut reply = Vec::new();
                for block in crate::model_wire::array(content, &b)
                    .map_err(|e| E::caused("planning-output-content-array", e))?
                {
                    let block = fields(block, &b)?;
                    if text(&block, "type")? == "text" {
                        let text = text(&block, "text")?;
                        let size = reply
                            .len()
                            .checked_add(text.len())
                            .ok_or(E::Refused("planning-response-size-representation"))?;
                        if self.reply_limit.is_some_and(|cap| size > cap) {
                            return Err(E::Refused("planning-response-byte-limit-exceeded"));
                        }
                        reply.extend_from_slice(text.as_bytes());
                    }
                }
                if !reply.is_empty() {
                    self.candidate(&reply)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    /// Clone safe provenance/quality observations for normalized native usage,
    /// including after write/terminal failure. No IO or serialization occurs.
    /// Missing/invalid/incomplete stays explicit; no cost or settlement inferred.
    pub(crate) fn facts_snapshot(&self) -> crate::agent_provider_facts::ProviderFactsSnapshot {
        self.facts.snapshot()
    }
    /// Latch incomplete transport after owner IO/drain failure, preserving prior
    /// valid declarations only as history. No later finish can fabricate success.
    pub(crate) fn incomplete_transport(&mut self) {
        self.failed = true;
        self.facts.incomplete();
    }
    /// Borrow the last complete response-message bytes already admitted by the
    /// response cap, including after a domain or owner-observed IO failure.
    /// This is not raw JSONL capture, a terminal marker, valid domain data or
    /// permission to publish. None means no retained candidate (including after
    /// successful ownership transfer in finish), never a successful empty reply.
    /// No IO, allocation, decoding, retry or authority change occurs. The native
    /// review journal preserves these bytes without promoting rejected output.
    pub(crate) fn retained_reply(&self) -> Option<&[u8]> {
        self.reply.as_deref()
    }
    /// Serialize current unverified facts without consuming response bytes or IO.
    /// Encoding errors retain causes; no measured invoice or authority inferred.
    pub fn provider_facts(&self) -> Result<crate::GoRaw, E> {
        serde_json::value::to_raw_value(&self.facts.snapshot())
            .map(crate::GoRaw)
            .map_err(|e| E::caused("planning-output-facts-encoding", e))
    }
    /// Finish exactly once after the owned stdout reaches EOF. A partial final
    /// JSON line, previous write failure, missing terminal or response refuses.
    /// Newline is optional when the last complete JSON event is present. The
    /// caller's decoder checks the complete original bytes before they move out;
    /// rejected bytes/facts remain in this object. Legacy error categories stay
    /// stable. No process, decision, usage settlement, authenticated model or
    /// acceptance is inferred.
    pub(crate) fn finish_decoded<T>(
        &mut self,
        decode: impl FnOnce(&[u8]) -> Result<T, E>,
    ) -> Result<(ProviderResponse, T), E> {
        if self.finished {
            return Err(E::Refused("planning-output-already-finished"));
        }
        self.finished = true;
        if self.failed {
            return Err(E::Refused("planning-output-stream-failed"));
        }
        if !self.line.is_empty() {
            let line = std::mem::take(&mut self.line);
            if let Err(e) = self.event(&line) {
                self.failed = true;
                self.facts.incomplete();
                return Err(e);
            }
        }
        if !self.terminal {
            self.facts.incomplete();
            return Err(E::Refused("planning-output-terminal-required"));
        }
        let raw = self
            .reply
            .as_deref()
            .ok_or(E::Refused("planning-output-response-required"))?;
        let decoded = decode(raw)?;
        let facts = self.facts.snapshot();
        // Transfer already bounded original bytes instead of duplicating the
        // complete response during receipt preparation.
        let response = self
            .reply
            .take()
            .ok_or(E::Refused("planning-output-response-required"))?;
        Ok((
            ProviderResponse {
                response,
                usage: facts.usage,
                reported_model: facts.reported_model,
                provider_facts: self.provider_facts()?,
            },
            decoded,
        ))
    }
}
impl ProviderResponseStream {
    /// Feed bytes with the original domain cause, latching any failure. Runtime
    /// Write and offline Root checks share this exact parser/cap consumer. No EOF
    /// or completion is inferred; a failed stream cannot be reused or finalized.
    pub fn feed(&mut self, raw: &[u8]) -> Result<(), E> {
        if self.failed || self.finished {
            return Err(E::Refused("planning-output-stream-unavailable"));
        }
        let result = (|| {
            for byte in raw {
                if *byte == b'\n' {
                    let line = std::mem::take(&mut self.line);
                    self.event(&line)?;
                } else {
                    let size = self
                        .line
                        .len()
                        .checked_add(1)
                        .ok_or(E::Refused("planning-output-size-representation"))?;
                    if self.event_limit.is_some_and(|cap| size > cap) {
                        return Err(E::Refused("planning-output-event-limit-exceeded"));
                    }
                    self.line.push(*byte);
                }
            }
            Ok(())
        })();
        result.inspect_err(|_| {
            self.failed = true;
            self.facts.incomplete();
        })
    }
}
impl Write for ProviderResponseStream {
    /// Feed the same complete parser with checked IO causes; the owned runner
    /// must retain failures while cleaning/draining. No implicit retry or EOF.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        self.feed(raw).map(|()| raw.len()).map_err(io::Error::other)
    }
    /// Memory flush is not EOF. A previous failure cannot become successful.
    fn flush(&mut self) -> io::Result<()> {
        if self.failed {
            Err(io::Error::other(E::Refused(
                "planning-output-stream-failed",
            )))
        } else {
            Ok(())
        }
    }
}
