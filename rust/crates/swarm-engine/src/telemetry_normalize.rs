//! OTLP/JSON normalization for provider telemetry (LIVE-UX-03, TEL-R9/R12).
//!
//! Claude Code and Codex export OpenTelemetry logs (one record per event) and
//! metrics. This module turns an OTLP/JSON request body into whitelisted records:
//! only known event/metric names are kept, and only scalar fields from a fixed
//! per-name allowlist (names, counters, durations, codes). Prompt, response,
//! tool parameter/output and error text never pass: content fields are simply
//! not in any allowlist. Unknown names are counted, never stored as known.
//!
//! The result is a provider *declaration*: nothing here authenticates the
//! values. Attachment to an agent is decided by the caller (ingest token), never
//! by attributes such as `swarm.agent_id` that the body itself carries.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

/// Longest string field value kept. Longer values are dropped from the record
/// (the record itself is kept): legitimate whitelisted values (model, tool
/// names, decisions, ids) are short; a long value is likely content.
pub const TELEMETRY_FIELD_MAX_CHARS: usize = 128;

/// Kind of a normalized record: an OTLP log record (event), a numeric metric
/// data point, or an explicit-bucket histogram data point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryKind {
    Event,
    Point,
    Histogram,
    Span,
}
impl TelemetryKind {
    /// Stable storage spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Point => "point",
            Self::Histogram => "histogram",
            Self::Span => "span",
        }
    }
}

/// Aggregation temporality declared by a metric data point (OTLP enum 1/2), or
/// `Gauge` for gauge points. Totals must never mix delta and cumulative values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Temporality {
    Delta,
    Cumulative,
    Gauge,
}
impl Temporality {
    /// Stable storage spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Delta => "delta",
            Self::Cumulative => "cumulative",
            Self::Gauge => "gauge",
        }
    }
}

/// One whitelisted record. `digest` identifies it for idempotent storage
/// (TEL-R7): the same OTLP record replayed by an exporter retry yields the same
/// digest once combined with the agent id by the store.
#[derive(Clone, Debug, PartialEq)]
pub struct TelemetryRecord {
    pub kind: TelemetryKind,
    /// Normalized name (`claude_code.` / `codex.` prefix removed).
    pub name: String,
    /// Provider timestamp in ms (record time, else observed time, else receipt).
    pub at_ms: i64,
    pub fields: BTreeMap<String, Value>,
    pub value: Option<f64>,
    pub temporality: Option<Temporality>,
    /// Hex sha256 over the normalized content and raw provider timestamps.
    pub digest: String,
}

/// Normalized request: kept records, permanently rejected items (malformed
/// record or point, unsupported metric kind) and unknown-name counts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NormalizedBatch {
    pub records: Vec<TelemetryRecord>,
    pub rejected: u64,
    pub unknown: BTreeMap<String, u64>,
}

/// The request body is not an OTLP/JSON document of the expected signal. The
/// whole request is refused (HTTP 400, not retryable); no record is kept.
#[derive(Debug)]
pub enum TelemetryInputError {
    Json(serde_json::Error),
    Shape(&'static str),
}
impl TelemetryInputError {
    /// Stable public code; carries no body text.
    pub fn code(&self) -> &'static str {
        "telemetry-body-invalid"
    }
}
impl fmt::Display for TelemetryInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(_) => f.write_str("telemetry-body-invalid: json"),
            Self::Shape(what) => write!(f, "telemetry-body-invalid: {what}"),
        }
    }
}
impl std::error::Error for TelemetryInputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            Self::Shape(_) => None,
        }
    }
}

/// Fields kept per known event (after prefix removal). Sources: Claude Code
/// monitoring docs and Codex `[otel]` events as normalized by LangWatch
/// (Apache-2.0, `modules/coding-agent`). Codex names and fields were checked
/// on 2026-10-22 against codex-cli 0.163.0 (local fake model): its
/// `tool_result` carries `arguments` and `output` content, never kept here.
const EVENT_FIELDS: &[(&str, &[&str])] = &[
    (
        "api_request",
        &[
            "model",
            "cost_usd",
            "duration_ms",
            "input_tokens",
            "output_tokens",
            "cache_read_tokens",
            "cache_creation_tokens",
            "attempt",
            "http.response.status_code",
        ],
    ),
    (
        "api_error",
        &["model", "status_code", "duration_ms", "attempt", "http.response.status_code"],
    ),
    (
        "tool_result",
        &[
            "tool_name",
            "success",
            "duration_ms",
            "decision",
            "source",
            "decision_type",
            "decision_source",
            "tool_result_size_bytes",
        ],
    ),
    ("tool_decision", &["tool_name", "decision", "source"]),
    ("user_prompt", &["prompt_length"]),
    (
        "sse_event",
        &[
            "event.kind",
            "duration_ms",
            "input_token_count",
            "output_token_count",
            "cached_token_count",
            "reasoning_token_count",
            "tool_token_count",
        ],
    ),
    ("conversation_starts", &["model"]),
    // Codex HTTP attempt (`codex.api_request`): timing and status, no tokens.
    ("api_attempt", &["model", "duration_ms", "attempt", "http.response.status_code"]),
];

/// Codex reports a model call's tokens on its `response.completed` SSE event,
/// not on `codex.api_request` (an HTTP attempt). Map that event to the common
/// `api_request` call shape: `input_tokens` = input − cached (OpenAI usage
/// counts cached tokens inside input; Claude reports them apart, and the
/// console adds input + cache read + cache creation), `cache_read_tokens` =
/// cached, `cache_creation_tokens` = cache write, plus `reasoning_tokens`.
/// Values stay decimal strings like the provider's. `None` for any other SSE
/// event or when the counts are missing or not decimal (never guessed).
fn codex_call(attrs: Option<&Value>) -> Option<BTreeMap<String, Value>> {
    if attribute_str(attrs, "event.kind") != Some("response.completed") {
        return None;
    }
    let count = |key: &str| attribute_str(attrs, key)?.parse::<u64>().ok();
    let (input, cached, output) = (count("input_token_count")?, count("cached_token_count").unwrap_or(0), count("output_token_count")?);
    let mut fields = BTreeMap::new();
    if let Some(model) = attribute_str(attrs, "model").filter(|m| m.chars().count() <= TELEMETRY_FIELD_MAX_CHARS) {
        fields.insert("model".to_owned(), Value::String(model.to_owned()));
    }
    fields.insert("input_tokens".to_owned(), Value::String(input.checked_sub(cached)?.to_string()));
    fields.insert("output_tokens".to_owned(), Value::String(output.to_string()));
    fields.insert("cache_read_tokens".to_owned(), Value::String(cached.to_string()));
    for (from, to) in [("cache_write_token_count", "cache_creation_tokens"), ("reasoning_token_count", "reasoning_tokens")] {
        if let Some(n) = count(from) {
            fields.insert(to.to_owned(), Value::String(n.to_string()));
        }
    }
    Some(fields)
}

/// Known metric names; data point attributes kept for all of them.
const METRIC_NAMES: &[&str] = &[
    "session.count",
    "lines_of_code.count",
    "pull_request.count",
    "commit.count",
    "cost.usage",
    "token.usage",
    "code_edit_tool.decision",
    "active_time.total",
    "turn.token_usage",
];
const POINT_FIELDS: &[&str] = &["type", "model", "decision", "tool_name", "language", "session.id", "token_type", "status", "success"];

/// Known explicit-bucket histograms (Codex, observed 2026-10-22 on codex-cli
/// 0.163.0): call and turn latencies in ms, and tokens per turn by
/// `token_type`. Kept as one record per data point: `value` = sum, fields
/// `count`, optional `min`/`max`, `bounds`/`buckets` (at most
/// [`HISTOGRAM_MAX_BUCKETS`] buckets) plus allowlisted attributes. Other
/// histogram names are counted as unknown, not rejected.
const HISTOGRAM_NAMES: &[&str] = &[
    "api_request.duration_ms",
    "turn.ttft.duration_ms",
    "turn.ttfm.duration_ms",
    "turn.e2e_duration_ms",
    "turn.token_usage",
];
/// Technical bound on stored buckets per histogram point (OTLP SDK defaults
/// use 16); a point with more is rejected and counted, never truncated.
pub const HISTOGRAM_MAX_BUCKETS: usize = 64;

/// Decode one explicit-bucket histogram point; `None` when its shape is not
/// the OTLP one (count/sum/buckets/bounds) so the caller rejects and counts it.
fn histogram_point(point: &Value) -> Option<(f64, BTreeMap<String, Value>)> {
    let int = |v: &Value| match v {
        Value::String(s) => s.parse::<u64>().ok(),
        Value::Number(n) => n.as_u64(),
        _ => None,
    };
    let count = int(point.get("count")?)?;
    let sum = point.get("sum").and_then(Value::as_f64).filter(|v| v.is_finite())?;
    let buckets = point.get("bucketCounts").and_then(Value::as_array).map_or(Some(Vec::new()), |b| b.iter().map(int).collect::<Option<Vec<_>>>())?;
    let bounds = point.get("explicitBounds").and_then(Value::as_array).map_or(Some(Vec::new()), |b| {
        b.iter().map(|x| x.as_f64().filter(|v| v.is_finite())).collect::<Option<Vec<_>>>()
    })?;
    if buckets.len() > HISTOGRAM_MAX_BUCKETS
        || (!buckets.is_empty() && (buckets.len() != bounds.len() + 1 || buckets.iter().sum::<u64>() != count))
    {
        return None;
    }
    let mut fields = attributes(point.get("attributes"), POINT_FIELDS);
    fields.insert("count".into(), Value::from(count));
    for key in ["min", "max"] {
        if let Some(v) = point.get(key).and_then(Value::as_f64).filter(|v| v.is_finite()) {
            fields.insert(key.into(), Value::from(v));
        }
    }
    if !buckets.is_empty() {
        fields.insert("buckets".into(), Value::from(buckets));
        fields.insert("bounds".into(), Value::from(bounds));
    }
    Some((sum, fields))
}

/// Strip the provider prefix; names without a known prefix are kept verbatim
/// (and are then usually unknown).
fn normalized(name: &str) -> &str {
    name.strip_prefix("claude_code.")
        .or_else(|| name.strip_prefix("codex."))
        .unwrap_or(name)
}

/// OTLP AnyValue → JSON scalar; arrays/maps/bytes are never kept.
fn scalar(value: &Value) -> Option<Value> {
    let value = value.as_object()?;
    if let Some(s) = value.get("stringValue").and_then(Value::as_str) {
        return (s.chars().count() <= TELEMETRY_FIELD_MAX_CHARS).then(|| Value::String(s.into()));
    }
    if let Some(b) = value.get("boolValue").and_then(Value::as_bool) {
        return Some(Value::Bool(b));
    }
    if let Some(v) = value.get("intValue") {
        // proto3 JSON encodes int64 as a string; accept a number too.
        return match v {
            Value::String(s) => s.parse::<i64>().ok().map(Value::from),
            Value::Number(n) => n.as_i64().map(Value::from),
            _ => None,
        };
    }
    value
        .get("doubleValue")
        .and_then(Value::as_f64)
        .and_then(|d| serde_json::Number::from_f64(d).map(Value::Number))
}

/// Whitelisted attributes from an OTLP `attributes` array.
fn attributes(raw: Option<&Value>, allowed: &[&str]) -> BTreeMap<String, Value> {
    let mut kept = BTreeMap::new();
    for item in raw.and_then(Value::as_array).into_iter().flatten() {
        let Some(key) = item.get("key").and_then(Value::as_str) else {
            continue;
        };
        if !allowed.contains(&key) {
            continue;
        }
        if let Some(v) = item.get("value").and_then(scalar) {
            kept.insert(key.to_owned(), v);
        }
    }
    kept
}

/// Raw string value of one attribute (used for `event.name` only).
fn attribute_str<'a>(raw: Option<&'a Value>, key: &str) -> Option<&'a str> {
    raw.and_then(Value::as_array)?
        .iter()
        .find(|item| item.get("key").and_then(Value::as_str) == Some(key))?
        .get("value")?
        .get("stringValue")?
        .as_str()
}

/// OTLP nanosecond timestamp (string or number); zero means absent.
fn nanos(value: Option<&Value>) -> Option<u64> {
    let n = match value? {
        Value::String(s) => s.parse::<u64>().ok()?,
        Value::Number(n) => n.as_u64()?,
        _ => return None,
    };
    (n > 0).then_some(n)
}

fn millis(ns: Option<u64>, received_ms: i64) -> i64 {
    ns.map_or(received_ms, |n| i64::try_from(n / 1_000_000).unwrap_or(i64::MAX))
}

/// Content digest of a record; raw timestamps keep two distinct calls in the
/// same millisecond apart, while an exporter retry of the same record matches.
fn digest(kind: TelemetryKind, name: &str, stamps: &[Option<u64>], fields: &BTreeMap<String, Value>, value: Option<f64>, temporality: Option<Temporality>) -> String {
    let canonical = serde_json::json!({
        "kind": kind.as_str(),
        "name": name,
        "stamps": stamps,
        "fields": fields,
        "value": value,
        "temporality": temporality.map(Temporality::as_str),
    });
    hex(&Sha256::digest(canonical.to_string().as_bytes()))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Iterate `outer[*].inner[*].leaf[*]` of an OTLP envelope. A non-array
/// container refuses the whole body; absent containers are empty.
fn nested<'a>(
    root: &'a Map<String, Value>,
    path: [&'static str; 3],
) -> Result<Vec<&'a Value>, TelemetryInputError> {
    fn items<'b>(v: Option<&'b Value>, what: &'static str) -> Result<&'b [Value], TelemetryInputError> {
        match v {
            None => Ok(&[]),
            Some(Value::Array(a)) => Ok(a),
            Some(_) => Err(TelemetryInputError::Shape(what)),
        }
    }
    let mut leaves = Vec::new();
    for outer in items(root.get(path[0]), path[0])? {
        for inner in items(outer.get(path[1]), path[1])? {
            leaves.extend(items(inner.get(path[2]), path[2])?);
        }
    }
    Ok(leaves)
}

fn document(body: &[u8]) -> Result<Map<String, Value>, TelemetryInputError> {
    match serde_json::from_slice::<Value>(body).map_err(TelemetryInputError::Json)? {
        Value::Object(map) => Ok(map),
        _ => Err(TelemetryInputError::Shape("request-object")),
    }
}

/// Normalize an OTLP/JSON `ExportLogsServiceRequest`.
///
/// The event name is the `event.name` attribute, else the string body. Known
/// events keep their allowlisted fields; unknown names are counted; a record
/// that is not an object or has no name is rejected (`partialSuccess`).
/// `received_ms` stamps records without provider time. Errors: the body is not
/// JSON or its containers are not arrays → whole request refused.
pub fn normalize_logs(body: &[u8], received_ms: i64) -> Result<NormalizedBatch, TelemetryInputError> {
    let root = document(body)?;
    let mut batch = NormalizedBatch::default();
    for record in nested(&root, ["resourceLogs", "scopeLogs", "logRecords"])? {
        let Some(map) = record.as_object() else {
            batch.rejected += 1;
            continue;
        };
        let attrs = map.get("attributes");
        let name = attribute_str(attrs, "event.name")
            .or_else(|| map.get("body")?.get("stringValue")?.as_str());
        let raw = name;
        let Some(mut name) = name.map(normalized).filter(|n| !n.is_empty()) else {
            batch.rejected += 1;
            continue;
        };
        // Codex: an HTTP attempt is not a model call; the completed response is.
        let codex = raw.is_some_and(|r| r.starts_with("codex."));
        let mut mapped = None;
        if codex && name == "api_request" {
            name = "api_attempt";
        } else if codex && name == "sse_event" {
            match codex_call(attrs) {
                Some(fields) => {
                    name = "api_request";
                    mapped = Some(fields);
                }
                // Other stream events are per-chunk noise: counted, not stored.
                None => {
                    *batch.unknown.entry("sse_event".to_owned()).or_default() += 1;
                    continue;
                }
            }
        }
        let Some((_, allowed)) = EVENT_FIELDS.iter().find(|(known, _)| *known == name) else {
            *batch.unknown.entry(name.to_owned()).or_default() += 1;
            continue;
        };
        let fields = mapped.unwrap_or_else(|| attributes(attrs, allowed));
        let stamps = [nanos(map.get("timeUnixNano")), nanos(map.get("observedTimeUnixNano"))];
        batch.records.push(TelemetryRecord {
            kind: TelemetryKind::Event,
            name: name.to_owned(),
            at_ms: millis(stamps[0].or(stamps[1]), received_ms),
            digest: digest(TelemetryKind::Event, name, &stamps, &fields, None, None),
            fields,
            value: None,
            temporality: None,
        });
    }
    Ok(batch)
}

/// Span names kept (Codex, observed 2026-10-22 on codex-cli 0.163.0): the run,
/// each turn, each model sampling request, and each tool execution. Codex
/// emits hundreds of internal spans per turn (`fs.*`, config, plugin loading):
/// those are counted under `span (other)`, never stored. Claude Code spans
/// (enhanced telemetry beta, observed 2026-10-22 on Claude Code 2.1.280 with a
/// local fake model, see `docs/plans/telemetrie-agents/claude-otel-probe-20261022`):
/// the interaction, each model request, each tool, its execution and the
/// permission wait.
const SPAN_NAMES: &[&str] = &[
    "claude_code.interaction",
    "claude_code.llm_request",
    "claude_code.tool",
    "claude_code.tool.execution",
    "claude_code.tool.blocked_on_user",
    "codex.exec",
    "session_task.turn",
    "run_sampling_request",
    "codex.tool_blocking",
    "exec_command",
    "handle_responses",
];
/// Span attributes kept: identity of the turn/tool, token counts and
/// outcome only; code locations, thread data, prompts (Claude
/// `user_prompt`, redacted anyway), the program name of a shell command
/// (`bash_argv0`, may be a private path) and every other attribute are
/// dropped.
const SPAN_FIELDS: &[&str] = &[
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_creation_tokens",
    "success",
    "ttft_ms",
    "stop_reason",
    "decision",
    "source",
    "bash_command_class",
    "attempt",
    "model",
    "turn_id",
    "turn.id",
    "tool_name",
    "call_id",
    "outcome",
    "aborted",
    "codex.turn.token_usage.input_tokens",
    "codex.turn.token_usage.cached_input_tokens",
    "codex.turn.token_usage.output_tokens",
    "codex.turn.token_usage.reasoning_output_tokens",
    "gen_ai.usage.input_tokens",
    "gen_ai.usage.output_tokens",
    "gen_ai.usage.cache_read.input_tokens",
];

/// Normalize an OTLP/JSON `ExportTraceServiceRequest`: known spans become
/// `span` records (`at_ms` = start, `value` = duration in ms, fields
/// `trace_id`, `span_id`, `parent_id` plus allowlisted attributes); unknown
/// span names are counted as `span (other)`; spans without ids or with an
/// end before their start are rejected and counted. Errors as for
/// [`normalize_logs`].
pub fn normalize_traces(body: &[u8], received_ms: i64) -> Result<NormalizedBatch, TelemetryInputError> {
    let root = document(body)?;
    let mut batch = NormalizedBatch::default();
    for span in nested(&root, ["resourceSpans", "scopeSpans", "spans"])? {
        let Some(map) = span.as_object() else {
            batch.rejected += 1;
            continue;
        };
        let Some(name) = map.get("name").and_then(Value::as_str) else {
            batch.rejected += 1;
            continue;
        };
        if !SPAN_NAMES.contains(&name) {
            *batch.unknown.entry("span (other)".to_owned()).or_default() += 1;
            continue;
        }
        let id = |k: &str| map.get(k).and_then(Value::as_str).filter(|v| !v.is_empty() && v.len() <= 64 && v.bytes().all(|b| b.is_ascii_hexdigit()));
        let (Some(trace), Some(span_id)) = (id("traceId"), id("spanId")) else {
            batch.rejected += 1;
            continue;
        };
        let (start, end) = (nanos(map.get("startTimeUnixNano")), nanos(map.get("endTimeUnixNano")));
        let (Some(start), Some(end)) = (start, end) else {
            batch.rejected += 1;
            continue;
        };
        if end < start {
            batch.rejected += 1;
            continue;
        }
        let mut fields = attributes(map.get("attributes"), SPAN_FIELDS);
        fields.insert("trace_id".into(), Value::String(trace.into()));
        fields.insert("span_id".into(), Value::String(span_id.into()));
        if let Some(parent) = id("parentSpanId") {
            fields.insert("parent_id".into(), Value::String(parent.into()));
        }
        let duration = (end - start) as f64 / 1_000_000.0;
        let stamps = [Some(start), Some(end)];
        batch.records.push(TelemetryRecord {
            kind: TelemetryKind::Span,
            name: name.to_owned(),
            at_ms: millis(Some(start), received_ms),
            digest: digest(TelemetryKind::Span, name, &stamps, &fields, Some(duration), None),
            fields,
            value: Some(duration),
            temporality: None,
        });
    }
    Ok(batch)
}

/// Normalize an OTLP/JSON `ExportMetricsServiceRequest`.
///
/// Known `sum` and `gauge` metrics keep each numeric data point with its
/// declared temporality and allowlisted attributes; known explicit-bucket
/// histograms keep each point (see `HISTOGRAM_NAMES`). Exponential
/// histograms, summaries, malformed and non-numeric points are rejected
/// (counted per point); unknown metric names, histograms included, are
/// counted per point. Errors as for [`normalize_logs`].
pub fn normalize_metrics(body: &[u8], received_ms: i64) -> Result<NormalizedBatch, TelemetryInputError> {
    let root = document(body)?;
    let mut batch = NormalizedBatch::default();
    for metric in nested(&root, ["resourceMetrics", "scopeMetrics", "metrics"])? {
        let Some(map) = metric.as_object() else {
            batch.rejected += 1;
            continue;
        };
        let (data, temporality) = if let Some(sum) = map.get("sum") {
            let temporality = match sum.get("aggregationTemporality").and_then(Value::as_i64) {
                Some(1) => Some(Temporality::Delta),
                Some(2) => Some(Temporality::Cumulative),
                _ => None,
            };
            (sum, temporality)
        } else if let Some(gauge) = map.get("gauge") {
            (gauge, Some(Temporality::Gauge))
        } else if let Some(histogram) = map.get("histogram") {
            let points: &[Value] = histogram.get("dataPoints").and_then(Value::as_array).map_or(&[], Vec::as_slice);
            let Some(name) = map.get("name").and_then(Value::as_str).map(normalized).filter(|n| !n.is_empty()) else {
                batch.rejected += points.len().max(1) as u64;
                continue;
            };
            if !HISTOGRAM_NAMES.contains(&name) {
                *batch.unknown.entry(name.to_owned()).or_default() += points.len().max(1) as u64;
                continue;
            }
            let temporality = match histogram.get("aggregationTemporality").and_then(Value::as_i64) {
                Some(1) => Temporality::Delta,
                Some(2) => Temporality::Cumulative,
                _ => {
                    batch.rejected += points.len() as u64;
                    continue;
                }
            };
            for point in points {
                let Some((sum, fields)) = histogram_point(point) else {
                    batch.rejected += 1;
                    continue;
                };
                let stamps = [nanos(point.get("timeUnixNano")), nanos(point.get("startTimeUnixNano"))];
                batch.records.push(TelemetryRecord {
                    kind: TelemetryKind::Histogram,
                    name: name.to_owned(),
                    at_ms: millis(stamps[0], received_ms),
                    digest: digest(TelemetryKind::Histogram, name, &stamps, &fields, Some(sum), Some(temporality)),
                    fields,
                    value: Some(sum),
                    temporality: Some(temporality),
                });
            }
            continue;
        } else {
            let points = ["exponentialHistogram", "summary"]
                .iter()
                .find_map(|k| map.get(*k)?.get("dataPoints")?.as_array().map(Vec::len))
                .unwrap_or(1);
            batch.rejected += points as u64;
            continue;
        };
        let points: &[Value] = data.get("dataPoints").and_then(Value::as_array).map_or(&[], Vec::as_slice);
        let Some(name) = map.get("name").and_then(Value::as_str).map(normalized).filter(|n| !n.is_empty()) else {
            batch.rejected += points.len().max(1) as u64;
            continue;
        };
        if !METRIC_NAMES.contains(&name) {
            *batch.unknown.entry(name.to_owned()).or_default() += points.len().max(1) as u64;
            continue;
        }
        let Some(temporality) = temporality else {
            batch.rejected += points.len() as u64;
            continue;
        };
        for point in points {
            let value = point.get("asDouble").and_then(Value::as_f64).or_else(|| match point.get("asInt") {
                Some(Value::String(s)) => s.parse::<i64>().ok().map(|i| i as f64),
                Some(Value::Number(n)) => n.as_f64(),
                _ => None,
            });
            let Some(value) = value.filter(|v| v.is_finite()) else {
                batch.rejected += 1;
                continue;
            };
            let fields = attributes(point.get("attributes"), POINT_FIELDS);
            let stamps = [nanos(point.get("timeUnixNano")), nanos(point.get("startTimeUnixNano"))];
            batch.records.push(TelemetryRecord {
                kind: TelemetryKind::Point,
                name: name.to_owned(),
                at_ms: millis(stamps[0], received_ms),
                digest: digest(TelemetryKind::Point, name, &stamps, &fields, Some(value), Some(temporality)),
                fields,
                value: Some(value),
                temporality: Some(temporality),
            });
        }
    }
    Ok(batch)
}

#[cfg(test)]
mod tests {
    #[test]
    fn claude_spans_keep_outcome_and_tokens_but_drop_prompt_and_program() {
        // Shapes from the Claude Code 2.1.280 probe (local fake model).
        let attr = |k: &str, v: &str| serde_json::json!({"key": k, "value": {"stringValue": v}});
        let span = |id: &str, parent: &str, name: &str, attrs: Vec<serde_json::Value>| serde_json::json!({
            "traceId": "0a1b", "spanId": id, "parentSpanId": parent, "name": name,
            "startTimeUnixNano": "1000000", "endTimeUnixNano": "3000000", "attributes": attrs});
        let body = serde_json::json!({"resourceSpans": [{"scopeSpans": [{"spans": [
            span("01", "", "claude_code.interaction", vec![attr("user_prompt", "<REDACTED>"), attr("user_prompt_length", "46"), attr("session.id", "s")]),
            span("02", "01", "claude_code.llm_request", vec![attr("model", "claude-sonnet-4-5"), attr("input_tokens", "20"), attr("output_tokens", "5"), attr("cache_read_tokens", "0"), attr("ttft_ms", "20"), attr("stop_reason", "tool_use"), attr("success", "True")]),
            span("03", "01", "claude_code.tool", vec![attr("tool_name", "Bash"), attr("bash_command_class", "shell_builtin"), attr("bash_argv0", "/home/private/script.sh")]),
            span("04", "03", "claude_code.tool.blocked_on_user", vec![attr("decision", "unknown"), attr("source", "unknown")]),
            span("06", "03", "claude_code.tool.execution", vec![attr("success", "True"), attr("tool_use_id", "toolu_x")]),
            span("05", "01", "claude_code.internal_thing", vec![])]}]}]});
        let batch = normalize_traces(body.to_string().as_bytes(), 1).unwrap();
        let kept: Vec<_> = batch.records.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(kept, ["claude_code.interaction", "claude_code.llm_request", "claude_code.tool", "claude_code.tool.blocked_on_user", "claude_code.tool.execution"]);
        let fields = |name: &str| batch.records.iter().find(|r| r.name == name).unwrap().fields.clone();
        assert_eq!(fields("claude_code.tool.execution").get("success"), Some(&serde_json::json!("True")));
        assert!(!fields("claude_code.tool.execution").contains_key("tool_use_id"), "ids outside the allowlist dropped");
        assert!(!fields("claude_code.interaction").contains_key("user_prompt"), "prompt never kept, even redacted");
        assert_eq!(fields("claude_code.llm_request").get("output_tokens"), Some(&serde_json::json!("5")));
        assert_eq!(fields("claude_code.llm_request").get("stop_reason"), Some(&serde_json::json!("tool_use")));
        let tool = fields("claude_code.tool");
        assert_eq!((tool.get("tool_name"), tool.get("bash_command_class")), (Some(&serde_json::json!("Bash")), Some(&serde_json::json!("shell_builtin"))));
        assert!(!tool.contains_key("bash_argv0"), "program path dropped");
        assert_eq!(batch.unknown.get("span (other)"), Some(&1));
        assert!(!format!("{batch:?}").contains("/home/private") && !format!("{batch:?}").contains("REDACTED"), "no path or prompt marker kept");
    }

    use super::*;
    use serde_json::json;

    fn attr(key: &str, value: Value) -> Value {
        json!({"key": key, "value": value})
    }

    fn logs(records: Vec<Value>) -> Vec<u8> {
        json!({"resourceLogs": [{
            "resource": {"attributes": [attr("swarm.agent_id", json!({"stringValue": "other"}))]},
            "scopeLogs": [{"logRecords": records}]
        }]})
        .to_string()
        .into_bytes()
    }

    /// Shape observed from codex-cli 0.163.0 (2026-10-22): zero `timeUnixNano`,
    /// time in `observedTimeUnixNano`, every value a string, name in `event.name`.
    fn codex(event: &str, observed: u64, extra: &[(&str, &str)]) -> Value {
        let mut attrs = vec![attr("event.name", json!({"stringValue": event})), attr("model", json!({"stringValue": "gpt-x"}))];
        attrs.extend(extra.iter().map(|(k, v)| attr(k, json!({"stringValue": v}))));
        json!({"timeUnixNano": "0", "observedTimeUnixNano": observed.to_string(), "body": null, "attributes": attrs})
    }

    #[test]
    fn codex_completed_response_is_the_call_and_content_never_kept() {
        let body = logs(vec![
            codex("codex.user_prompt", 1_000_000_000, &[("prompt", "[REDACTED]"), ("prompt_length", "6")]),
            codex("codex.api_request", 2_000_000_000, &[("duration_ms", "2013"), ("http.response.status_code", "200"), ("attempt", "0")]),
            codex("codex.sse_event", 3_000_000_000, &[("event.kind", "response.created"), ("duration_ms", "0")]),
            codex("codex.sse_event", 4_000_000_000, &[("event.kind", "response.completed"), ("input_token_count", "10"), ("output_token_count", "3"),
                ("cached_token_count", "4"), ("cache_write_token_count", "0"), ("reasoning_token_count", "1"), ("tool_token_count", "13")]),
            codex("codex.tool_result", 5_000_000_000, &[("tool_name", "exec_command"), ("duration_ms", "146"), ("success", "true"),
                ("arguments", r#"{"cmd": "echo PRIVATE-ARG"}"#), ("output", "PRIVATE-OUTPUT"), ("call_id", "call1")]),
            codex("codex.sse_event", 6_000_000_000, &[("event.kind", "response.completed")]), // no counts: not a call
        ]);
        let batch = normalize_logs(&body, 99).unwrap();
        let names: Vec<_> = batch.records.iter().map(|r| (r.name.as_str(), r.at_ms)).collect();
        assert_eq!(names, [("user_prompt", 1000), ("api_attempt", 2000), ("api_request", 4000), ("tool_result", 5000)]);
        let call = &batch.records[2].fields;
        assert_eq!(serde_json::to_value(call).unwrap(), json!({"model": "gpt-x", "input_tokens": "6", "output_tokens": "3",
            "cache_read_tokens": "4", "cache_creation_tokens": "0", "reasoning_tokens": "1"}), "uncached input = 10 - 4");
        assert_eq!(batch.unknown.get("sse_event"), Some(&2), "stream chunks counted, not stored");
        let stored = serde_json::to_string(&batch.records.iter().map(|r| &r.fields).collect::<Vec<_>>()).unwrap();
        assert!(!stored.contains("PRIVATE") && !stored.contains("REDACTED"), "no prompt, argument or output");
        assert_eq!(batch.records[3].fields.get("tool_name"), Some(&json!("exec_command")));
        // Claude's own api_request is untouched by the Codex mapping.
        let claude = normalize_logs(&logs(vec![json!({"timeUnixNano": "7000000", "body": {"stringValue": "claude_code.api_request"},
            "attributes": [attr("input_tokens", json!({"stringValue": "5"}))]})]), 0).unwrap();
        assert_eq!((claude.records[0].name.as_str(), claude.records[0].fields.get("input_tokens")), ("api_request", Some(&json!("5"))));
    }

    #[test]
    fn claude_events_keep_only_allowlisted_fields_and_drop_content() {
        let body = logs(vec![
            json!({"timeUnixNano": "1700000000123000000", "body": {"stringValue": "claude_code.api_request"},
                "attributes": [attr("event.name", json!({"stringValue": "api_request"})),
                    attr("model", json!({"stringValue": "claude-x"})),
                    attr("input_tokens", json!({"stringValue": "12"})),
                    attr("cost_usd", json!({"doubleValue": 0.25})),
                    attr("prompt", json!({"stringValue": "SECRET PROMPT"}))]}),
            json!({"body": {"stringValue": "claude_code.tool_result"},
                "attributes": [attr("tool_name", json!({"stringValue": "Bash"})),
                    attr("success", json!({"stringValue": "false"})),
                    attr("tool_parameters", json!({"stringValue": "{\"command\":\"cat secret\"}"})),
                    attr("error", json!({"stringValue": "permission denied: /home/x"}))]}),
            json!({"body": {"stringValue": "claude_code.user_prompt"},
                "attributes": [attr("prompt_length", json!({"intValue": "42"})),
                    attr("prompt", json!({"stringValue": "SECRET"}))]}),
        ]);
        let batch = normalize_logs(&body, 5).unwrap();
        assert_eq!(batch.rejected, 0);
        assert_eq!(batch.records.len(), 3);
        let api = &batch.records[0];
        assert_eq!((api.name.as_str(), api.at_ms), ("api_request", 1_700_000_000_123));
        assert_eq!(api.fields.get("input_tokens"), Some(&json!("12")));
        assert_eq!(api.fields.get("cost_usd"), Some(&json!(0.25)));
        assert_eq!(batch.records[1].at_ms, 5, "receipt time without provider time");
        assert_eq!(batch.records[2].fields.get("prompt_length"), Some(&json!(42)));
        let text = format!("{:?}", batch.records);
        for secret in ["SECRET", "cat secret", "permission denied", "other"] {
            assert!(!text.contains(secret), "{secret} leaked");
        }
    }

    #[test]
    fn unknown_names_are_counted_not_kept_and_malformed_records_rejected() {
        let body = logs(vec![
            json!({"body": {"stringValue": "codex.brand_new_event"}}),
            json!({"body": {"stringValue": "codex.brand_new_event"}}),
            json!("not-an-object"),
            json!({"attributes": []}),
            json!({"attributes": [attr("event.name", json!({"stringValue": "codex.tool_result"})),
                attr("tool_name", json!({"stringValue": "shell"}))]}),
        ]);
        let batch = normalize_logs(&body, 1).unwrap();
        assert_eq!(batch.unknown.get("brand_new_event"), Some(&2));
        assert_eq!(batch.rejected, 2);
        assert_eq!(batch.records.len(), 1);
        assert_eq!(batch.records[0].name, "tool_result");
    }

    #[test]
    fn retry_has_same_digest_and_distinct_calls_differ() {
        let one = json!({"timeUnixNano": "1000000000", "body": {"stringValue": "claude_code.api_request"}});
        let two = json!({"timeUnixNano": "1000000001", "body": {"stringValue": "claude_code.api_request"}});
        let a = normalize_logs(&logs(vec![one.clone()]), 1).unwrap();
        let b = normalize_logs(&logs(vec![one]), 99).unwrap();
        let c = normalize_logs(&logs(vec![two]), 1).unwrap();
        assert_eq!(a.records[0].digest, b.records[0].digest);
        assert_ne!(a.records[0].digest, c.records[0].digest);
    }

    #[test]
    fn metrics_keep_temporality_and_reject_unsupported_points() {
        let body = json!({"resourceMetrics": [{"scopeMetrics": [{"metrics": [
            {"name": "claude_code.token.usage", "unit": "tokens", "sum": {"aggregationTemporality": 1,
                "dataPoints": [
                    {"timeUnixNano": "2000000000", "asDouble": 10, "attributes": [attr("type", json!({"stringValue": "input"}))]},
                    {"timeUnixNano": "2000000000", "asInt": "4", "attributes": [attr("type", json!({"stringValue": "output"})), attr("user.email", json!({"stringValue": "a@b"}))]},
                    {"timeUnixNano": "2000000000", "asDouble": "x"}
                ]}},
            {"name": "claude_code.cost.usage", "sum": {"aggregationTemporality": 2, "dataPoints": [{"asDouble": 0.5}]}},
            {"name": "claude_code.lines_of_code.count", "histogram": {"dataPoints": [{}, {}]}},
            {"name": "codex.mystery", "gauge": {"dataPoints": [{"asDouble": 1}]}},
            {"name": "claude_code.commit.count", "sum": {"dataPoints": [{"asDouble": 1}]}}
        ]}]}]})
        .to_string();
        let batch = normalize_metrics(body.as_bytes(), 7).unwrap();
        assert_eq!(batch.records.len(), 3);
        assert_eq!(batch.records[0].temporality, Some(Temporality::Delta));
        assert_eq!(batch.records[1].value, Some(4.0));
        assert!(!batch.records[1].fields.contains_key("user.email"));
        assert_eq!(batch.records[2].temporality, Some(Temporality::Cumulative));
        // bad value (1) + sum without temporality (1); a histogram with an
        // unknown name is counted as unknown (2 points), not rejected.
        assert_eq!(batch.rejected, 2);
        assert_eq!(batch.unknown.get("mystery"), Some(&1));
        assert_eq!(batch.unknown.get("lines_of_code.count"), Some(&2));
    }

    #[test]
    fn invalid_envelopes_refuse_the_whole_request() {
        for body in [&b"not json"[..], b"[]", br#"{"resourceLogs": {}}"#, br#"{"resourceLogs": [{"scopeLogs": 3}]}"#] {
            assert!(normalize_logs(body, 1).is_err(), "{}", String::from_utf8_lossy(body));
        }
        assert_eq!(normalize_logs(b"{}", 1).unwrap(), NormalizedBatch::default());
        assert!(normalize_metrics(br#"{"resourceMetrics": [{"scopeMetrics": [{"metrics": 1}]}]}"#, 1).is_err());
    }
}
