//! Deterministic observations over declared tool events; no process/permission.
use crate::{AgentGuardSnapshot, RunLimits};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    fmt,
};
/// Safe code plus original typed parse/policy/encoding cause. No raw tool input,
/// freeform technical message or metadata is printed by Display or Debug.
pub struct AgentGuardError {
    code: &'static str,
    cause: Option<Box<dyn Error + Send + Sync>>,
}
impl AgentGuardError {
    /// Refuse a known contract boundary without echoing supplied data.
    pub(crate) fn refused(code: &'static str) -> Self {
        Self { code, cause: None }
    }
    /// Preserve a typed cause while rendering only its static safe family.
    pub(crate) fn caused(code: &'static str, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            code,
            cause: Some(Box::new(cause)),
        }
    }
    /// Stable safe reason, never an execution receipt or authorization.
    pub fn code(&self) -> &'static str {
        self.code
    }
}
impl fmt::Display for AgentGuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Debug for AgentGuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for AgentGuardError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}
/// A reason derived from observed counters only. A supervisor must still own and
/// authorize any stop; this enum signals neither termination nor task acceptance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentGuardStop {
    ToolCalls,
    IdenticalCalls,
    InterleavedFailures,
    ConsecutiveErrors,
}
/// Loss of event attribution, not evidence that the provider is dead or idle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentGuardVisibility {
    MalformedEvent,
    MissingEvents,
    StreamFailure,
}
/// Reconstructed counters, with output/tool state separate from vitality. Times
/// are supplied monotonic milliseconds; they are not authenticated wall clocks.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AgentGuardObservation {
    pub calls: i64,
    pub results: i64,
    pub pending: i64,
    pub consecutive_identical: i64,
    pub consecutive_errors: i64,
    pub total_errors: i64,
    pub total_repeats: i64,
    pub reads: i64,
    pub writes: i64,
    pub unclassified: i64,
    pub output_state: &'static str,
    pub tool_state: &'static str,
    pub provider_vitality: &'static str,
    pub silence_threshold_exceeded: bool,
    pub tool_time_threshold_exceeded: bool,
    pub oldest_tool_wait_millis: i64,
    pub stop_reason: Option<AgentGuardStop>,
    pub visibility: Option<AgentGuardVisibility>,
}
#[derive(Clone)]
struct Pending {
    signature: [u8; 32],
    started: i64,
}
#[derive(Clone)]
struct Failed {
    signature: [u8; 32],
    completed: i64,
}
/// Private state retains hashes, not arguments or freeform error strings. Limits
/// come from the attempt, never current defaults. Policy is an explicit snapshot.
/// It observes provider declarations and cannot prove actual OS tool execution.
#[derive(Clone)]
pub struct AgentToolGuard {
    limits: RunLimits,
    window: Option<u128>,
    last_at: i64,
    last_output: Option<i64>,
    seen: HashMap<String, [u8; 32]>,
    pending: HashMap<String, Pending>,
    signatures: HashMap<[u8; 32], i64>,
    completed: HashMap<String, bool>,
    failures: Vec<Failed>,
    last_signature: Option<[u8; 32]>,
    calls: i64,
    results: i64,
    repeated: i64,
    errors: i64,
    total_errors: i64,
    total_repeats: i64,
    reads: i64,
    writes: i64,
    unclassified: i64,
    reason: Option<AgentGuardStop>,
    visibility: Option<AgentGuardVisibility>,
}
impl fmt::Debug for AgentToolGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AgentToolGuard { data: redacted }")
    }
}
/// Checked representation increment. Overflow refuses instead of wrapping or
/// resetting historical consumption; caller calculates all changes first.
fn increment(value: i64) -> Result<i64, AgentGuardError> {
    value
        .checked_add(1)
        .ok_or_else(|| AgentGuardError::refused("agent-guard-counter-overflow"))
}
/// Validate complete provider JSON, including overwritten duplicate values and
/// Go float64 ranges, without constructing or dropping a recursive Value tree.
/// Returns borrowed-span boundaries, not authentication or event attribution.
/// Syntax/depth/range errors preserve their typed cause under a safe code.
pub(crate) fn validated_json(
    input: &[u8],
) -> Result<crate::model_wire::Boundaries, AgentGuardError> {
    use crate::model_wire::{array, object, Boundaries};
    swarm_wire_json::decode::<serde::de::IgnoredAny>(input)
        .map_err(|e| AgentGuardError::caused("agent-guard-input-invalid", e))?;
    let boundaries = Boundaries::new(input)
        .map_err(|e| AgentGuardError::caused("agent-guard-input-invalid", e))?;
    // Go's any decoder checks all values before duplicate keys overwrite them.
    // Validate numeric range even in an older duplicate value that hashing skips.
    // This uses shared boundaries and borrowed spans, not recursive Value trees.
    let mut validate = vec![input.trim_ascii()];
    while let Some(raw) = validate.pop() {
        match raw.first() {
            Some(b'{') => validate.extend(
                object(raw, &boundaries)
                    .map_err(|e| AgentGuardError::caused("agent-guard-input-invalid", e))?
                    .into_iter()
                    .map(|(_, value)| value),
            ),
            Some(b'[') => validate.extend(
                array(raw, &boundaries)
                    .map_err(|e| AgentGuardError::caused("agent-guard-input-invalid", e))?,
            ),
            Some(b'-' | b'0'..=b'9') => {
                swarm_wire_json::decode::<crate::GoFloat>(raw)
                    .map_err(|e| AgentGuardError::caused("agent-guard-input-invalid", e))?;
            }
            _ => {}
        }
    }
    Ok(boundaries)
}
/// Hash Go's `[name,input]` signature using original spans and an iterative stack.
/// Existing JSON syntax/depth validation and boundary scanner avoid recursive
/// Value construction/drop. Map keys sort, duplicates use last value, all input
/// numbers use Go float64 formatting. Parse/range/encoding errors retain causes.
fn signature(name: &str, input: &[u8]) -> Result<[u8; 32], AgentGuardError> {
    use crate::model_wire::{array, object};
    let boundaries = validated_json(input)?;
    enum Part<'a> {
        Value(&'a [u8]),
        Byte(u8),
        Key(String),
    }
    let mut hash = Sha256::new();
    hash.update(b"[");
    hash.update(
        crate::encode_model(&name)
            .map_err(|e| AgentGuardError::caused("agent-guard-input-encoding", e))?,
    );
    hash.update(b",");
    let mut stack = vec![Part::Value(input.trim_ascii())];
    while let Some(part) = stack.pop() {
        match part {
            Part::Byte(value) => hash.update([value]),
            Part::Key(key) => hash.update(
                crate::encode_model(&key)
                    .map_err(|e| AgentGuardError::caused("agent-guard-input-encoding", e))?,
            ),
            Part::Value(raw) => {
                match raw.first() {
                    Some(b'{') => {
                        let entries: BTreeMap<_, _> = object(raw, &boundaries)
                            .map_err(|e| AgentGuardError::caused("agent-guard-input-invalid", e))?
                            .into_iter()
                            .collect();
                        hash.update(b"{");
                        stack.push(Part::Byte(b'}'));
                        for (i, (key, value)) in entries.into_iter().enumerate().rev() {
                            stack.push(Part::Value(value));
                            stack.push(Part::Byte(b':'));
                            stack.push(Part::Key(key));
                            if i > 0 {
                                stack.push(Part::Byte(b','));
                            }
                        }
                    }
                    Some(b'[') => {
                        let entries = array(raw, &boundaries)
                            .map_err(|e| AgentGuardError::caused("agent-guard-input-invalid", e))?;
                        hash.update(b"[");
                        stack.push(Part::Byte(b']'));
                        for (i, value) in entries.into_iter().enumerate().rev() {
                            stack.push(Part::Value(value));
                            if i > 0 {
                                stack.push(Part::Byte(b','));
                            }
                        }
                    }
                    Some(b'"') => {
                        let value: String = swarm_wire_json::decode(raw)
                            .map_err(|e| AgentGuardError::caused("agent-guard-input-invalid", e))?;
                        hash.update(crate::encode_model(&value).map_err(|e| {
                            AgentGuardError::caused("agent-guard-input-encoding", e)
                        })?);
                    }
                    Some(b't' | b'f' | b'n') => hash.update(raw),
                    _ => {
                        let value: crate::GoFloat = swarm_wire_json::decode(raw)
                            .map_err(|e| AgentGuardError::caused("agent-guard-input-invalid", e))?;
                        hash.update(crate::encode_model(&value).map_err(|e| {
                            AgentGuardError::caused("agent-guard-input-encoding", e)
                        })?);
                    }
                }
            }
        }
    }
    hash.update(b"]");
    Ok(hash.finalize().into())
}
impl AgentToolGuard {
    /// Construct from already frozen positive limits and explicit native snapshot.
    /// Missing/invalid policy and zero/negative historical limits refuse without
    /// current-default normalization. Durations observe only; no timer is started.
    pub fn new(limits: RunLimits, snapshot: &AgentGuardSnapshot) -> Result<Self, AgentGuardError> {
        if !matches!(limits.observation_mode, 0 | 1)
            || [
                limits.silence_seconds,
                limits.tool_seconds,
                limits.max_tool_calls,
                limits.max_repeated_calls,
                limits.max_consecutive_errors,
            ]
            .iter()
            .any(|v| *v < 1)
        {
            return Err(AgentGuardError::refused(
                "agent-guard-frozen-limits-invalid",
            ));
        }
        let policy = snapshot
            .policy()
            .map_err(|e| AgentGuardError::caused("agent-guard-snapshot-invalid", e))?;
        let window = policy
            .multiplier()
            .map(|v| (v as u128) * (limits.max_repeated_calls as u128));
        Ok(Self {
            limits,
            window,
            last_at: 0,
            last_output: None,
            seen: HashMap::new(),
            pending: HashMap::new(),
            signatures: HashMap::new(),
            completed: HashMap::new(),
            failures: vec![],
            last_signature: None,
            calls: 0,
            results: 0,
            repeated: 0,
            errors: 0,
            total_errors: 0,
            total_repeats: 0,
            reads: 0,
            writes: 0,
            unclassified: 0,
            reason: None,
            visibility: None,
        })
    }
    /// Check monotonic nonnegative supplied time before changing any state. It is
    /// an observation coordinate, never a deadline, retry or provider-health proof.
    fn clock(&self, at: i64) -> Result<(), AgentGuardError> {
        if at < self.last_at || at < 0 {
            Err(AgentGuardError::refused("agent-guard-clock-regressed"))
        } else {
            Ok(())
        }
    }
    /// Observe stream bytes separately from tools. This proves output only.
    pub fn output(&mut self, at: i64) -> Result<(), AgentGuardError> {
        self.clock(at)?;
        self.last_at = at;
        self.last_output = Some(at);
        Ok(())
    }
    /// Observe one attributed call. Same ID/signature is idempotent; changing its
    /// input/name refuses before counters/time change. Unknown names are counted
    /// unclassified, never inferred from commands. Input/hash failures are atomic.
    pub fn call(
        &mut self,
        at: i64,
        id: &str,
        name: &str,
        input: &[u8],
    ) -> Result<(), AgentGuardError> {
        self.clock(at)?;
        if id.is_empty() || name.is_empty() {
            return Err(AgentGuardError::refused(
                "agent-guard-call-attribution-missing",
            ));
        }
        let sig = signature(name, input)?;
        if let Some(prior) = self.seen.get(id) {
            if *prior != sig {
                return Err(AgentGuardError::refused("agent-guard-call-id-conflict"));
            }
            return self.output(at);
        }
        let calls = increment(self.calls)?;
        let repeated = if self.last_signature == Some(sig) {
            increment(self.repeated)?
        } else {
            1
        };
        let seen = self.signatures.get(&sig).copied().unwrap_or(0);
        let occurrence = increment(seen)?;
        let total_repeats = if seen > 0 {
            increment(self.total_repeats)?
        } else {
            self.total_repeats
        };
        let (mut reads, mut writes, mut unclassified) =
            (self.reads, self.writes, self.unclassified);
        match name {
            "Read" | "Glob" | "Grep" => reads = increment(reads)?,
            "Write" | "Edit" | "MultiEdit" | "file_change" => writes = increment(writes)?,
            _ => unclassified = increment(unclassified)?,
        }
        self.last_at = at;
        self.last_output = Some(at);
        self.calls = calls;
        self.repeated = repeated;
        self.total_repeats = total_repeats;
        self.reads = reads;
        self.writes = writes;
        self.unclassified = unclassified;
        self.last_signature = Some(sig);
        self.signatures.insert(sig, occurrence);
        self.seen.insert(id.into(), sig);
        self.pending.insert(
            id.into(),
            Pending {
                signature: sig,
                started: at,
            },
        );
        if self.limits.observation_mode == 0 {
            if self.reason.is_none() && self.calls > self.limits.max_tool_calls {
                self.reason = Some(AgentGuardStop::ToolCalls);
            }
            if self.reason.is_none() && self.repeated >= self.limits.max_repeated_calls {
                self.reason = Some(AgentGuardStop::IdenticalCalls);
            }
        }
        Ok(())
    }
    /// Match a declared result to its pending call. Freeform error text is not an
    /// authority input: infrastructure incidents belong to the engine/supervisor,
    /// not a substring exemption here. A true result flag increments observed
    /// errors; success resets only the streak, never cumulative counters. Unknown
    /// result/clock/overflow refuses atomically; the last authorized call may finish.
    /// An identical result is idempotent; a changed outcome under its ID refuses.
    /// Later observations retain the original stop reason and cumulative facts.
    pub fn result(&mut self, at: i64, id: &str, failed: bool) -> Result<(), AgentGuardError> {
        self.clock(at)?;
        let Some(pending) = self.pending.get(id) else {
            return match self.completed.get(id) {
                Some(previous) if *previous == failed => self.output(at),
                Some(_) => Err(AgentGuardError::refused("agent-guard-result-id-conflict")),
                None => Err(AgentGuardError::refused("agent-guard-result-unmatched")),
            };
        };
        let sig = pending.signature;
        let completed = increment(self.results)?;
        let errors = if failed { increment(self.errors)? } else { 0 };
        let total_errors = if failed {
            increment(self.total_errors)?
        } else {
            self.total_errors
        };
        let mut recent = Vec::new();
        let mut same = 0i64;
        for old in &self.failures {
            let expired = self
                .window
                .is_some_and(|window| ((completed - old.completed) as u128) >= window);
            if expired || (!failed && old.signature == sig) {
                continue;
            }
            if old.signature == sig {
                same = increment(same)?;
            }
            recent.push(old.clone());
        }
        let occurrences = if failed { increment(same)? } else { same };
        if failed {
            recent.push(Failed {
                signature: sig,
                completed,
            });
        }
        self.last_at = at;
        self.last_output = Some(at);
        self.results = completed;
        self.errors = errors;
        self.total_errors = total_errors;
        self.failures = recent;
        self.pending.remove(id);
        self.completed.insert(id.into(), failed);
        if self.limits.observation_mode == 0 {
            if self.reason.is_none()
                && self.calls >= self.limits.max_tool_calls
                && self.pending.is_empty()
            {
                self.reason = Some(AgentGuardStop::ToolCalls);
            }
            if self.reason.is_none() && failed && occurrences >= self.limits.max_repeated_calls {
                self.reason = Some(AgentGuardStop::InterleavedFailures);
            }
            if self.reason.is_none() && self.errors >= self.limits.max_consecutive_errors {
                self.reason = Some(AgentGuardStop::ConsecutiveErrors);
            }
        }
        Ok(())
    }
    /// Mark missing/invalid event visibility, preserving cumulative facts and any
    /// previously established stop reason. Streak assumptions/history are invalid
    /// across a gap; pending tools remain unknown, never silently reported idle.
    pub fn lose_visibility(
        &mut self,
        at: i64,
        cause: AgentGuardVisibility,
    ) -> Result<(), AgentGuardError> {
        self.clock(at)?;
        self.last_at = at;
        if self.visibility.is_none() {
            self.visibility = Some(cause);
        }
        self.errors = 0;
        self.repeated = 0;
        self.failures.clear();
        Ok(())
    }
    /// Report counters at a nonregressing observation time. Silence/wait thresholds
    /// never generate stop reasons and vitality is always unknown. Pending count
    /// conversion errors refuse; this makes no writes or state/time mutation.
    pub fn observation(&self, at: i64) -> Result<AgentGuardObservation, AgentGuardError> {
        self.clock(at)?;
        let pending = i64::try_from(self.pending.len())
            .map_err(|e| AgentGuardError::caused("agent-guard-counter-representation", e))?;
        let silence = self.last_output.is_some_and(|last| {
            ((at - last) as u128) >= (self.limits.silence_seconds as u128) * 1000
        });
        let oldest = if self.visibility.is_none() {
            self.pending
                .values()
                .map(|p| at - p.started)
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        Ok(AgentGuardObservation {
            calls: self.calls,
            results: self.results,
            pending,
            consecutive_identical: self.repeated,
            consecutive_errors: self.errors,
            total_errors: self.total_errors,
            total_repeats: self.total_repeats,
            reads: self.reads,
            writes: self.writes,
            unclassified: self.unclassified,
            output_state: if self.last_output.is_none() {
                "none"
            } else if silence {
                "silent"
            } else {
                "recent"
            },
            tool_state: if self.visibility.is_some() {
                "unknown"
            } else if pending > 0 {
                "waiting"
            } else {
                "idle"
            },
            provider_vitality: "unknown",
            silence_threshold_exceeded: silence,
            tool_time_threshold_exceeded: self.visibility.is_none()
                && pending > 0
                && (oldest as u128) >= (self.limits.tool_seconds as u128) * 1000,
            oldest_tool_wait_millis: oldest,
            stop_reason: if self.limits.observation_mode == 1 {
                None
            } else {
                self.reason
            },
            visibility: self.visibility,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Representation-limit injection after a valid control. Overflow refuses
    /// before clocks/counters/pending mutate; it never wraps historical counts.
    #[test]
    fn counter_overflow_refuses_atomically() {
        let limits = RunLimits {
            silence_seconds: 1,
            tool_seconds: 1,
            max_tool_calls: 10,
            max_repeated_calls: 10,
            max_consecutive_errors: 10,
            observation_mode: 0,
        };
        let snapshot = AgentGuardSnapshot {
            schema_version: 1,
            settings_revision: "1".into(),
            lookback_multiplier: crate::SettingCap(None),
            extensions: BTreeMap::new(),
        };
        let mut guard = AgentToolGuard::new(limits, &snapshot).unwrap();
        guard.call(0, "valid", "Read", b"null").unwrap();
        guard.result(1, "valid", false).unwrap();
        guard.calls = i64::MAX;
        let before = guard.observation(1).unwrap();
        assert_eq!(
            guard
                .call(2, "overflow", "Read", b"null")
                .unwrap_err()
                .code(),
            "agent-guard-counter-overflow"
        );
        assert_eq!(guard.observation(1).unwrap(), before);
        guard.calls = 1;
        guard.call(2, "pending", "Read", b"null").unwrap();
        guard.results = i64::MAX;
        let before = guard.observation(2).unwrap();
        assert_eq!(
            guard.result(3, "pending", true).unwrap_err().code(),
            "agent-guard-counter-overflow"
        );
        assert_eq!(guard.observation(2).unwrap(), before);
        guard.results = 1;
        guard.total_errors = i64::MAX;
        let before = guard.observation(2).unwrap();
        assert_eq!(
            guard.result(3, "pending", true).unwrap_err().code(),
            "agent-guard-counter-overflow"
        );
        assert_eq!(guard.observation(2).unwrap(), before);
        guard.total_errors = 0;
        guard.result(3, "pending", true).unwrap();
        assert_eq!(guard.observation(3).unwrap().pending, 0);
    }
}
