//! Pure wire guards. Explicit fixture values are neither a business projection
//! nor a claim that runtime measurements, validation or permissions were observed.
use super::*;
use serde_json::{json, Value};

/// Fully supplied inert fixture shared with pure insight tests. It performs no
/// Root, environment, clock or provider IO and grants nothing to its consumer.
pub(crate) fn status() -> MissionStatus {
    let observation = || VersionObservation {
        available: false,
        version: None,
        commit: None,
        modified: None,
        provenance: "fixture-unobserved".into(),
        hint: String::new(),
    };
    MissionStatus {
        changes: MissionChanges {
            from: 0,
            to: 7,
            first: true,
            more: false,
            items: Some(vec![]),
        },
        spending: MissionSpending {
            calls: Some(vec![]),
            observed_at: "fixture-time".into(),
            source: "fixture".into(),
            rows: Some(vec![]),
            controls: 0,
            retries: 0,
            note: "fixture".into(),
            attempts: Some(vec![]),
        },
        guidance: MissionGuidance {
            what: "fixture".into(),
            next: "fixture".into(),
            actor: "fixture".into(),
            primary: primary(),
        },
        runtime: RuntimeHealth {
            state: "fixture-unobserved".into(),
            launch_allowed: false,
            observed_at: "fixture-time".into(),
            message: "fixture".into(),
            next: "fixture".into(),
            volumes: Some(vec![]),
            version: ServerVersion {
                binary: BinaryVersion {
                    version: "fixture".into(),
                    commit: None,
                    modified: None,
                    build_date: None,
                    provenance: "fixture".into(),
                },
                sources: SourceVersion {
                    available: false,
                    commit: None,
                    modified: None,
                    compared: false,
                    matches_binary: None,
                },
                history: VersionHistory {
                    schema: 1,
                    available: false,
                    releases: Some(vec![]),
                    local_changes: None,
                },
            },
            diagnostic: VersionDiagnostic {
                installed_cli: observation(),
                active_server: observation(),
                candidate: observation(),
                state: "unknown".into(),
                differences: Some(vec![]),
                next_step: String::new(),
            },
        },
        provider_cooldowns: None,
        evidence_stage: "fixture-unverified".into(),
        organization: Organization {
            ready: false,
            label: "fixture".into(),
            issues: Some(vec![]),
            next: "fixture".into(),
            verification: "fixture".into(),
        },
        authorized: false,
        enabled: false,
        paused: false,
        summary: "fixture".into(),
        next: "fixture".into(),
        validated: 0,
        review: 0,
        running: 0,
        active_agents: 0,
        uncertain_agents: 0,
        total: 0,
        supervision: MissionSupervision {
            state: "absent".into(),
            source: String::new(),
            observed_at: String::new(),
            last_check_at: String::new(),
            last_check_relative: String::new(),
            reconciled_at: String::new(),
            last_error: String::new(),
            next_check_at: String::new(),
            next_check_relative: String::new(),
            late: false,
            clock_issue: String::new(),
            last_action: None,
        },
        understanding: understanding(),
        coordination: Some(vec![]),
        tasks: Some(vec![]),
    }
}
/// Explicit task fixture for wire/insight guards; no default fills missing proof.
pub(crate) fn task() -> MissionTask {
    MissionTask {
        waits: None,
        recovery_preview: RecoveryPreview {
            dialogue: None,
            work: "fixture-work".into(),
            task: "fixture-task".into(),
            revision: 7,
            agent: String::new(),
            kept: None,
            redone: Some(vec![]),
            correction: String::new(),
            criteria: Some(vec![]),
            limits: "fixture".into(),
            since_refusal: None,
            evidence: None,
            remaining: Some(vec![]),
            refusal_note: String::new(),
            evidence_note: String::new(),
        },
        primary: primary(),
        attempts_used: 0,
        attempts_allowed: 3,
        attempt_limit_reached: false,
        id: "fixture-task".into(),
        title: "fixture".into(),
        state: "todo".into(),
        reason: String::new(),
        action: "inspect".into(),
        label: "fixture".into(),
        target: String::new(),
        impact: 0,
        deliverable: "fixture".into(),
        validation_mode: "human".into(),
        validation_receipt: String::new(),
        result: ResultPresentation {
            process_label: String::new(),
            report_label: String::new(),
            validation_label: String::new(),
            state: "fixture-unverified".into(),
            label: String::new(),
            process_state: String::new(),
            report_state: String::new(),
            validation_state: String::new(),
            reason: String::new(),
            next_step: String::new(),
            attempt_id: String::new(),
            report_id: String::new(),
            gate_id: String::new(),
            receipt_id: String::new(),
        },
        understanding: understanding(),
        diagnostic: None,
        attempt: MissionAttemptProjection {
            agent_id: String::new(),
            attempt_id: String::new(),
            role: String::new(),
            process_state: String::new(),
            activity: String::new(),
            validation: "unknown".into(),
            requested_model: String::new(),
            observed_model: String::new(),
            cost_state: "unknown".into(),
        },
    }
}
/// Pure presentation fixture; it applies no suggested action.
fn primary() -> MissionPrimaryAction {
    MissionPrimaryAction {
        kind: "fixture".into(),
        label: "fixture".into(),
        effect: "fixture".into(),
        task: String::new(),
        tone: "fixture".into(),
    }
}
/// Pure explanation fixture; it authenticates no actor.
fn understanding() -> MissionUnderstanding {
    MissionUnderstanding {
        what: "fixture".into(),
        next_step: "fixture".into(),
        actor: "fixture".into(),
        actor_kind: "none".into(),
        situation: "fixture-unverified".into(),
    }
}
/// Sort only object field names for exact schema assertions; values and array
/// ordering are left untouched, including null and signed/unsigned number kinds.
fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<_> = value
        .as_object()
        .expect("object fixture")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

/// Go MissionStatus has22 declared fields, but empty cooldowns omit their key.
/// No native wrapper fields appear and required empty arrays remain arrays.
#[test]
fn mission_status_contract_has_exact_legacy_keys_and_no_native_wrapper() {
    let mut status = status();
    let control = serde_json::to_value(&status).unwrap();
    let mut expected = vec![
        "changes_since_visit",
        "spending",
        "guidance",
        "runtime",
        "evidence_stage",
        "organization",
        "authorized",
        "enabled",
        "paused",
        "summary",
        "next",
        "validated",
        "review",
        "running",
        "active_agents",
        "uncertain_agents",
        "total",
        "supervision",
        "understanding",
        "coordination",
        "tasks",
    ];
    expected.sort_unstable();
    assert_eq!(keys(&control), expected);
    assert_eq!(control["tasks"], json!([]));
    assert_eq!(
        control["supervision"],
        json!({"state":"absent","late":false})
    );
    status.provider_cooldowns = Some(vec![]);
    assert_eq!(serde_json::to_value(&status).unwrap(), control);
    status.provider_cooldowns = Some(vec![ProviderCooldown {
        provider: "fixture".into(),
        observed_at: "fixture-time".into(),
        resets_at: 0,
        source: "fixture".into(),
        signal: "fixture".into(),
        cleared_at: String::new(),
        clear_event: String::new(),
        clear_reason: String::new(),
        clear_actor: String::new(),
        clear_account_change: false,
    }]);
    let present = serde_json::to_value(&status).unwrap();
    assert_eq!(present.as_object().unwrap().len(), 22);
    assert_eq!(
        present["provider_cooldowns"],
        json!([{"provider":"fixture","observed_at":"fixture-time","source":"fixture","signal":"fixture"}])
    );
}

/// Required nil slices remain null while Some(empty) remains []; Go omitempty
/// slices remove both. Nested task names and optional strings match Go tags.
#[test]
fn mission_status_contract_preserves_nil_empty_and_nested_omitempty() {
    let mut value = task();
    let control = serde_json::to_value(&value).unwrap();
    assert!(control["waiting_on"].is_null());
    assert!(control["recovery_preview"]["kept"].is_null());
    assert_eq!(control["recovery_preview"]["redone"], json!([]));
    assert!(control.get("primary_action").is_some());
    for field in ["target", "validation_receipt", "diagnostic"] {
        assert!(control.get(field).is_none());
    }
    assert_eq!(
        control["attempt"],
        json!({"validation_state":"unknown","cost_state":"unknown"})
    );
    assert!(control["recovery_preview"].get("evidence").is_none());
    value.waits = Some(vec![]);
    value.recovery_preview.evidence = Some(vec![]);
    value.target = "fixture-target".into();
    value.validation_receipt = "fixture-receipt".into();
    let changed = serde_json::to_value(&value).unwrap();
    assert_eq!(changed["waiting_on"], json!([]));
    assert!(changed["recovery_preview"].get("evidence").is_none());
    assert_eq!(changed["target"], "fixture-target");
    assert_eq!(changed["validation_receipt"], "fixture-receipt");
    value.recovery_preview.evidence = Some(vec![RecoveryEvidence {
        report: "fixture-report".into(),
        state: "unknown".into(),
        reason: "fixture".into(),
    }]);
    assert_eq!(
        serde_json::to_value(&value).unwrap()["recovery_preview"]["evidence"],
        json!([{"report":"fixture-report","state":"unknown","reason":"fixture"}])
    );
}

/// Unknown pointers stay null, while observed false/zero are preserved. Full
/// unsigned64 filesystem counts serialize as numbers without signed narrowing.
#[test]
fn mission_status_contract_keeps_nullable_values_and_numeric_widths() {
    let mut source = SourceVersion {
        available: false,
        commit: None,
        modified: None,
        compared: false,
        matches_binary: None,
    };
    assert_eq!(
        serde_json::to_value(&source).unwrap(),
        json!({"available":false,"commit":null,"modified":null,"compared":false,"matches_binary":null})
    );
    source.commit = Some(Box::new(String::new()));
    source.modified = Some(Box::new(false));
    source.matches_binary = Some(Box::new(false));
    assert_eq!(serde_json::to_value(&source).unwrap()["modified"], false);
    assert_eq!(serde_json::to_value(&source).unwrap()["commit"], "");
    let volume = StorageVolume {
        kind: "fixture".into(),
        path: "fixture".into(),
        available: u64::MAX,
        inodes: u64::MAX,
        state: "fixture-unobserved".into(),
    };
    let volume = serde_json::to_value(&volume).unwrap();
    assert_eq!(volume["available_bytes"].as_u64(), Some(u64::MAX));
    assert_eq!(volume["available_inodes"].as_u64(), Some(u64::MAX));
    let mut row = SpendingRow {
        kind: "fixture".into(),
        id: "fixture".into(),
        label: "fixture".into(),
        calls: i64::MAX,
        tools: 0,
        unknown_tools: 1,
        input: i64::MAX,
        output: 0,
        missing_usage: 1,
        cost: CostTotal {
            reported_usd: crate::GoFloat(0.0),
            attempts_with_cost: 0,
            attempts_without_cost: 1,
        },
        reviews: 0,
        task_retries: 0,
        duration_ms: None,
        missing_durations: 1,
    };
    let missing = serde_json::to_value(&row).unwrap();
    assert!(missing["recorded_process_ms"].is_null());
    assert_eq!(missing["recorded_calls"].as_i64(), Some(i64::MAX));
    assert_eq!(missing["input_tokens"].as_i64(), Some(i64::MAX));
    row.duration_ms = Some(Box::new(0));
    assert_eq!(
        serde_json::to_value(&row).unwrap()["recorded_process_ms"],
        0
    );
}

/// Reused CostTotal and Usage retain observed zero versus missing measurement.
/// A finite witness precedes each nonfinite cost; serializer errors propagate
/// instead of producing successful JSON null or a fabricated measurement.
#[test]
fn mission_status_contract_reuses_cost_usage_and_propagates_float_errors() {
    let mut call = SpendingCall {
        id: "fixture".into(),
        record_type: "fixture".into(),
        kind: "worker".into(),
        role: "worker".into(),
        scope: String::new(),
        task: String::new(),
        attempt: String::new(),
        provider: String::new(),
        requested_model: String::new(),
        observed_model: String::new(),
        state: "queued".into(),
        registered_at: String::new(),
        usage: None,
    };
    assert!(serde_json::to_value(&call).unwrap()["usage"].is_null());
    call.usage = Some(Box::new(Usage {
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        cached_input_tokens: None,
        provider_reported_cost_usd: Some(Box::new(crate::GoFloat(0.0))),
        input_tokens: 0,
        output_tokens: 0,
        source: "fixture".into(),
        scope: "fixture".into(),
        at: "fixture-time".into(),
    }));
    assert_eq!(
        serde_json::to_value(&call).unwrap()["usage"],
        json!({"provider_reported_cost_usd":0,"input_tokens":0,"output_tokens":0,"source":"fixture","scope":"fixture","at":"fixture-time"})
    );
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let cost = CostTotal {
            reported_usd: crate::GoFloat(0.25),
            attempts_with_cost: 1,
            attempts_without_cost: 0,
        };
        assert_eq!(
            serde_json::to_value(&cost).unwrap(),
            json!({"reported_usd":0.25,"attempts_with_cost":1,"attempts_without_cost":0})
        );
        let invalid = CostTotal {
            reported_usd: crate::GoFloat(invalid),
            attempts_with_cost: 1,
            attempts_without_cost: 0,
        };
        assert!(serde_json::to_vec(&invalid).is_err());
    }
}
