//! Candidate private raw retention; it grants no reply/process authority.
use crate::{AgentExecutionPolicy, AgentLaunchError as E};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Engine stream identity, not caller permission or operator configuration.
#[derive(Clone, Copy)]
pub(super) enum Channel {
    Stdout,
    Stderr,
}
/// Exact observed bytes may exceed retained bytes after explicit policy
/// truncation. The digest covers observed bytes, not only the retained prefix;
/// none of this authenticates provider content or proves process completion.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StreamSnapshot {
    pub retained: Vec<u8>,
    pub observed_bytes: u64,
    pub logical_lines: u64,
    pub sha256: String,
    pub truncated: bool,
}
/// One shared capture budget across both streams, in the owner's observed poll
/// order. This does not reconstruct a kernel-wide chronological interleaving.
/// Intentional truncation is not capture IO failure or invalid domain response.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    pub policy: AgentExecutionPolicy,
    pub retained_bytes: u64,
    pub observed_lines: u64,
    pub failed: bool,
    pub stdout: StreamSnapshot,
    pub stderr: StreamSnapshot,
}
impl Snapshot {
    /// Validate private receipt structure, not process/source/model authority.
    /// Successful capture must have exact counter sums, valid full hashes when
    /// untruncated and prefixes within the explicit saved caps. Failed capture
    /// may retain partial counters/hash state; it cannot prove completeness.
    /// No IO occurs; malformed policy/count/hash data keeps a safe typed refusal.
    pub(super) fn validate(&self) -> Result<(), E> {
        self.policy
            .validate()
            .map_err(|e| E::caused(e.reason(), e))?;
        let streams = [&self.stdout, &self.stderr];
        let retained = streams
            .iter()
            .try_fold(0u64, |n, s| n.checked_add(s.retained.len() as u64))
            .ok_or(E::Refused("review-capture-receipt-count"))?;
        if retained != self.retained_bytes
            || self
                .policy
                .capture_max_bytes
                .0
                .is_some_and(|n| retained > n.0 as u64)
        {
            return Err(E::Refused("review-capture-receipt-budget"));
        }
        let mut retained_lines = 0u64;
        for stream in streams {
            let bytes = stream.retained.len() as u64;
            let lines = stream.retained.iter().filter(|b| **b == b'\n').count() as u64
                + u64::from(stream.retained.last().is_some_and(|b| *b != b'\n'));
            retained_lines = retained_lines
                .checked_add(lines)
                .ok_or(E::Refused("review-capture-receipt-count"))?;
            if stream.sha256.len() != 64
                || !stream
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || stream.observed_bytes < bytes
                || stream.logical_lines < lines
            {
                return Err(E::Refused("review-capture-receipt-stream"));
            }
            if !self.failed
                && (stream.logical_lines > stream.observed_bytes
                    || (stream.observed_bytes > 0 && stream.logical_lines == 0)
                    || (!stream.truncated && stream.logical_lines != lines)
                    || (stream.truncated != (stream.observed_bytes > bytes))
                    || (!stream.truncated
                        && stream.sha256 != format!("{:x}", Sha256::digest(&stream.retained))))
            {
                return Err(E::Refused("review-capture-receipt-completeness"));
            }
        }
        if self
            .policy
            .capture_max_lines
            .0
            .is_some_and(|n| retained_lines > n.0 as u64)
            || (!self.failed
                && self
                    .stdout
                    .logical_lines
                    .checked_add(self.stderr.logical_lines)
                    != Some(self.observed_lines))
        {
            return Err(E::Refused("review-capture-receipt-lines"));
        }
        Ok(())
    }
}

struct Stream {
    bytes: Vec<u8>,
    observed: u64,
    lines: u64,
    open_line: bool,
    hash: Sha256,
    truncated: bool,
}
impl Stream {
    /// Empty actual observations; zero is not a provider usage/cost declaration.
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            observed: 0,
            lines: 0,
            open_line: false,
            hash: Sha256::new(),
            truncated: false,
        }
    }
    /// Clone private retained bytes/hash state for a receipt. Standard Vec/serde
    /// allocation behavior applies; the explicit saved caps bound retention,
    /// while null removes that ceiling. No decoder, EOF or operation occurs.
    fn snapshot(&self) -> StreamSnapshot {
        StreamSnapshot {
            retained: self.bytes.clone(),
            observed_bytes: self.observed,
            logical_lines: self.lines,
            sha256: format!("{:x}", self.hash.clone().finalize()),
            truncated: self.truncated,
        }
    }
}
/// Frozen saved execution policy and combined retention accounting. Callers
/// must bind the supplied policy to the job's actual saved generation; this
/// private value is not constructible from a public request or execution grant.
pub(super) struct Capture {
    policy: AgentExecutionPolicy,
    streams: [Stream; 2],
    retained: u64,
    lines: u64,
    failed: bool,
}
impl Capture {
    /// Validate the actual supplied saved policy without adoption or defaults.
    /// Zero captures no nonempty bytes; null removes its ceiling independently
    /// of role reply/event caps. No process, file, parser or budget is touched.
    pub(super) fn new(policy: AgentExecutionPolicy) -> Result<Self, E> {
        policy.validate().map_err(|e| E::caused(e.reason(), e))?;
        Ok(Self {
            policy,
            streams: [Stream::new(), Stream::new()],
            retained: 0,
            lines: 0,
            failed: false,
        })
    }
    /// Observe exact stdout/stderr bytes using one cumulative byte/line budget.
    /// Logical lines continue across chunks but start independently per stream.
    /// All bytes are hashed/counted even after intentional truncation. Checked
    /// representation/allocation failure latches failed and preserves its cause;
    /// counters then describe only the observations completed before that error.
    /// Caller must invalidate its reply transport and retain cleanup errors on
    /// any Err; successful truncation alone must not invalidate a complete reply.
    pub(super) fn record(&mut self, channel: Channel, raw: &[u8]) -> Result<(), E> {
        if self.failed {
            return Err(E::Refused("review-capture-already-failed"));
        }
        let result = (|| {
            let slot = match channel {
                Channel::Stdout => 0,
                Channel::Stderr => 1,
            };
            let stream = &mut self.streams[slot];
            let length =
                u64::try_from(raw.len()).map_err(|e| E::caused("review-capture-length", e))?;
            stream.observed = stream
                .observed
                .checked_add(length)
                .ok_or(E::Refused("review-capture-count-overflow"))?;
            stream.hash.update(raw);
            for byte in raw {
                if !stream.open_line {
                    stream.lines = stream
                        .lines
                        .checked_add(1)
                        .ok_or(E::Refused("review-capture-count-overflow"))?;
                    self.lines = self
                        .lines
                        .checked_add(1)
                        .ok_or(E::Refused("review-capture-count-overflow"))?;
                    stream.open_line = true;
                }
                let bytes_fit = self
                    .policy
                    .capture_max_bytes
                    .0
                    .is_none_or(|limit| self.retained < limit.0 as u64);
                let lines_fit = self
                    .policy
                    .capture_max_lines
                    .0
                    .is_none_or(|limit| self.lines <= limit.0 as u64);
                if bytes_fit && lines_fit {
                    stream
                        .bytes
                        .try_reserve(1)
                        .map_err(|e| E::caused("review-capture-allocation", e))?;
                    self.retained = self
                        .retained
                        .checked_add(1)
                        .ok_or(E::Refused("review-capture-count-overflow"))?;
                    stream.bytes.push(*byte);
                } else {
                    stream.truncated = true;
                }
                if *byte == b'\n' {
                    stream.open_line = false;
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    /// Freeze private retained prefixes and original observed hashes/counters.
    /// Failed=true means counters/completeness are not a successful full capture;
    /// truncation is separate and never means failed provider or valid verdict.
    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            policy: self.policy.clone(),
            retained_bytes: self.retained,
            observed_lines: self.lines,
            failed: self.failed,
            stdout: self.streams[0].snapshot(),
            stderr: self.streams[1].snapshot(),
        }
    }
}

/// Synchronous owned-runner adapters: the same capture observes both channels,
/// while every stdout byte reaches the independent role-capped decoder even
/// after intentional raw truncation. The caller supplies the actual saved
/// policy and a source-bound reply decoder; no authorization is inferred here.
/// Not thread-safe; sinks must not be recursively invoked while borrowed.
pub(super) struct Transport<'a> {
    state: std::cell::RefCell<TransportState<'a>>,
}
struct TransportState<'a> {
    capture: Capture,
    reply: crate::ReviewOutputStream<'a>,
    failed: bool,
}
/// Borrowed single-channel adapter for the owned runner's synchronous Write
/// contract. IO errors preserve typed causes; neither write nor flush is EOF.
pub(super) struct Sink<'owner, 'source> {
    owner: &'owner Transport<'source>,
    channel: Channel,
}
impl<'a> Transport<'a> {
    /// Validate explicit capture policy before returning any sink. Reply policy
    /// and source binding remain the caller's obligation. No IO, default policy,
    /// process, ledger mutation or implicit completion occurs.
    pub(super) fn new(
        policy: AgentExecutionPolicy,
        reply: crate::ReviewOutputStream<'a>,
    ) -> Result<Self, E> {
        Ok(Self {
            state: std::cell::RefCell::new(TransportState {
                capture: Capture::new(policy)?,
                reply,
                failed: false,
            }),
        })
    }
    /// Return stdout/stderr adapters spending one shared budget. Observed order
    /// follows the runner, not a claimed total ordering of kernel pipe writes.
    pub(super) fn sinks(&self) -> (Sink<'_, 'a>, Sink<'_, 'a>) {
        (
            Sink {
                owner: self,
                channel: Channel::Stdout,
            },
            Sink {
                owner: self,
                channel: Channel::Stderr,
            },
        )
    }
    /// Recover raw evidence and decoder after the owner drops both sinks.
    /// This does not finish/validate the reply: the owner must first establish
    /// actual EOF/wait and invalidate transport on runner/drain failures.
    /// Raw capture or parser failure has already latched decoder incompleteness.
    pub(super) fn into_parts(self) -> (Snapshot, crate::ReviewOutputStream<'a>) {
        let state = self.state.into_inner();
        (state.capture.snapshot(), state.reply)
    }
}
impl std::io::Write for Sink<'_, '_> {
    /// Count/hash exact input first, then decode stdout regardless of intentional
    /// truncation. A genuine capture/parser failure invalidates the reply and
    /// preserves its typed cause as an IO error; subsequent writes refuse.
    fn write(&mut self, raw: &[u8]) -> std::io::Result<usize> {
        let mut state = self.owner.state.borrow_mut();
        if state.failed {
            return Err(std::io::Error::other(E::Refused(
                "review-stream-already-failed",
            )));
        }
        let result = state
            .capture
            .record(self.channel, raw)
            .and_then(|()| match self.channel {
                Channel::Stdout => state.reply.feed(raw),
                Channel::Stderr => Ok(()),
            });
        match result {
            Ok(()) => Ok(raw.len()),
            Err(error) => {
                state.failed = true;
                state.reply.incomplete_transport();
                Err(std::io::Error::other(error))
            }
        }
    }
    /// Memory-only flush; no EOF, persistence, provider success or verdict.
    /// A previous adapter failure remains an error rather than empty success.
    fn flush(&mut self) -> std::io::Result<()> {
        if self.owner.state.borrow().failed {
            Err(std::io::Error::other(E::Refused(
                "review-stream-already-failed",
            )))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Saved installation profile, then explicit fixture choices; no runtime
    /// policy fallback or test watchdog is introduced by these values.
    fn capture(bytes: Option<i64>, lines: Option<i64>) -> Capture {
        let mut policy = crate::EngineSettingsValues::installation_defaults()
            .unwrap()
            .agent_execution_policy()
            .unwrap();
        policy.capture_max_bytes = crate::SettingCap(bytes.map(crate::SettingCount));
        policy.capture_max_lines = crate::SettingCap(lines.map(crate::SettingCount));
        Capture::new(policy).unwrap()
    }
    /// Both channels spend one cap, never two copies of the configured budget;
    /// hashes/counts still include discarded bytes and truncation is not failure.
    #[test]
    fn reviewer_raw_capture_shared_byte_budget_and_full_observed_hash() {
        let mut c = capture(Some(5), None);
        c.record(Channel::Stdout, b"abc").unwrap();
        c.record(Channel::Stderr, b"def").unwrap();
        let s = c.snapshot();
        assert_eq!(s.stdout.retained, b"abc");
        assert_eq!(s.stderr.retained, b"de");
        assert_eq!(s.retained_bytes, 5);
        assert_eq!(s.stderr.observed_bytes, 3);
        assert_eq!(s.stderr.sha256, format!("{:x}", Sha256::digest(b"def")));
        assert!(!s.failed && s.stderr.truncated && !s.stdout.truncated);
    }
    /// Zero and null are independent explicit choices, not empty-success defaults.
    #[test]
    fn reviewer_raw_capture_zero_and_unlimited_preserve_observation() {
        for (bytes, lines) in [(Some(0), None), (None, Some(0))] {
            let mut c = capture(bytes, lines);
            c.record(Channel::Stdout, b"x\n").unwrap();
            let s = c.snapshot();
            assert!(s.stdout.retained.is_empty());
            assert_eq!(s.stdout.observed_bytes, 2);
            assert!(s.stdout.truncated && !s.failed);
        }
        let mut c = capture(None, None);
        c.record(Channel::Stdout, b"x\n").unwrap();
        c.record(Channel::Stderr, b"y\n").unwrap();
        let s = c.snapshot();
        assert_eq!(s.retained_bytes, 4);
        assert!(!s.stdout.truncated && !s.stderr.truncated);
    }
    /// Per-stream unfinished lines survive chunks; the cumulative line cap is
    /// shared in observed order. Empty writes create neither lines nor bytes.
    #[test]
    fn reviewer_raw_capture_chunked_lines_are_shared_without_reset() {
        let mut c = capture(None, Some(2));
        c.record(Channel::Stdout, b"a").unwrap();
        c.record(Channel::Stdout, b"b\n").unwrap();
        c.record(Channel::Stderr, b"c").unwrap();
        c.record(Channel::Stderr, b"d\n").unwrap();
        c.record(Channel::Stdout, b"e\n").unwrap();
        c.record(Channel::Stderr, b"").unwrap();
        let s = c.snapshot();
        assert_eq!(s.stdout.retained, b"ab\n");
        assert_eq!(s.stderr.retained, b"cd\n");
        assert_eq!(s.observed_lines, 3);
        assert_eq!(s.stdout.logical_lines, 2);
        assert!(s.stdout.truncated && !s.failed);
    }
    /// Invalid saved policy refuses; technical counter overflow latches failure
    /// and a later empty write cannot turn that error into a successful capture.
    #[test]
    fn reviewer_raw_capture_invalid_policy_and_counter_failure_latch() {
        let mut policy = crate::EngineSettingsValues::installation_defaults()
            .unwrap()
            .agent_execution_policy()
            .unwrap();
        policy.capture_max_bytes = crate::SettingCap(Some(crate::SettingCount(-1)));
        assert!(Capture::new(policy).is_err());
        let mut c = capture(None, None);
        c.record(Channel::Stdout, b"valid").unwrap();
        c.streams[0].observed = u64::MAX;
        assert!(c.record(Channel::Stdout, b"x").is_err());
        assert!(c.snapshot().failed);
        assert!(c.record(Channel::Stdout, b"").is_err());
    }
    const REPLY: &str = r#"{"reason":"Valid rationale","criteria":[{"index":1,"verdict":"pass","evidence":"Original exact evidence"}]}"#;
    /// Exact inert provider protocol fixture, not a policy or model assertion.
    fn provider_bytes() -> Vec<u8> {
        format!("{}\n{}\n", serde_json::json!({"type":"item.completed","item":{"type":"agent_message","id":"reply","text":REPLY}}), serde_json::json!({"type":"turn.completed","usage":{"input_tokens":7,"output_tokens":8}})).into_bytes()
    }
    /// A zero raw cap must not disable or truncate independently capped review
    /// decoding; stderr is never fed to the JSONL parser. Flush is not EOF.
    #[test]
    fn reviewer_raw_capture_streaming_truncation_preserves_full_reply() {
        use std::io::Write;
        let values = crate::EngineSettingsValues::installation_defaults().unwrap();
        let originals = [crate::ReviewQuoteOriginal {
            name: "report.md",
            kind: "report",
            content: "Original exact evidence",
        }];
        let reply = crate::ReviewOutputStream::from_saved_settings(&values, 1, &originals).unwrap();
        let mut policy = values.agent_execution_policy().unwrap();
        policy.capture_max_bytes = crate::SettingCap(Some(crate::SettingCount(0)));
        let transport = Transport::new(policy, reply).unwrap();
        let (mut out, mut err) = transport.sinks();
        err.write_all(b"non JSON private stderr\n").unwrap();
        for chunk in provider_bytes().chunks(7) {
            out.write_all(chunk).unwrap();
        }
        out.flush().unwrap();
        err.flush().unwrap();
        let (raw, mut reply) = transport.into_parts();
        assert_eq!(raw.retained_bytes, 0);
        assert!(raw.stdout.truncated && raw.stderr.truncated && !raw.failed);
        assert_eq!(reply.finish().unwrap().transport.response, REPLY.as_bytes());
    }
    /// Healthy same-policy witness precedes an injected representation failure.
    /// Even a complete-looking decoder is invalidated by stderr capture failure;
    /// original typed IO cause survives and later writes/flush cannot succeed.
    #[test]
    fn reviewer_raw_capture_streaming_failure_invalidates_complete_reply() {
        use std::io::Write;
        reviewer_raw_capture_streaming_truncation_preserves_full_reply();
        let values = crate::EngineSettingsValues::installation_defaults().unwrap();
        let originals = [crate::ReviewQuoteOriginal {
            name: "report.md",
            kind: "report",
            content: "Original exact evidence",
        }];
        let reply = crate::ReviewOutputStream::from_saved_settings(&values, 1, &originals).unwrap();
        let transport = Transport::new(values.agent_execution_policy().unwrap(), reply).unwrap();
        let (mut out, mut err) = transport.sinks();
        out.write_all(&provider_bytes()).unwrap();
        transport.state.borrow_mut().capture.streams[1].observed = u64::MAX;
        let failure = err.write_all(b"x").unwrap_err();
        assert!(failure.get_ref().unwrap().downcast_ref::<E>().is_some());
        assert!(failure
            .to_string()
            .contains("review-capture-count-overflow"));
        assert!(out.write(b"").is_err());
        assert!(err.flush().is_err());
        let (raw, mut reply) = transport.into_parts();
        assert!(raw.failed);
        assert!(reply.finish().is_err());
        assert_eq!(reply.retained_reply(), Some(REPLY.as_bytes()));
    }
    /// Malformed stdout fails transport with its typed cause and retains the
    /// exact already observed raw bytes; a later clean empty write is refused.
    #[test]
    fn reviewer_raw_capture_streaming_parser_failure_keeps_raw() {
        use std::io::Write;
        reviewer_raw_capture_streaming_truncation_preserves_full_reply();
        let values = crate::EngineSettingsValues::installation_defaults().unwrap();
        let reply = crate::ReviewOutputStream::from_saved_settings(&values, 1, &[]).unwrap();
        let transport = Transport::new(values.agent_execution_policy().unwrap(), reply).unwrap();
        let (mut out, mut err) = transport.sinks();
        let failure = out.write_all(b"not-json\n").unwrap_err();
        assert!(failure.get_ref().unwrap().downcast_ref::<E>().is_some());
        assert!(err.write(b"").is_err());
        let (raw, mut reply) = transport.into_parts();
        assert_eq!(raw.stdout.retained, b"not-json\n");
        assert!(!raw.failed); // Capture succeeded; the independent decoder failed.
        assert!(reply.finish().is_err());
    }
    /// Same-policy witness before forged counters, hash and budget checks; these
    /// are private schema refusals, never content authenticity or fresh authority.
    #[test]
    fn reviewer_raw_capture_snapshot_rejects_counter_hash_and_policy_forgery() {
        let mut c = capture(Some(5), Some(2));
        c.record(Channel::Stdout, b"abc").unwrap();
        c.record(Channel::Stderr, b"deff").unwrap();
        c.snapshot().validate().unwrap();
        let mut bad = c.snapshot();
        bad.retained_bytes += 1;
        assert!(bad.validate().is_err());
        let mut bad = c.snapshot();
        bad.stdout.logical_lines += 1;
        bad.observed_lines += 1;
        assert!(
            bad.validate().is_err(),
            "self-consistent forged line counter accepted"
        );
        let mut bad = c.snapshot();
        bad.stdout.sha256 = "0".repeat(64);
        assert!(bad.validate().is_err());
        let mut bad = c.snapshot();
        bad.stderr.truncated = false;
        assert!(bad.validate().is_err());
        let mut bad = c.snapshot();
        bad.observed_lines += 1;
        assert!(bad.validate().is_err());
        let mut bad = c.snapshot();
        bad.policy.capture_max_bytes = crate::SettingCap(Some(crate::SettingCount(0)));
        assert!(bad.validate().is_err());
    }
    /// Partial failed evidence may be retained, while no complete capture is
    /// claimed. Zero capture still requires a digest of observed provider bytes.
    #[test]
    fn reviewer_raw_capture_snapshot_zero_and_partial_failure_are_distinct() {
        let mut c = capture(Some(0), None);
        c.record(Channel::Stdout, b"private").unwrap();
        c.snapshot().validate().unwrap();
        let mut bad = c.snapshot();
        bad.stdout.logical_lines = 0;
        bad.observed_lines = 0;
        assert!(bad.validate().is_err());
        c.streams[1].observed = u64::MAX;
        assert!(c.record(Channel::Stderr, b"x").is_err());
        let s = c.snapshot();
        assert!(s.failed);
        s.validate().unwrap();
    }
}
