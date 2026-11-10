//! Typed transcript of one finished attempt (LangWatch L2, TR-R1..R4).
//!
//! Derived at read time from the provider stream already retained in
//! `native_execution_result.capture` (bounded by `agent_capture_max_bytes`):
//! nothing new is persisted. The capture is the provider's JSON event stream
//! cleaned for terminal display (newlines became spaces), so values are read
//! back with a JSON stream decoder. Recognized shapes: Claude Code
//! `stream-json` (`system`, `assistant`, `user`, `result`) and Codex `exec
//! --json` (`thread.*`, `turn.*`, `item.*`, `error`). Provider is inferred from
//! event shapes, never from the provider's configured name.
//!
//! Content policy (TR-R2): assistant text, tool inputs and tool outputs are
//! never returned; only kinds, tool names, ids, error flags, byte lengths and
//! declared token counts. Unknown event types become a counted `note`
//! (TR-R4); the raw capture is not modified. A truncated capture ends with a
//! `note` marked `truncated`. Stream lines carry no timestamps, so entries are
//! in stream order without durations.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// One typed transcript entry. `kind` is one of `system_prompt` (session
/// start), `assistant_message`, `model_call`, `tool`, `tool_result`,
/// `tool_rejected`, `note`.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct TranscriptEntry {
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Declared failure (`is_error`, failed status), never inferred from text.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub error: bool,
    /// Byte length of the omitted content (text, tool input or output).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<usize>,
    /// Declared tokens for `model_call` (input, output, cache read/creation).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<&'static str, u64>,
    /// Short label: model, note reason or event type (no content).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Typed transcript plus per-kind counts and stream diagnostics.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Transcript {
    #[serde(skip)]
    tool_names: BTreeMap<String, String>,
    /// `claude`, `codex` or `unknown` (from shapes, first recognized event).
    pub format: &'static str,
    pub entries: Vec<TranscriptEntry>,
    pub counts: BTreeMap<&'static str, u64>,
    /// Unrecognized event types by name (also present as `note` entries).
    pub unknown: BTreeMap<String, u64>,
    /// The capture ended inside a value (retention cap or interrupted output).
    pub truncated: bool,
}

fn len(v: &Value) -> usize {
    match v {
        Value::String(s) => s.len(),
        Value::Null => 0,
        other => other.to_string().len(),
    }
}

fn tokens(usage: Option<&Value>, keys: &[(&'static str, &str)]) -> BTreeMap<&'static str, u64> {
    let mut out = BTreeMap::new();
    if let Some(u) = usage {
        for (name, key) in keys {
            if let Some(n) = u.get(*key).and_then(Value::as_u64) {
                out.insert(*name, n);
            }
        }
    }
    out
}

const CLAUDE_USAGE: &[(&str, &str)] = &[
    ("input", "input_tokens"),
    ("output", "output_tokens"),
    ("cache_read", "cache_read_input_tokens"),
    ("cache_creation", "cache_creation_input_tokens"),
];
const CODEX_USAGE: &[(&str, &str)] = &[
    ("input", "input_tokens"),
    ("output", "output_tokens"),
    ("cache_read", "cached_input_tokens"),
    ("reasoning", "reasoning_output_tokens"),
];

impl Transcript {
    fn push(&mut self, entry: TranscriptEntry) {
        *self.counts.entry(entry.kind).or_default() += 1;
        self.entries.push(entry);
    }
    fn note(&mut self, label: String) {
        self.push(TranscriptEntry { kind: "note", label: Some(label), ..Default::default() });
    }
    fn unknown(&mut self, name: &str) {
        *self.unknown.entry(name.to_owned()).or_default() += 1;
        self.note(format!("unrecognized:{name}"));
    }
    fn claim(&mut self, format: &'static str) {
        if self.format == "unknown" {
            self.format = format;
        }
    }

    fn claude(&mut self, kind: &str, event: &Value) {
        self.claim("claude");
        let message = event.get("message");
        match kind {
            "system" => self.push(TranscriptEntry {
                kind: "system_prompt",
                label: event.get("model").and_then(Value::as_str).map(str::to_owned),
                ..Default::default()
            }),
            "assistant" => {
                let usage = message.and_then(|m| m.get("usage"));
                if usage.is_some() {
                    self.push(TranscriptEntry {
                        kind: "model_call",
                        label: message.and_then(|m| m.get("model")).and_then(Value::as_str).map(str::to_owned),
                        tokens: tokens(usage, CLAUDE_USAGE),
                        ..Default::default()
                    });
                }
                for part in message.and_then(|m| m.get("content")).and_then(Value::as_array).into_iter().flatten() {
                    match part.get("type").and_then(Value::as_str) {
                        Some("text") => self.push(TranscriptEntry {
                            kind: "assistant_message",
                            bytes: part.get("text").map(len),
                            ..Default::default()
                        }),
                        Some("tool_use") => {
                            let tool = part.get("name").and_then(Value::as_str).map(str::to_owned);
                            let id = part.get("id").and_then(Value::as_str).map(str::to_owned);
                            if let (Some(id), Some(tool)) = (&id, &tool) {
                                self.tool_names.insert(id.clone(), tool.clone());
                            }
                            self.push(TranscriptEntry { kind: "tool", tool, id, bytes: part.get("input").map(len), ..Default::default() })
                        }
                        Some("thinking") => {}
                        Some(other) => self.unknown(&format!("assistant.{other}")),
                        None => self.unknown("assistant.part"),
                    }
                }
            }
            "user" => {
                for part in message.and_then(|m| m.get("content")).and_then(Value::as_array).into_iter().flatten() {
                    if part.get("type").and_then(Value::as_str) == Some("tool_result") {
                        let id = part.get("tool_use_id").and_then(Value::as_str).map(str::to_owned);
                        // The tool name comes from the matching earlier tool_use (same stream).
                        let tool = id.as_ref().and_then(|i| self.tool_names.get(i)).cloned();
                        self.push(TranscriptEntry {
                            kind: "tool_result",
                            tool,
                            id,
                            error: part.get("is_error").and_then(Value::as_bool).unwrap_or(false),
                            bytes: part.get("content").map(len),
                            ..Default::default()
                        });
                    }
                }
            }
            "result" => {
                for denial in event.get("permission_denials").and_then(Value::as_array).into_iter().flatten() {
                    self.push(TranscriptEntry {
                        kind: "tool_rejected",
                        tool: denial.get("tool_name").and_then(Value::as_str).map(str::to_owned),
                        id: denial.get("tool_use_id").and_then(Value::as_str).map(str::to_owned),
                        ..Default::default()
                    });
                }
                let error = event.get("is_error").and_then(Value::as_bool).unwrap_or(false);
                self.push(TranscriptEntry {
                    kind: "note",
                    error,
                    label: Some(format!("result:{}", event.get("subtype").and_then(Value::as_str).unwrap_or("?"))),
                    tokens: tokens(event.get("usage"), CLAUDE_USAGE),
                    ..Default::default()
                });
            }
            other => self.unknown(other),
        }
    }

    fn codex(&mut self, kind: &str, event: &Value) {
        self.claim("codex");
        match kind {
            "thread.started" => self.push(TranscriptEntry { kind: "system_prompt", ..Default::default() }),
            "turn.started" | "item.started" | "item.updated" => {}
            "turn.completed" => self.push(TranscriptEntry {
                kind: "model_call",
                tokens: tokens(event.get("usage"), CODEX_USAGE),
                ..Default::default()
            }),
            "turn.failed" | "error" => self.push(TranscriptEntry {
                kind: "note",
                error: true,
                label: Some(kind.to_owned()),
                ..Default::default()
            }),
            "item.completed" => {
                let item = event.get("item").cloned().unwrap_or(Value::Null);
                let failed = matches!(item.get("status").and_then(Value::as_str), Some("failed" | "declined"))
                    || item.get("exit_code").and_then(Value::as_i64).is_some_and(|c| c != 0);
                let id = item.get("id").and_then(Value::as_str).map(str::to_owned);
                match item.get("type").and_then(Value::as_str) {
                    Some("agent_message") => self.push(TranscriptEntry {
                        kind: "assistant_message",
                        bytes: item.get("text").map(len),
                        ..Default::default()
                    }),
                    Some("reasoning") => {}
                    Some(tool @ ("command_execution" | "file_change" | "mcp_tool_call" | "web_search")) => {
                        let declined = item.get("status").and_then(Value::as_str) == Some("declined");
                        self.push(TranscriptEntry {
                            kind: if declined { "tool_rejected" } else { "tool" },
                            tool: Some(item.get("tool").and_then(Value::as_str).unwrap_or(tool).to_owned()),
                            id,
                            error: failed && !declined,
                            bytes: item.get("aggregated_output").map(len),
                            ..Default::default()
                        })
                    }
                    Some(other) => self.unknown(&format!("item.{other}")),
                    None => self.unknown("item"),
                }
            }
            other => self.unknown(other),
        }
    }
}

/// Build the typed transcript of `capture` (see module doc). Never fails: a
/// non-JSON or truncated tail becomes a `note`, and unknown events are counted.
pub fn typed_transcript(capture: &str) -> Transcript {
    let mut t = Transcript { format: "unknown", ..Default::default() };
    let mut stream = serde_json::Deserializer::from_str(capture).into_iter::<Value>();
    loop {
        match stream.next() {
            None => break,
            Some(Ok(event)) => {
                let Some(kind) = event.get("type").and_then(Value::as_str).map(str::to_owned) else {
                    t.unknown("untyped");
                    continue;
                };
                if matches!(kind.as_str(), "system" | "assistant" | "user" | "result") {
                    t.claude(&kind, &event);
                } else if kind == "error" || kind.starts_with("thread.") || kind.starts_with("turn.") || kind.starts_with("item.") {
                    t.codex(&kind, &event);
                } else {
                    t.unknown(&kind);
                }
            }
            Some(Err(e)) => {
                if e.is_eof() {
                    t.truncated = true;
                    t.note("truncated".into());
                } else {
                    t.note("non-json".into());
                }
                break;
            }
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Claude stream-json after terminal cleaning (newlines → spaces).
    const CLAUDE: &str = concat!(
        r#"{"type":"system","subtype":"init","model":"claude-x","tools":["Bash"]} "#,
        r#"{"type":"assistant","message":{"model":"claude-x","usage":{"input_tokens":12,"output_tokens":5,"cache_read_input_tokens":900},"content":[{"type":"text","text":"SECRET plan"},{"type":"tool_use","id":"tu1","name":"Bash","input":{"command":"cat secret"}}]}} "#,
        r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu1","is_error":true,"content":"SECRET output"}]}} "#,
        r#"{"type":"result","subtype":"success","is_error":false,"permission_denials":[{"tool_name":"Edit","tool_use_id":"tu2"}],"usage":{"input_tokens":12,"output_tokens":5}} "#,
        r#"{"type":"stream_event","event":{}}"#
    );

    #[test]
    fn claude_stream_is_typed_without_content() {
        let t = typed_transcript(CLAUDE);
        assert_eq!(t.format, "claude");
        let kinds: Vec<_> = t.entries.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, ["system_prompt", "model_call", "assistant_message", "tool", "tool_result", "tool_rejected", "note", "note"]);
        assert_eq!(t.entries[1].tokens.get("cache_read"), Some(&900));
        assert_eq!(t.entries[3].tool.as_deref(), Some("Bash"));
        assert!(t.entries[4].error, "declared is_error");
        assert_eq!(t.entries[4].tool.as_deref(), Some("Bash"), "name from the matching tool_use");
        assert_eq!(t.entries[5].tool.as_deref(), Some("Edit"));
        assert_eq!(t.unknown.get("stream_event"), Some(&1));
        let json = serde_json::to_string(&t).unwrap();
        assert!(!json.contains("SECRET") && !json.contains("cat secret"), "content leaked: {json}");
        assert!(!t.truncated);
    }

    #[test]
    fn codex_stream_and_truncated_or_garbage_tails() {
        let codex = concat!(
            r#"{"type":"thread.started","thread_id":"th"} {"type":"turn.started"} "#,
            r#"{"type":"item.completed","item":{"id":"i1","type":"command_execution","command":"ls","aggregated_output":"a b","exit_code":2,"status":"failed"}} "#,
            r#"{"type":"item.completed","item":{"id":"i2","type":"agent_message","text":"done"}} "#,
            r#"{"type":"item.completed","item":{"id":"i3","type":"file_change","status":"declined"}} "#,
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":40,"output_tokens":7}} "#,
            r#"{"type":"item.completed","item":{"id":"i4","type":"todo_list"}} {"type":"item.comp"#
        );
        let t = typed_transcript(codex);
        assert_eq!(t.format, "codex");
        let kinds: Vec<_> = t.entries.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, ["system_prompt", "tool", "assistant_message", "tool_rejected", "model_call", "note", "note"]);
        assert!(t.entries[1].error && t.entries[1].tool.as_deref() == Some("command_execution"));
        assert_eq!(t.entries[4].tokens.get("cache_read"), Some(&40));
        assert_eq!(t.unknown.get("item.todo_list"), Some(&1));
        assert!(t.truncated, "capture ended mid-value");
        let garbage = typed_transcript("plain text, not json");
        assert_eq!((garbage.format, garbage.entries[0].label.as_deref()), ("unknown", Some("non-json")));
        assert_eq!(typed_transcript("").entries.len(), 0);
    }
}
