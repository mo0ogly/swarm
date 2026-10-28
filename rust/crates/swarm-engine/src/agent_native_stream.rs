//! Incremental provider declarations. Stream text grants no tool or actor rights.
use crate::{
    agent_terminal_text::terminal_text,
    model_wire::{array, object, Boundaries},
    Agent, AgentExecutionPolicy, AgentGuardError, AgentGuardObservation, AgentGuardVisibility,
    AgentToolGuard,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    time::Instant,
};
/// Retained facts remain distinct from truncation, malformed declarations and
/// process execution. No raw tool input/reasoning becomes a public message.
pub(crate) struct NativeStream {
    guard: AgentToolGuard,
    facts: crate::agent_provider_facts::ProviderFacts,
    policy: AgentExecutionPolicy,
    clock: Instant,
    line: Vec<u8>,
    dropping: bool,
    pub output_bytes: u64,
    hash: Sha256,
    capture: Vec<u8>,
    capture_lines: u64,
    capture_open_line: bool,
    capture_enabled: bool,
    pub capture_truncated: bool,
    pub public_messages: Vec<String>,
    public_bytes: u64,
    pub public_truncated: bool,
    codex_items: BTreeMap<String, String>,
    pub first_event_error: Option<AgentGuardError>,
}
/// Preserve codec/clock/guard causes through the process sink; no raw event is
/// included in the error's display. Stream refusal triggers owned cleanup.
fn io_error(e: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::other(e)
}
/// Borrow only immediate object members. Earlier duplicates were validated too;
/// last-key-wins follows Go's actual event-map decoder, never recursive Value.
fn fields<'a>(
    raw: &'a [u8],
    b: &Boundaries,
) -> Result<BTreeMap<String, &'a [u8]>, AgentGuardError> {
    object(raw.trim_ascii(), b)
        .map(|v| v.into_iter().collect())
        .map_err(|e| AgentGuardError::caused("agent-event-object", e))
}
/// Require a named textual event field; absent/wrong-type is lost attribution.
fn string(m: &BTreeMap<String, &[u8]>, k: &str) -> Result<String, AgentGuardError> {
    swarm_wire_json::decode(
        m.get(k)
            .copied()
            .ok_or_else(|| AgentGuardError::refused("agent-event-field-missing"))?,
    )
    .map_err(|e| AgentGuardError::caused("agent-event-field-invalid", e))
}
/// Nullable operator caps were validated before stream construction. Compare in
/// u128 so host allocation width never silently becomes another policy ceiling.
fn fits(cap: crate::SettingCap, value: u128) -> bool {
    cap.0.is_none_or(|n| value <= n.0 as u128)
}
impl NativeStream {
    /// Construct from complete frozen metadata and supplied policy/clock; absent
    /// guards or invalid caps refuse, without defaults or a provider launch.
    pub(crate) fn new(
        agent: &Agent,
        policy: AgentExecutionPolicy,
        clock: Instant,
    ) -> Result<Self, AgentGuardError> {
        policy
            .validate()
            .map_err(|e| AgentGuardError::caused(e.reason(), e))?;
        let snapshot = agent
            .native_tool_guard_policy
            .as_deref()
            .ok_or_else(|| AgentGuardError::refused("agent-guard-snapshot-missing"))?;
        Ok(Self {
            guard: AgentToolGuard::new(agent.limits.clone(), snapshot)?,
            facts: crate::agent_provider_facts::ProviderFacts::default(),
            policy,
            clock,
            line: vec![],
            dropping: false,
            output_bytes: 0,
            hash: Sha256::new(),
            capture: vec![],
            capture_lines: 0,
            capture_open_line: false,
            capture_enabled: agent.capture_output,
            capture_truncated: false,
            public_messages: vec![],
            public_bytes: 0,
            public_truncated: false,
            codex_items: BTreeMap::new(),
            first_event_error: None,
        })
    }
    /// Monotonic elapsed milliseconds, checked against the guard's i64 protocol.
    /// Representation overflow refuses; no saturation or hidden duration policy.
    pub(crate) fn at(&self) -> Result<i64, AgentGuardError> {
        i64::try_from(self.clock.elapsed().as_millis())
            .map_err(|e| AgentGuardError::caused("agent-stream-clock-representation", e))
    }
    /// Current cumulative observations never imply provider vitality or task
    /// acceptance. Silence and waiting remain observations, not stop reasons.
    pub(crate) fn observation(&self) -> Result<AgentGuardObservation, AgentGuardError> {
        self.guard.observation(self.at()?)
    }
    /// SHA256 of every observed byte, including content omitted by retention.
    pub(crate) fn sha256(&self) -> String {
        format!("{:x}", self.hash.clone().finalize())
    }
    /// Return selected raw capture cleaned for terminal display. The flag reports
    /// invalid UTF-8 replacement; byte caps refer to source bytes, not this text.
    pub(crate) fn captured(&self) -> (String, bool) {
        let s = String::from_utf8_lossy(&self.capture);
        (terminal_text(&s), matches!(s, std::borrow::Cow::Owned(_)))
    }
    /// Decode only explicit public assistant text. UTF-8 boundaries and aggregate
    /// message cap do not reset tool facts; omitted content is visibly flagged.
    fn public(&mut self, text: String) {
        let text = terminal_text(&text);
        let mut end = text.len();
        if let Some(max) = self.policy.public_message_max_bytes.0 {
            let separator = u64::from(!self.public_messages.is_empty());
            let remaining = (max.0 as u64)
                .saturating_sub(self.public_bytes)
                .saturating_sub(separator);
            end = end.min(usize::try_from(remaining).unwrap_or(usize::MAX));
        }
        if end < text.len() {
            self.public_truncated = true;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
        }
        if end > 0 {
            self.public_bytes += end as u64 + u64::from(!self.public_messages.is_empty());
        }
        if end > 0 {
            self.public_messages.push(text[..end].into());
        }
    }
    /// Incomplete/malformed attribution degrades visibility while retaining
    /// counters and a previously latched stop. Provider channels lose canonical
    /// visibility but keep last-valid history. No tool success is fabricated.
    fn loss(&mut self, at: i64, cause: AgentGuardVisibility) -> Result<(), AgentGuardError> {
        self.facts.incomplete();
        self.guard.lose_visibility(at, cause)
    }
    /// Apply one complete validated provider declaration. Completion-only file/
    /// search reports may introduce a call; a prior started input is frozen and
    /// is not replaced by legitimate completion changes under the same item ID.
    /// Engine-generated seen_at timestamps declarations independently of supplied
    /// provider clocks. Metadata refusals do not prevent valid tool attribution.
    fn event(&mut self, raw: &[u8], at: i64, seen_at: &str) -> Result<(), AgentGuardError> {
        let b = crate::agent_tool_guard::validated_json(raw)?;
        let top = fields(raw, &b)?;
        let kind = crate::agent_provider_facts::event_kind(&top)?;
        self.facts.observe(&top, &kind, &b, seen_at)?;
        if kind == "assistant" || kind == "user" {
            let message = fields(
                top.get("message")
                    .copied()
                    .ok_or_else(|| AgentGuardError::refused("agent-event-message-missing"))?,
                &b,
            )?;
            let blocks = array(
                message
                    .get("content")
                    .copied()
                    .ok_or_else(|| AgentGuardError::refused("agent-event-content-missing"))?,
                &b,
            )
            .map_err(|e| AgentGuardError::caused("agent-event-content-invalid", e))?;
            for block in blocks {
                let block = fields(block, &b)?;
                match string(&block, "type")?.as_str() {
                    "tool_use" => {
                        let id = string(&block, "id")?;
                        let name = string(&block, "name")?;
                        let input = block.get("input").copied().ok_or_else(|| {
                            AgentGuardError::refused("agent-event-tool-input-missing")
                        })?;
                        self.guard.call(at, &id, &name, input)?;
                    }
                    "tool_result" => {
                        let id = string(&block, "tool_use_id")?;
                        // Claude's existing stream protocol omits is_error on
                        // ordinary success. Wrong supplied type still refuses.
                        let failed = match block.get("is_error") {
                            Some(v) => swarm_wire_json::decode::<bool>(v).map_err(|e| {
                                AgentGuardError::caused("agent-event-result-invalid", e)
                            })?,
                            None => false,
                        };
                        self.guard.result(at, &id, failed)?;
                    }
                    "text" if kind == "assistant" => self.public(string(&block, "text")?),
                    "text" | "thinking" | "redacted_thinking" => {}
                    _ => self.loss(at, AgentGuardVisibility::MissingEvents)?,
                }
            }
        } else if kind == "item.started" || kind == "item.completed" {
            let item = fields(
                top.get("item")
                    .copied()
                    .ok_or_else(|| AgentGuardError::refused("agent-event-item-missing"))?,
                &b,
            )?;
            let typ = string(&item, "type")?;
            if typ == "agent_message" && kind == "item.completed" {
                self.public(string(&item, "text")?);
                return Ok(());
            }
            if matches!(
                typ.as_str(),
                "file_change" | "web_search" | "command_execution" | "mcp_tool_call"
            ) {
                let id = string(&item, "id")?;
                if self.codex_items.get(&id).is_some_and(|t| *t != typ) {
                    return Err(AgentGuardError::refused("agent-event-item-type-conflict"));
                }
                let start = kind == "item.started";
                if start
                    || (!self.codex_items.contains_key(&id)
                        && matches!(typ.as_str(), "file_change" | "web_search"))
                {
                    let key = match typ.as_str() {
                        "file_change" => "changes",
                        "web_search" => "query",
                        "command_execution" => "command",
                        _ => "arguments",
                    };
                    let input = item.get(key).copied().ok_or_else(|| {
                        AgentGuardError::refused("agent-event-tool-input-missing")
                    })?;
                    if typ == "mcp_tool_call" {
                        let server = string(&item, "server")?;
                        let tool = string(&item, "tool")?;
                        let mut combined = serde_json::to_vec(&[server, tool]).map_err(|e| {
                            AgentGuardError::caused("agent-event-input-encoding", e)
                        })?;
                        combined.pop();
                        combined.push(b',');
                        combined.extend_from_slice(input);
                        combined.push(b']');
                        self.guard.call(at, &id, &typ, &combined)?;
                    } else {
                        self.guard.call(at, &id, &typ, input)?;
                    }
                    self.codex_items.insert(id.clone(), typ.clone());
                }
                if !start {
                    let status = string(&item, "status")?;
                    if !matches!(status.as_str(), "completed" | "failed") {
                        return Err(AgentGuardError::refused(
                            "agent-event-result-status-invalid",
                        ));
                    }
                    let mut failed = status == "failed";
                    if let Some(code) = item.get("exit_code") {
                        if *code != b"null" {
                            failed |= swarm_wire_json::decode::<crate::GoFloat>(code)
                                .map_err(|e| {
                                    AgentGuardError::caused("agent-event-result-code-invalid", e)
                                })?
                                .0
                                != 0.0;
                        }
                    }
                    failed |= item.get("error").is_some_and(|v| *v != b"null");
                    self.guard.result(at, &id, failed)?;
                }
            } else if !matches!(typ.as_str(), "reasoning" | "agent_message") {
                self.loss(at, AgentGuardVisibility::MissingEvents)?;
            }
        } else if !matches!(
            kind.as_str(),
            "system"
                | "result"
                | "terminal_result"
                | "thread.started"
                | "turn.started"
                | "turn.completed"
                | "turn.failed"
        ) {
            self.loss(at, AgentGuardVisibility::MissingEvents)?;
        }
        Ok(())
    }
    /// Borrow-independent snapshot of current provider declarations and quality.
    /// Only complete valid metadata is canonical; neither model nor cost is
    /// authenticated. First typed refusals remain in the stream, safe codes in
    /// the projection. No IO, financial mutation or tool-state reset occurs.
    pub(crate) fn provider_facts(&self) -> crate::agent_provider_facts::ProviderFactsSnapshot {
        self.facts.snapshot()
    }
    /// Record stream failure without treating an unfinished line as an EOF
    /// event. Facts and a latched stop survive; visibility becomes unknown.
    /// Caller must retain the active lease until recovery is actually proven.
    pub(crate) fn incomplete(&mut self) -> io::Result<()> {
        self.loss(
            self.at().map_err(io_error)?,
            AgentGuardVisibility::StreamFailure,
        )
        .map_err(io_error)
    }
    /// Finish a trailing non-newline record after actual EOF. Truncated events
    /// remain degraded; no artificial newline changes output hash or byte count.
    pub(crate) fn finish(&mut self) -> io::Result<()> {
        if !self.line.is_empty() || self.dropping {
            self.line_complete()?;
        }
        Ok(())
    }
    /// Consume a frame or retain a safe visibility gap. A provider's malformed
    /// data is not a driver failure: drain continues, cumulative guards persist.
    /// An engine clock failure is instead typed IO and propagates to owned
    /// cleanup; no provider timestamp or stale timestamp is used as fallback.
    fn line_complete(&mut self) -> io::Result<()> {
        let raw = std::mem::take(&mut self.line);
        let at = self.at().map_err(io_error)?;
        if self.dropping {
            self.loss(at, AgentGuardVisibility::MissingEvents)
                .map_err(io_error)?;
        } else if !raw.trim_ascii().is_empty() {
            let seen_at = crate::system_timestamp().map_err(|e| {
                io_error(AgentGuardError::caused(
                    "agent-provider-observation-clock",
                    e,
                ))
            })?;
            if let Err(error) = self.event(raw.trim_ascii(), at, &seen_at) {
                if self.first_event_error.is_none() {
                    self.first_event_error = Some(error);
                }
                self.loss(at, AgentGuardVisibility::MalformedEvent)
                    .map_err(io_error)?;
            }
        }
        self.dropping = false;
        Ok(())
    }
}
impl Write for NativeStream {
    /// Drain every supplied byte. Frame/capture caps are independent and never
    /// bound process duration/output or clear errors. IO/clock overflow fails
    /// explicitly; raw malformed events only degrade observable attribution.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        let at = self.at().map_err(io_error)?;
        self.guard.output(at).map_err(io_error)?;
        self.output_bytes = self
            .output_bytes
            .checked_add(u64::try_from(raw.len()).map_err(io_error)?)
            .ok_or_else(|| io::Error::other("agent-output-counter-representation"))?;
        self.hash.update(raw);
        for &byte in raw {
            if self.capture_enabled {
                if !self.capture_open_line {
                    self.capture_lines = self
                        .capture_lines
                        .checked_add(1)
                        .ok_or_else(|| io::Error::other("agent-capture-counter-representation"))?;
                    self.capture_open_line = true;
                }
                if fits(
                    self.policy.capture_max_bytes,
                    self.capture.len() as u128 + 1,
                ) && fits(
                    self.policy.capture_max_lines,
                    u128::from(self.capture_lines),
                ) {
                    self.capture.push(byte);
                } else {
                    self.capture_truncated = true;
                }
                if byte == b'\n' {
                    self.capture_open_line = false;
                }
            }
            if byte == b'\n' {
                self.line_complete()?;
            } else if !self.dropping {
                if fits(self.policy.event_max_bytes, self.line.len() as u128 + 1) {
                    self.line.push(byte);
                } else {
                    self.line.clear();
                    self.dropping = true;
                    self.loss(
                        self.at().map_err(io_error)?,
                        AgentGuardVisibility::MissingEvents,
                    )
                    .map_err(io_error)?;
                }
            }
        }
        Ok(raw.len())
    }
    /// No external buffer or descriptor belongs to this sink.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentGuardSnapshot, AgentGuardStop, RunLimits, SettingCap, SettingCount};
    /// Explicit fixture policies are not installation defaults or a fallback.
    fn stream() -> NativeStream {
        let a = Agent {
            capture_output: true,
            limits: RunLimits {
                silence_seconds: 180,
                tool_seconds: 300,
                max_tool_calls: 100,
                max_repeated_calls: 4,
                max_consecutive_errors: 3,
                observation_mode: 0,
            },
            native_tool_guard_policy: Some(Box::new(AgentGuardSnapshot {
                schema_version: 1,
                settings_revision: "1".into(),
                lookback_multiplier: SettingCap(Some(SettingCount(4))),
                extensions: BTreeMap::new(),
            })),
            ..Agent::default()
        };
        NativeStream::new(
            &a,
            AgentExecutionPolicy {
                poll_interval_ms: SettingCount(1),
                heartbeat_stale_ms: SettingCount(10),
                term_grace_ms: SettingCount(0),
                output_drain_ms: SettingCap(None),
                event_max_bytes: SettingCap(None),
                capture_max_bytes: SettingCap(None),
                capture_max_lines: SettingCap(None),
                public_message_max_bytes: SettingCap(None),
                log_message_max_bytes: SettingCap(None),
            },
            Instant::now(),
        )
        .unwrap()
    }
    /// Chunking and a trailing EOF frame preserve raw hashing while only explicit
    /// assistant text is public; user/tool/reasoning text never leaks there.
    #[test]
    fn stream_chunked_claude_tools_public_text_and_eof() {
        let mut s = stream();
        let raw=br#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"SECRET"},{"type":"text","text":"Hello"},{"type":"tool_use","id":"A","name":"Read","input":{"x":1}}]}}
{"type":"user","message":{"content":[{"type":"text","text":"PRIVATE"},{"type":"tool_result","tool_use_id":"A","content":"secret"}]}}"#;
        for part in raw.chunks(7) {
            s.write_all(part).unwrap();
        }
        s.finish().unwrap();
        let o = s.observation().unwrap();
        assert_eq!((o.calls, o.results, o.pending), (1, 1, 0));
        assert_eq!(o.visibility, None);
        assert_eq!(s.public_messages, vec!["Hello"]);
        assert_eq!(s.output_bytes, raw.len() as u64);
        assert_eq!(s.sha256(), format!("{:x}", Sha256::digest(raw)));
        assert!(!s.captured().1);
    }
    /// Changing file diffs at completion is normal provider progress, not a second
    /// invocation or permission to change the already attributed call signature.
    #[test]
    fn stream_codex_changed_completion_freezes_start_and_attributes_result() {
        let mut s = stream();
        s.write_all(br#"{"type":"item.started","item":{"type":"file_change","id":"A","changes":[]}}
{"type":"item.completed","item":{"type":"file_change","id":"A","changes":[{"path":"p"}],"status":"completed"}}
{"type":"item.completed","item":{"type":"web_search","id":"B","query":"q","status":"completed"}}
{"type":"item.completed","item":{"type":"agent_message","text":"Visible"}}
"#).unwrap();
        let o = s.observation().unwrap();
        assert_eq!((o.calls, o.results), (2, 2));
        assert_eq!(o.visibility, None);
        assert_eq!(s.public_messages, vec!["Visible"]);
    }
    /// Claimed technical/infrastructure wording never waives an observed failed
    /// tool. Provider failure text is neither a root infrastructure receipt nor
    /// authority to erase errors. The stop remains latched after later success.
    #[test]
    fn stream_tool_failures_latch_even_with_technical_wording() {
        let mut s = stream();
        for id in ["A", "B", "C"] {
            s.write_all(format!("{{\"type\":\"assistant\",\"message\":{{\"content\":[{{\"type\":\"tool_use\",\"id\":\"{id}\",\"name\":\"Read\",\"input\":{{}}}}]}}}}\n{{\"type\":\"user\",\"message\":{{\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"{id}\",\"is_error\":true,\"content\":\"infrastructure permission denied\"}}]}}}}\n").as_bytes()).unwrap();
        }
        let o = s.observation().unwrap();
        assert_eq!(o.total_errors, 3);
        assert_eq!(o.stop_reason, Some(AgentGuardStop::ConsecutiveErrors));
    }
    /// Valid witness precedes invalid result and orphan completion. Invalid
    /// declarations degrade visibility and do not become empty successful tools.
    #[test]
    fn stream_invalid_attribution_is_unknown_without_fake_success() {
        let mut s = stream();
        s.write_all(b"{\"type\":\"thread.started\"}\n").unwrap();
        assert_eq!(s.observation().unwrap().visibility, None);
        s.write_all(br#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"missing","is_error":"false"}]}}
{"type":"item.completed","item":{"type":"command_execution","id":"missing","status":"completed","exit_code":0}}
"#).unwrap();
        let o = s.observation().unwrap();
        assert_eq!((o.calls, o.results), (0, 0));
        assert_eq!(o.tool_state, "unknown");
        assert_eq!(o.provider_vitality, "unknown");
    }
    /// Complete JSON validation sees an overflowing overwritten duplicate. Deep
    /// unknown data uses borrowed spans, so neither parse nor drop recurse.
    #[test]
    fn stream_overwritten_overflow_and_deep_unknown_values_are_safe() {
        let mut s = stream();
        let deep = format!(
            "{{\"type\":\"system\",\"future\":{}0{}}}\n",
            "[".repeat(512),
            "]".repeat(512)
        );
        s.write_all(deep.as_bytes()).unwrap();
        assert_eq!(s.observation().unwrap().visibility, None);
        s.write_all(b"{\"type\":\"system\",\"n\":1e400,\"n\":0}\n")
            .unwrap();
        assert_eq!(
            s.observation().unwrap().visibility,
            Some(AgentGuardVisibility::MalformedEvent)
        );
    }
    /// Zero event visibility/capture limits keep draining and hashing every byte;
    /// no output-volume or whole-run deadline is invented by the parser.
    #[test]
    fn stream_zero_caps_and_truncation_do_not_hide_observed_output() {
        let mut s = stream();
        s.policy.event_max_bytes = SettingCap(Some(SettingCount(0)));
        s.policy.capture_max_bytes = SettingCap(Some(SettingCount(0)));
        s.write_all(b"data\nmore").unwrap();
        s.finish().unwrap();
        assert_eq!(s.output_bytes, 9);
        assert!(s.capture_truncated);
        assert_eq!(s.captured().0, "");
        let o = s.observation().unwrap();
        assert_eq!(o.visibility, Some(AgentGuardVisibility::MissingEvents));
        assert_eq!(o.stop_reason, None);
    }
    /// Public byte limits honor UTF-8 boundaries, line caps retain exactly the
    /// selected raw lines, and terminal controls are not exposed as public text.
    #[test]
    fn stream_retention_is_explicit_and_utf8_safe() {
        let mut s = stream();
        s.policy.public_message_max_bytes = SettingCap(Some(SettingCount(3)));
        s.policy.capture_max_lines = SettingCap(Some(SettingCount(1)));
        s.write_all("{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"éé\\u202e\"}}\nmore\n".as_bytes()).unwrap();
        assert_eq!(s.public_messages, vec!["é"]);
        assert!(s.public_truncated);
        assert!(s.capture_truncated);
        assert!(!s.captured().0.contains("more"));
        assert_eq!(s.observation().unwrap().calls, 0);
    }
    /// Malformed model/usage clears only those channels. Independent complete
    /// tool declarations still count; zero raw capture does not suppress facts.
    #[test]
    fn stream_metadata_refusals_do_not_hide_tools_or_become_empty_success() {
        let mut s = stream();
        s.policy.capture_max_bytes = SettingCap(Some(SettingCount(0)));
        let raw=br#"{"type":"system","subtype":"init","model":"observed"}
{"type":"result","usage":{"input_tokens":7,"output_tokens":3}}
{"type":"assistant","message":{"model":{"PRIVATE":"secret"},"content":[{"type":"tool_use","id":"A","name":"Read","input":{"file":"proof"}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"A"}]}}
{"type":"result","usage":{"input_tokens":-1,"output_tokens":0}}
"#;
        s.write_all(raw).unwrap();
        let o = s.observation().unwrap();
        assert_eq!((o.calls, o.results), (1, 1));
        assert_eq!(o.visibility, None);
        assert!(s.first_event_error.is_none());
        assert_eq!(s.captured().0, "");
        assert!(s.capture_truncated);
        assert_eq!(s.output_bytes, raw.len() as u64);
        let f = serde_json::to_value(s.provider_facts()).unwrap();
        assert_eq!(f["usage_state"], "invalid");
        assert_eq!(f["model_state"], "invalid");
        assert!(f["usage"].is_null() && f["reported_model"].is_null());
        assert_eq!(f["last_valid_usage"]["input_tokens"], 7);
        assert!(!serde_json::to_string(&f).unwrap().contains("PRIVATE"));
    }
    /// A balanced trailing JSON frame is not EOF after failed output. Actual
    /// EOF may decode a legacy terminal result; cap gaps clear canonical data.
    #[test]
    fn stream_metadata_gaps_partial_lines_and_actual_eof_are_distinct() {
        let good = b"{\"type\":\"result\",\"usage\":{\"input_tokens\":7,\"output_tokens\":3}}\n";
        let trailing=br#"{"subtype":"success","terminal_reason":"completed","session_id":"declared","usage":{"input_tokens":0,"output_tokens":1}}"#;
        let mut s = stream();
        s.write_all(good).unwrap();
        s.write_all(trailing).unwrap();
        s.incomplete().unwrap();
        let f = s.provider_facts();
        assert!(f.usage.is_none());
        let j = serde_json::to_value(f).unwrap();
        assert_eq!(j["usage_state"], "incomplete");
        assert_eq!(j["last_valid_usage"]["input_tokens"], 7);
        let mut s = stream();
        s.write_all(good).unwrap();
        s.write_all(trailing).unwrap();
        s.finish().unwrap();
        let f = s.provider_facts();
        assert_eq!(f.usage.unwrap().input_tokens, 0);
        let mut s = stream();
        s.write_all(good).unwrap();
        s.policy.event_max_bytes = SettingCap(Some(SettingCount(0)));
        s.write_all(good).unwrap();
        let j = serde_json::to_value(s.provider_facts()).unwrap();
        assert_eq!(j["usage_state"], "incomplete");
        assert!(j["usage"].is_null());
        assert_eq!(j["last_valid_usage"]["input_tokens"], 7);
    }
}
