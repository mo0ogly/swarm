//! Incremental Claude/Codex text output, without granting tool or model authority.
use crate::{
    agent_provider_facts::ProviderFacts,
    model_wire::{array, object, Boundaries},
    AgentGuardError as E, Usage,
};
use std::collections::BTreeMap;

/// Private-worker output accumulator. Limits are supplied from saved Admin policy;
/// None means no operator limit. Bytes, including ignored diagnostics and line
/// delimiters, count toward the response limit. No provider text is logged here.
/// Parsing alone establishes neither successful process exit nor confinement.
#[derive(Default)]
pub struct PreparationCliOutput {
    line: Vec<u8>,
    total: usize,
    response_limit: Option<usize>,
    event_limit: Option<usize>,
    reply: String,
    terminal: bool,
    facts: ProviderFacts,
}
impl PreparationCliOutput {
    /// Start an empty observation using already validated persisted byte limits.
    /// No allocation proportional to a configured maximum and no IO occurs.
    pub fn new(response_limit: Option<usize>, event_limit: Option<usize>) -> Self {
        Self {
            response_limit,
            event_limit,
            ..Self::default()
        }
    }
    /// Consume arbitrary stdout chunks, enforcing both limits before buffering.
    /// Malformed JSON-looking events refuse with a retained codec cause; ordinary
    /// CLI banner lines are ignored. Caller must abandon this parser after Err.
    pub fn push(&mut self, bytes: &[u8]) -> Result<(), E> {
        let total = self
            .total
            .checked_add(bytes.len())
            .ok_or_else(|| E::refused("preparation-cli-byte-overflow"))?;
        if self.response_limit.is_some_and(|n| total > n) {
            return Err(E::refused("preparation-cli-response-over-limit"));
        }
        self.total = total;
        for &byte in bytes {
            if byte == b'\n' {
                self.event()?;
                self.line.clear();
            } else {
                if self.event_limit.is_some_and(|n| self.line.len() >= n) {
                    return Err(E::refused("preparation-cli-event-over-limit"));
                }
                self.line.push(byte);
            }
        }
        Ok(())
    }
    /// Decode one complete line through the iterative engine JSON boundary parser.
    /// Latest complete reply wins, as in Go; structured Claude results override
    /// their text field. Provider error envelopes refuse without exposing messages.
    fn event(&mut self) -> Result<(), E> {
        let raw = self.line.trim_ascii();
        if raw.is_empty() || raw[0] != b'{' {
            return Ok(());
        }
        swarm_wire_json::decode::<serde::de::IgnoredAny>(raw)
            .map_err(|e| E::caused("preparation-cli-json", e))?;
        let boundaries =
            Boundaries::new(raw).map_err(|e| E::caused("preparation-cli-boundaries", e))?;
        let top: BTreeMap<String, &[u8]> = object(raw, &boundaries)
            .map_err(|e| E::caused("preparation-cli-object", e))?
            .into_iter()
            .collect();
        let string = |v: Option<&&[u8]>| -> Result<String, E> {
            v.map(|raw| {
                swarm_wire_json::decode::<String>(raw)
                    .map_err(|e| E::caused("preparation-cli-string", e))
            })
            .transpose()
            .map(Option::unwrap_or_default)
        };
        let kind = string(top.get("type"))?;
        reject_tools(&top, &kind, &boundaries)?;
        let is_error = top
            .get("is_error")
            .map(|v| {
                swarm_wire_json::decode::<bool>(v)
                    .map_err(|e| E::caused("preparation-cli-error-flag", e))
            })
            .transpose()?
            .unwrap_or(false);
        if matches!(kind.as_str(), "error" | "turn.failed") || (kind == "result" && is_error) {
            return Err(E::refused("preparation-cli-provider-failed"));
        }
        if matches!(kind.as_str(), "result" | "turn.completed") {
            self.terminal = true;
        }
        if kind == "result" || string(top.get("subtype"))? == "success" {
            let text = string(top.get("result"))?;
            if !text.is_empty() {
                self.reply = text;
            }
        }
        if kind == "item.completed" {
            if let Some(item) = top.get("item") {
                let item: BTreeMap<String, &[u8]> = object(item, &boundaries)
                    .map_err(|e| E::caused("preparation-cli-item", e))?
                    .into_iter()
                    .collect();
                if string(item.get("type"))? == "agent_message" {
                    let text = string(item.get("text"))?;
                    if !text.is_empty() {
                        self.reply = text;
                    }
                }
            }
        }
        if kind == "result" {
            if let Some(structured) = top.get("structured_output") {
                if structured.first() == Some(&b'{') {
                    self.reply = std::str::from_utf8(structured)
                        .map_err(|e| E::caused("preparation-cli-utf8", e))?
                        .into();
                }
            }
        }
        // No timestamp is invented: the turn settlement owns its observation time.
        self.facts.observe(&top, &kind, &boundaries, "")?;
        Ok(())
    }
    /// Consume final unterminated line, requiring a terminal event and nonempty
    /// reply. EOF/exit alone is not completion. Unknown/invalid usage stays None;
    /// the caller separately checks exit status, ownership and current admission.
    pub fn finish(mut self) -> Result<(String, Option<Usage>), E> {
        if !self.line.is_empty() {
            self.event()?;
        }
        if !self.terminal {
            return Err(E::refused("preparation-cli-terminal-missing"));
        }
        if self.reply.trim().is_empty() {
            return Err(E::refused("preparation-cli-reply-missing"));
        }
        Ok((self.reply, self.facts.snapshot().usage))
    }
}
/// Inspect only protocol object types, never quoted tool names in assistant text.
/// Preparation has no tool authority: known Claude/Codex call/result declarations
/// refuse immediately. These protocol labels are not operator policy or proof of
/// effective OS confinement; unseen/unknown external effects remain unestablished.
fn reject_tools(top: &BTreeMap<String, &[u8]>, kind: &str, b: &Boundaries) -> Result<(), E> {
    /// Borrow fields from a validated object; preserve malformed-envelope causes.
    /// No IO or authority is created, and duplicate fields retain wire semantics.
    fn fields<'a>(raw: &'a [u8], b: &Boundaries) -> Result<BTreeMap<String, &'a [u8]>, E> {
        object(raw, b)
            .map(|v| v.into_iter().collect())
            .map_err(|e| E::caused("preparation-cli-tool-envelope", e))
    }
    /// Decode only the protocol type and refuse known tool call/result labels.
    /// Missing/unknown labels establish no absence of external actions. No IO.
    fn tool(block: &BTreeMap<String, &[u8]>) -> Result<(), E> {
        let kind = block
            .get("type")
            .map(|v| {
                swarm_wire_json::decode::<String>(v)
                    .map_err(|e| E::caused("preparation-cli-tool-type", e))
            })
            .transpose()?
            .unwrap_or_default();
        if matches!(
            kind.as_str(),
            "tool_use"
                | "tool_result"
                | "server_tool_use"
                | "command_execution"
                | "mcp_tool_call"
                | "web_search"
                | "file_change"
                | "function_call"
                | "tool_call"
        ) {
            return Err(E::refused("preparation-cli-tool-declaration-refused"));
        }
        Ok(())
    }
    tool(top)?;
    if matches!(kind, "item.started" | "item.updated" | "item.completed") {
        if let Some(item) = top.get("item") {
            tool(&fields(item, b)?)?;
        }
    }
    if matches!(kind, "assistant" | "user") {
        if let Some(message) = top.get("message") {
            let message = fields(message, b)?;
            if let Some(content) = message.get("content") {
                for block in
                    array(content, b).map_err(|e| E::caused("preparation-cli-tool-content", e))?
                {
                    tool(&fields(block, b)?)?;
                }
            }
        }
    }
    if kind == "content_block_start" {
        if let Some(block) = top.get("content_block") {
            tool(&fields(block, b)?)?;
        }
    }
    if kind == "stream_event" {
        if let Some(event) = top.get("event") {
            let event = fields(event, b)?;
            if let Some(block) = event.get("content_block") {
                tool(&fields(block, b)?)?;
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_declarations_refuse_but_quoted_text_remains_text() {
        for raw in [
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"x"}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"x"}]}}"#,
            r#"{"type":"item.started","item":{"type":"command_execution","id":"x"}}"#,
            r#"{"type":"item.completed","item":{"type":"mcp_tool_call","id":"x"}}"#,
            r#"{"type":"stream_event","event":{"type":"content_block_start","content_block":{"type":"tool_use"}}}"#,
        ] {
            let mut p = PreparationCliOutput::new(None, None);
            let line = format!("{raw}\n");
            let error = p.push(line.as_bytes()).unwrap_err();
            assert_eq!(error.code(), "preparation-cli-tool-declaration-refused");
        }
        let mut p = PreparationCliOutput::new(None, None);
        p.push(br#"{"type":"result","result":"quoted tool_use command_execution"}"#)
            .unwrap();
        assert_eq!(p.finish().unwrap().0, "quoted tool_use command_execution");
    }
    #[test]
    fn claude_and_codex_split_streams_preserve_reply_and_unknown_usage() {
        for raw in [r#"{"type":"result","result":"text","structured_output":{"message":"bonjour"},"usage":{"input_tokens":2,"output_tokens":3}}"#,
                    "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"hello\"}}\n{\"type\":\"turn.completed\"}"] {
            let mut p=PreparationCliOutput::new(None,None);
            for chunk in raw.as_bytes().chunks(3){p.push(chunk).unwrap();}
            let (text,usage)=p.finish().unwrap();
            if text=="hello"{assert!(usage.is_none());}else{assert_eq!(text,r#"{"message":"bonjour"}"#);assert_eq!(usage.unwrap().input_tokens,2);}
        }
    }
    #[test]
    fn rejects_incomplete_error_malformed_and_empty_success() {
        for raw in [
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"premature"}}"#,
            r#"{"type":"result","result":"ok","is_error":true}"#,
            r#"{"type":"turn.failed","message":"secret"}"#,
            r#"{"type":"result"}"#,
            r#"{"type":"result""#,
        ] {
            let mut p = PreparationCliOutput::new(None, None);
            let result = p.push(raw.as_bytes()).and_then(|_| p.finish().map(|_| ()));
            assert!(result.is_err(), "accepted {raw}");
            assert!(!format!("{:?}", result.err().unwrap()).contains("secret"));
        }
    }
    #[test]
    fn saved_limits_count_ignored_bytes_and_refuse_before_growth() {
        let mut p = PreparationCliOutput::new(Some(3), None);
        assert!(p.push(b"abcd").is_err());
        assert!(p.line.is_empty());
        let mut p = PreparationCliOutput::new(None, Some(3));
        p.push(b"abc").unwrap();
        assert!(p.push(b"d").is_err());
        assert_eq!(p.line.len(), 3);
        let mut p = PreparationCliOutput::new(Some(4), Some(3));
        p.push(b"abc\n").unwrap();
        assert!(p.push(b"x").is_err());
    }
    #[test]
    fn bad_latest_usage_does_not_replay_previous_measurement() {
        let mut p = PreparationCliOutput::new(None, None);
        p.push(b"{\"type\":\"result\",\"result\":\"ok\",\"usage\":{\"input_tokens\":0,\"output_tokens\":0}}\n").unwrap();
        p.push(b"{\"type\":\"result\",\"result\":\"ok\",\"usage\":{\"input_tokens\":-1,\"output_tokens\":1}}\n").unwrap();
        assert!(p.finish().unwrap().1.is_none());
    }
}
