//! Cancellable text-only HTTP adapter. Callers own authorization, budget, Root
//! identity and deadlines; this crate alone grants none. No tools/retries or
//! partial success; response usage is a declaration, not an invoice.
use serde_json::{json, Value};
mod strict_json;
use std::{error::Error as StdError, fmt};

/// Local observation of this call's send boundary. False only means this
/// witness has not observed send preparation; it grants no remote/billing claim.
/// Clones share monotone state. Not serialized, authenticated or execution rights.
#[derive(Clone,Default)]
pub struct ContactObservation(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl ContactObservation {
    /// Whether this witness observed the configured request reach send. True
    /// means possible remote effects, never confirmed completion or billed cost.
    pub fn may_have_occurred(&self)->bool {self.0.load(std::sync::atomic::Ordering::Acquire)}
    /// Record the send edge before polling network IO, retaining uncertainty if
    /// the future is cancelled or a local transport error follows.
    fn sending(&self){self.0.store(true,std::sync::atomic::Ordering::Release);}
}
/// Existing native completion versus manual-probe contract. Requiring a key
/// for complete is retained behavior, not a universal HTTP protocol invariant.
/// Broader Go optional-auth completion parity remains a separate migration item.
enum Operation { Completion, Probe }
/// Explicit operator transport choice; no protocol fallback after a failure.
pub enum Transport {
    Json,
    Sse,
}
/// Private connection and prompt. Deliberately neither Debug nor Serialize.
/// Caps originate in saved policy: None is unlimited, zero denies nonempty.
pub struct Request {
    pub base_url: String,
    pub key: String,
    pub model: String,
    pub prompt: String,
    pub transport: Transport,
    pub response_max_bytes: Option<usize>,
    pub event_max_bytes: Option<usize>,
    pub http1_max_headers: usize,
    pub http2_max_header_bytes: u32,
}
/// Complete validated content only; no partial answer or reasoning disclosure.
pub struct Response {
    pub content: String,
    pub usage: Option<Value>,
    pub reported_model: Option<String>,
}
/// Static safe error and original sanitized transport/codec cause. Raw remote
/// bodies, prompts, authorization headers and configured URLs are never rendered.
pub struct Error {
    code: &'static str,
    cause: Option<Box<dyn StdError + Send + Sync>>,
}
impl Error {
    /// Safe family, not a statement that no request was sent or billed.
    pub fn code(&self) -> &'static str {
        self.code
    }
    fn caused(code: &'static str, error: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            code,
            cause: Some(Box::new(error)),
        }
    }
    fn refused(code: &'static str) -> Self {
        Self { code, cause: None }
    }
    fn json(e: serde_json::Error) -> Self {
        Self {
            code: "api-response-json-invalid",
            cause: Some(Box::new(e)),
        }
    }
    fn transport(e: reqwest::Error) -> Self {
        Self {
            code: "api-transport-failed",
            cause: Some(Box::new(e.without_url())),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.cause.as_deref().map(|e| e as _)
    }
}
type Result<T> = std::result::Result<T, Error>;
/// Enforce supplied byte bounds without overflow or a second hidden ceiling.
fn limit(len: usize, cap: Option<usize>) -> Result<()> {
    if cap.is_some_and(|cap| len > cap) {
        Err(Error::refused("api-response-over-limit"))
    } else {
        Ok(())
    }
}
/// Tools/functions with any non-null declaration are refused, even empty arrays;
/// absence/null represent text-only provider protocol, not a sandbox guarantee.
fn text_only(v: &Value) -> Result<()> {
    if ["tool_calls", "function_call"]
        .iter()
        .any(|k| v.get(*k).is_some_and(|v| !v.is_null()))
    {
        Err(Error::refused("api-tool-declaration-refused"))
    } else {
        Ok(())
    }
}
/// Validate one complete ordinary Chat Completions response. Exactly one stopped
/// choice with nonempty text, no tool declarations, no fabricated usage defaults.
fn ordinary(v: Value) -> Result<Response> {
    let choices = v
        .get("choices")
        .and_then(Value::as_array)
        .filter(|c| c.len() == 1)
        .ok_or_else(|| Error::refused("api-response-choice-invalid"))?;
    let choice = &choices[0];
    if choice.get("finish_reason").and_then(Value::as_str) != Some("stop") {
        return Err(Error::refused("api-response-incomplete"));
    }
    let message = choice
        .get("message")
        .filter(|v| v.is_object())
        .ok_or_else(|| Error::refused("api-response-message-invalid"))?;
    text_only(message)?;
    let content = message
        .get("content")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| Error::refused("api-response-empty"))?
        .to_owned();
    let model = model(&v)?;
    let usage = v.get("usage").filter(|v| !v.is_null()).cloned();
    if usage.as_ref().is_some_and(|v| !v.is_object()) {
        return Err(Error::refused("api-response-usage-invalid"));
    }
    Ok(Response {
        content,
        usage,
        reported_model: model,
    })
}
/// Optional provider-reported model must be a nonempty string when present.
fn model(v: &Value) -> Result<Option<String>> {
    v.get("model")
        .filter(|v| !v.is_null())
        .map(|v| {
            v.as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| Error::refused("api-response-model-invalid"))
        })
        .transpose()
}
/// Incremental SSE parser keeps bytes until complete event boundaries, so UTF-8
/// split between network chunks remains valid. Reasoning is ignored and never
/// published. Each raw event and total logical content have separate saved caps.
struct Sse {
    buffer: Vec<u8>,
    content: String,
    usage: Option<Value>,
    model: Option<String>,
    stopped: bool,
    done: bool,
    cap: Option<usize>,
    event_cap: Option<usize>,
}
impl Sse {
    /// Initialize empty protocol state; caps are supplied, not adopted here.
    fn new(cap: Option<usize>, event_cap: Option<usize>) -> Self {
        Self {
            buffer: Vec::new(),
            content: String::new(),
            usage: None,
            model: None,
            stopped: false,
            done: false,
            cap,
            event_cap,
        }
    }
    /// Feed arbitrary transport bytes, parsing complete LF or CRLF events.
    /// A truncated event is retained until finish, where it refuses.
    fn feed(&mut self, bytes: &[u8]) -> Result<()> {
        // Process bytewise to enforce the event cap before allocating a whole
        // transport chunk; reqwest's received chunk itself is transport-owned.
        for byte in bytes {
            self.buffer
                .try_reserve(1)
                .map_err(|e| Error::caused("api-response-allocation", e))?;
            self.buffer.push(*byte);
            limit(self.buffer.len(), self.event_cap)?;
            if self.buffer.ends_with(b"\n\n") || self.buffer.ends_with(b"\n\r\n") {
                let event = std::mem::take(&mut self.buffer);
                self.event(&event)?;
            }
        }
        Ok(())
    }
    /// Parse one SSE event without publishing remote diagnostics or reasoning.
    fn event(&mut self, raw: &[u8]) -> Result<()> {
        let text =
            std::str::from_utf8(raw).map_err(|e| Error::caused("api-sse-utf8-invalid", e))?;
        let mut data = String::new();
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(value.strip_prefix(' ').unwrap_or(value));
            }
        }
        if data.is_empty() {
            return Ok(());
        }
        if self.done {
            return Err(Error::refused("api-sse-data-after-done"));
        }
        if data == "[DONE]" {
            if !self.stopped {
                return Err(Error::refused("api-sse-incomplete"));
            }
            self.done = true;
            return Ok(());
        }
        let v: Value = strict_json::decode(data.as_bytes()).map_err(Error::json)?;
        if v.get("error").is_some() {
            return Err(Error::refused("api-provider-declared-error"));
        }
        if let Some(model) = model(&v)? {
            if self.model.as_ref().is_some_and(|m| m != &model) {
                return Err(Error::refused("api-response-model-changed"));
            }
            self.model = Some(model);
        }
        if let Some(usage) = v.get("usage").filter(|v| !v.is_null()) {
            if !usage.is_object() || self.usage.as_ref().is_some_and(|u| u != usage) {
                return Err(Error::refused("api-response-usage-invalid"));
            }
            self.usage = Some(usage.clone());
        }
        let choices = v
            .get("choices")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::refused("api-response-choice-invalid"))?;
        if choices.is_empty() && v.get("usage").is_some_and(|v| v.is_object()) {
            return Ok(());
        }
        if choices.len() != 1 || choices[0].get("index").and_then(Value::as_u64) != Some(0) {
            return Err(Error::refused("api-response-choice-invalid"));
        }
        let c = &choices[0];
        let delta = c
            .get("delta")
            .filter(|d| d.is_object())
            .ok_or_else(|| Error::refused("api-response-delta-invalid"))?;
        text_only(delta)?;
        if let Some(content) = delta.get("content").filter(|v| !v.is_null()) {
            let content = content
                .as_str()
                .ok_or_else(|| Error::refused("api-response-content-invalid"))?;
            if self.stopped && !content.is_empty() {
                return Err(Error::refused("api-sse-content-after-stop"));
            }
            let total = self
                .content
                .len()
                .checked_add(content.len())
                .ok_or_else(|| Error::refused("api-response-over-limit"))?;
            limit(total, self.cap)?;
            self.content
                .try_reserve(content.len())
                .map_err(|e| Error::caused("api-response-allocation", e))?;
            self.content.push_str(content);
        }
        if let Some(reason) = c.get("finish_reason").filter(|v| !v.is_null()) {
            if self.stopped || reason.as_str() != Some("stop") {
                return Err(Error::refused("api-response-incomplete"));
            }
            self.stopped = true;
        }
        Ok(())
    }
    /// EOF is success only after stopped + DONE and complete nonempty content.
    fn finish(self) -> Result<Response> {
        if !self.done || !self.stopped || !self.buffer.is_empty() || self.content.trim().is_empty()
        {
            Err(Error::refused("api-sse-incomplete"))
        } else {
            Ok(Response {
                content: self.content,
                usage: self.usage,
                reported_model: self.model,
            })
        }
    }
}
/// Perform one configured text API call. TLS uses system trust; redirects and
/// protocol retries are disabled. No client deadline, max_tokens, tools, cookie
/// jar or model fallback is imposed. Caller must reserve/mark budget first and
/// cancel on saved timeout/stop. HTTP errors discard remote bodies; errors never
/// establish absent effects. JSON cap covers wire body; SSE cap covers content.
pub async fn complete(r: Request) -> Result<Response> {
    request(r,Operation::Completion,None).await
}
/// Explicit manual JSON probe permits optional credentials, unlike completion.
/// Caller must own the connection/destination admission, saved byte/header caps,
/// deadline/cancellation and the witness through all output/close failures.
/// Only HTTP200 complete text is accepted; no tools/retry/redirect or authority.
/// Cancellation is local and leaves remote execution/billing possibly unknown.
pub async fn probe(r:Request,contact:&ContactObservation)->Result<Response> {
    if !matches!(r.transport,Transport::Json){return Err(Error::refused("api-probe-transport-invalid"));}
    request(r,Operation::Probe,Some(contact)).await
}
/// Shared complete-text HTTP implementation. All parsing/client/body construction
/// errors precede the send witness; response/transport failures never imply rollback.
async fn request(r: Request,operation:Operation,contact:Option<&ContactObservation>) -> Result<Response> {
    let mut endpoint =
        reqwest::Url::parse(&r.base_url).map_err(|e| Error::caused("api-endpoint-invalid", e))?;
    if !matches!(endpoint.scheme(), "http" | "https")
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || r.model.is_empty()
        || (matches!(operation,Operation::Completion) && r.key.is_empty())
        || r.prompt.is_empty()
        || r.http1_max_headers == 0
        || r.http2_max_header_bytes == 0
    {
        return Err(Error::refused("api-request-invalid"));
    }
    endpoint.set_path(&format!(
        "{}/chat/completions",
        endpoint.path().trim_end_matches('/')
    ));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .referer(false)
        .http1_max_headers(r.http1_max_headers)
        .http2_max_header_list_size(r.http2_max_header_bytes)
        .no_gzip()
        .no_brotli()
        .no_zstd()
        .no_deflate()
        .pool_idle_timeout(None)
        .pool_max_idle_per_host(0)
        .build()
        .map_err(Error::transport)?;
    let stream = matches!(r.transport, Transport::Sse);
    let body = serde_json::to_vec(
        &json!({"model":r.model,"messages":[{"role":"user","content":r.prompt}],"stream":stream}),
    )
    .map_err(Error::json)?;
    let mut pending=client.post(endpoint).header(reqwest::header::CONTENT_TYPE,"application/json").body(body);
    if !r.key.is_empty(){pending=pending.bearer_auth(r.key);}
    if let Some(witness)=contact{witness.sending();}
    let mut response=pending.send().await.map_err(Error::transport)?;
    if !response.status().is_success() || (matches!(operation,Operation::Probe) && response.status()!=reqwest::StatusCode::OK) {
        return Err(Error::refused("api-http-status-refused"));
    }
    if stream {
        if !response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(';')
                    .next()
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("text/event-stream"))
            })
        {
            return Err(Error::refused("api-sse-content-type-invalid"));
        }
        let mut parser = Sse::new(r.response_max_bytes, r.event_max_bytes);
        while let Some(bytes) = response.chunk().await.map_err(Error::transport)? {
            parser.feed(&bytes)?;
        }
        parser.finish()
    } else {
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(Error::transport)? {
            let len = bytes
                .len()
                .checked_add(chunk.len())
                .ok_or_else(|| Error::refused("api-response-over-limit"))?;
            limit(len, r.response_max_bytes)?;
            bytes
                .try_reserve(chunk.len())
                .map_err(|e| Error::caused("api-response-allocation", e))?;
            bytes.extend_from_slice(&chunk);
        }
        ordinary(strict_json::decode(&bytes).map_err(Error::json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Public completion still refuses empty credentials before transport.
    /// An invalid manual probe likewise preserves an untouched contact witness;
    /// actual optional-auth success belongs to the loopback public-route fixture.
    #[tokio::test(flavor="current_thread")]
    async fn complete_credentials_and_probe_precontact_refusals_are_distinct() {
        let make=|base:&str| Request{base_url:base.into(),key:String::new(),model:"fixture".into(),prompt:"fixture only".into(),transport:Transport::Json,response_max_bytes:None,event_max_bytes:None,http1_max_headers:100,http2_max_header_bytes:16384};
        match complete(make("http://127.0.0.1:0/v1")).await {Err(e)=>assert_eq!(e.code(),"api-request-invalid"),Ok(_)=>panic!("empty credential completion accepted")}
        let observed=ContactObservation::default();
        match probe(make("\\0"),&observed).await {Err(e)=>assert_eq!(e.code(),"api-endpoint-invalid"),Ok(_)=>panic!("invalid probe accepted")}
        assert!(!observed.may_have_occurred());
    }
    /// Protocol fixture, not a saved product policy.
    fn event(content: Value, finish: Value) -> Vec<u8> {
        format!(
            "data: {}\n\n",
            json!({"choices":[{"index":0,"delta":content,"finish_reason":finish}]})
        )
        .into_bytes()
    }
    #[test]
    fn sse_fragmented_utf8_reasoning_and_usage() {
        let mut p = Sse::new(Some(100), Some(1000));
        let mut wire = event(
            json!({"reasoning_content":"PRIVATE_REASONING","content":"é"}),
            Value::Null,
        );
        wire.extend(event(json!({}), json!("stop")));
        wire.extend(b"data: [DONE]\n\n");
        for byte in wire {
            p.feed(&[byte]).unwrap();
        }
        let out = p.finish().unwrap();
        assert_eq!(out.content, "é");
        assert!(out.usage.is_none());
    }
    #[test]
    fn sse_partial_tools_and_caps_refuse() {
        let mut p = Sse::new(None, None);
        p.feed(&event(json!({"content":"partial"}), Value::Null))
            .unwrap();
        assert!(p.finish().is_err());
        let mut p = Sse::new(None, None);
        assert!(p
            .feed(&event(json!({"tool_calls":[]}), Value::Null))
            .is_err());
        let mut p = Sse::new(Some(0), None);
        assert!(p.feed(&event(json!({"content":"x"}), Value::Null)).is_err());
        let mut p = Sse::new(None, Some(0));
        assert!(p.feed(b"x").is_err());
    }
    #[test]
    fn duplicate_tool_declaration_must_not_be_erased() {
        let mut p = Sse::new(None, None);
        assert!(p.feed(b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[],\"tool_calls\":null,\"content\":\"unsafe\"},\"finish_reason\":\"stop\"}]}\n\n").is_err());
    }
}
