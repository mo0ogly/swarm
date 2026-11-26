//! Safe observations in the producer's existing mission journal.
use super::{
    store::Job,
    transaction::{self, Change},
};
use crate::{RootStore, RootStoreError as E};
use serde::Serialize;
use std::{cell::RefCell, time::Instant};

/// Collected safe observation failures, separate from CLI and provider streams.
pub(super) type Failures = RefCell<Vec<&'static str>>;

/// Numeric observations only. Missing values were not measured. A null limit
/// in a populated limit set denotes the existing unlimited setting.
#[derive(Default, Serialize)]
pub(in crate::root_store) struct Sizes {
    pub source_bytes: Option<usize>,
    pub source_count: Option<usize>,
    pub prompt_bytes: Option<usize>,
    pub effective_file_limit: Option<usize>,
    limits: Option<Limits>,
}

#[derive(Serialize)]
struct Limits {
    source_count: Option<usize>,
    source_bytes: Option<usize>,
    prompt_bytes: Option<usize>,
    deliveries: Option<usize>,
}

impl Sizes {
    /// Preserve the effective configured caps without interpreting absent limits
    /// as measurements or changing the byte checks.
    pub(in crate::root_store) fn limits(
        &mut self,
        files: Option<usize>,
        sources: Option<usize>,
        prompt: Option<usize>,
        deliveries: Option<usize>,
    ) {
        self.limits = Some(Limits {
            source_count: files,
            source_bytes: sources,
            prompt_bytes: prompt,
            deliveries,
        });
    }
}

/// Scope taken exclusively from the authenticated producer evidence already
/// verified by the review snapshot. No caller supplied filename or event text.
struct Scope {
    agent: String,
    work: String,
    revision: i64,
    producer_attempt: String,
    cap: Option<i64>,
}

/// Collect preflight observations without any SQL writes while the review
/// transaction is live. Logging occurs after commit or rollback.
#[derive(Default)]
pub(super) struct Preflight {
    scope: Option<Scope>,
    policy_unavailable: bool,
    pub sizes: Sizes,
    ended: Option<(Instant, u128, Option<&'static str>)>,
}

impl Preflight {
    /// Bind to a verified producer before context assembly. If the log policy
    /// cannot be read, omit the event and let the business path report its cause.
    pub(super) fn bind(
        &mut self,
        preview: &crate::TaskReviewPreview,
        settings: &crate::EngineSettingsDocument,
    ) {
        match settings.values.agent_execution_policy() {
            Ok(policy) => {
                self.scope = Some(Scope {
                    agent: preview.producer_agent_id.clone(),
                    work: preview.work_id.clone(),
                    revision: preview.work_revision,
                    producer_attempt: preview.attempt_id.clone(),
                    cap: policy.log_message_max_bytes.0.map(|v| v.0),
                })
            }
            Err(_) => self.policy_unavailable = true,
        }
    }

    /// Record only the elapsed time and stable code, leaving the original result
    /// and error tree with the business caller.
    pub(super) fn finish<T>(&mut self, start: Instant, result: &Result<T, E>) {
        let ended = Instant::now();
        self.ended = Some((
            ended,
            ended.duration_since(start).as_millis(),
            result.as_ref().err().map(|error| match error {
                E::Context(error) => error.reason(),
                _ => error.code(),
            }),
        ));
    }

    /// Return the observed end of preflight for reservation timing.
    pub(super) fn ended_at(&self) -> Option<Instant> {
        self.ended.map(|(ended, _, _)| ended)
    }

    /// Join the safe events on the digest of the decoded request intent, scoped
    /// separately by the verified producer agent. Replay creates no new event.
    pub(super) fn attempt<'a>(
        &'a self,
        correlation: &'a str,
        failures: &'a Failures,
    ) -> Option<Attempt<'a>> {
        Some(Attempt {
            scope: self.scope.as_ref()?,
            correlation,
            failures,
        })
    }

    /// Emit one completed preflight observation. Return its outcome so a
    /// preflight refusal is never also reported as a failed reservation.
    pub(super) fn emit(&self, root: &RootStore, correlation: &str, failures: &Failures) -> bool {
        if self.policy_unavailable {
            failures
                .borrow_mut()
                .push("review-diagnostic-policy-unavailable");
        }
        let (Some(attempt), Some((_, duration, cause))) =
            (self.attempt(correlation, failures), self.ended)
        else {
            return false;
        };
        attempt.emit_elapsed(root, "preflight", duration, cause, &self.sizes);
        cause.is_none()
    }
}

/// Restricted writer for one exact intent in an already bound mission scope.
pub(super) struct Attempt<'a> {
    scope: &'a Scope,
    correlation: &'a str,
    failures: &'a Failures,
}

#[derive(Serialize)]
struct Event<'a, T> {
    review_attempt: &'a str,
    phase: &'static str,
    cause: Option<&'static str>,
    component: &'static str,
    family: &'static str,
    operation: &'static str,
    level: &'static str,
    outcome: &'static str,
    work_revision: i64,
    producer_attempt: &'a str,
    elapsed_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    resume: Option<&'static str>,
    facts: T,
}

/// Preserve a JSON writer cause internally while exposing a stable sink code.
fn encoding(error: serde_json::Error) -> E {
    E::Launch(Box::new(crate::AgentLaunchError::caused(
        "review-diagnostic-encode",
        error,
    )))
}

/// Project known preparation/recovery conditions. Unknown causes deliberately
/// supply no recovery instruction and never authorize a retry.
fn recovery(code: &str) -> Option<&'static str> {
    match code {
        "review-context-bytes-exceeded"
        | "review-source-bytes-exceeded"
        | "review-context-files-exceeded"
        | "review-delivery-files-exceeded"
        | "source-exceeds-configured-cap" => Some("resolve-effective-review-limit"),
        "review-call-budget-exhausted" => Some("review-call-budget-availability"),
        "review-job-active" | "review-running-state-requires-reconciliation" => {
            Some("reconcile-current-review")
        }
        _ => None,
    }
}

impl Attempt<'_> {
    /// Append a phase using a monotonic start and already projected facts. This
    /// observation cannot replace the authoritative review result.
    pub(super) fn emit<T: Serialize>(
        &self,
        root: &RootStore,
        phase: &'static str,
        start: Instant,
        cause: Option<&'static str>,
        facts: T,
    ) {
        self.emit_elapsed(root, phase, start.elapsed().as_millis(), cause, facts);
    }

    /// Use the existing exact agent-log emitter and transaction. Check current
    /// agent ownership and suppress phase duplicates under the same write lock.
    /// Never truncate structured data or modify review/customer state on error.
    fn emit_elapsed<T: Serialize>(
        &self,
        root: &RootStore,
        phase: &'static str,
        elapsed_ms: u128,
        cause: Option<&'static str>,
        facts: T,
    ) {
        let (level, outcome) = match (phase, cause) {
            (_, None) => ("info", "observed"),
            ("preflight" | "reservation" | "reply", Some(_)) => ("warn", "refused"),
            (_, Some(_)) => ("error", "failed"),
        };
        let event = Event {
            review_attempt: self.correlation,
            phase,
            cause,
            component: "swarm-engine",
            family: "review",
            operation: "task-review",
            level,
            outcome,
            work_revision: self.scope.revision,
            producer_attempt: &self.scope.producer_attempt,
            elapsed_ms,
            resume: cause.and_then(recovery),
            facts,
        };
        let result = (|| {
            let message = serde_json::to_string(&event).map_err(encoding)?;
            if self
                .scope
                .cap
                .is_some_and(|cap| i64::try_from(message.len()).map_or(true, |length| length > cap))
            {
                return Err(E::Refused("review-diagnostic-log-cap"));
            }
            let cause_json = serde_json::to_string(&cause).map_err(encoding)?;
            let prefix = format!(
                "{{\"review_attempt\":\"{}\",\"phase\":\"{}\",\"cause\":{},",
                self.correlation, phase, cause_json,
            );
            let at = crate::system_timestamp().map_err(E::Runtime)?;
            crate::root_store::material_path::with_material_operation(root, || {
                transaction::apply(root, |tx| {
                    let agent = crate::read_agent(tx, &self.scope.agent).map_err(E::Agent)?;
                    crate::agent_store::verify_agent_ownership(tx, &agent).map_err(E::Agent)?;
                    if agent.work_id != self.scope.work {
                        return Err(E::Refused("review-diagnostic-scope-changed"));
                    }
                    let seen: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM main.agent_logs WHERE agent_id=?1 AND kind IN ('review-info','review-warn','review-error') AND substr(message,1,length(?2))=?2)",
                    rusqlite::params![self.scope.agent,prefix], |row| row.get(0),
                ).map_err(E::Sql)?;
                    if seen {
                        return Ok(Change::Observe(()));
                    }
                    let kind = match level {
                        "info" => "review-info",
                        "warn" => "review-warn",
                        _ => "review-error",
                    };
                    crate::agent_execution_store::insert_log(
                        tx,
                        &self.scope.agent,
                        &at,
                        kind,
                        &message,
                    )
                    .map_err(|error| E::Launch(Box::new(error)))?;
                    Ok(Change::Mutate(()))
                })
            })
        })();
        if let Err(error) = result {
            self.failures.borrow_mut().push(error.code());
        }
    }
}

/// Reservation fields from the committed Work and job, with no provider or
/// account identifiers. The review id is engine-generated.
pub(super) fn reservation(job: &Job, work: &crate::Work) -> impl Serialize {
    let reviewer = work
        .planning
        .as_deref()
        .and_then(|planning| planning.reviewer.as_deref());
    serde_json::json!({
        "review_id": job.id, "recorded_revision": work.revision,
        "calls": reviewer.map(|value| value.calls), "max_calls": reviewer.map(|value| value.max_calls),
    })
}

/// The helper's wire code is untrusted text at the parent boundary. Only match
/// exact stable codes with a known origin in the review gate.
pub(super) fn gate_cause(code: &str) -> Option<&'static str> {
    Some(match code {
        "review-gate-child-changed" => "review-gate-child-changed",
        "review-gate-peer-user-changed" => "review-gate-peer-user-changed",
        "review-gate-identity-or-permission" => "review-gate-identity-or-permission",
        "review-gate-parent-changed" => "review-gate-parent-changed",
        "review-gate-release-invalid" => "review-gate-release-invalid",
        "biba-landlock-unavailable" => "biba-landlock-unavailable",
        "biba-ruleset" => "biba-ruleset",
        "biba-workspace-instance-changed" => "biba-workspace-instance-changed",
        "biba-workspace-overlaps-private-root" => "biba-workspace-overlaps-private-root",
        _ => return None,
    })
}
