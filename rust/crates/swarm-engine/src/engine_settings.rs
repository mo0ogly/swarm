//! Persisted-setting contract. Decimal strings avoid JS rounding. A preview is
//! neither authority nor publication; logical history is not rollback protection.
use crate::{EngineMutationLimits, TaskDefinitionLimits, ValidationPolicyLimits};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, fmt};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingCount(pub i64);
/// Providers with a supported telemetry injection (protocol capability, not an
/// operator choice): Claude Code reads OTEL exporter variables from its env;
/// Codex takes global `-c otel.…` overrides plus the header from its env.
pub const TELEMETRY_PROVIDERS: [&str; 2] = ["claude", "codex"];
/// An explicitly configured cap. Outer absence denotes a journal version before
/// this field was introduced; inner null explicitly disables the relevant cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SettingCap(pub Option<SettingCount>);
/// Saved telemetry store bounds (v33): (retention days, records per attempt),
/// each `None` when explicitly unlimited.
pub type TelemetryBounds = (Option<i64>, Option<i64>);
/// Explicitly saved storage-only diagnosis thresholds. These neither reserve
/// capacity nor grant launch rights; callers must preserve probe failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeStoragePolicy {
    pub stop_bytes: u64,
    pub warning_bytes: u64,
    pub stop_inodes: u64,
}
impl Serialize for SettingCount {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for SettingCount {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        let n = raw
            .parse::<i64>()
            .map_err(|_| serde::de::Error::custom("nonnegative decimal string required"))?;
        if n < 0 || n.to_string() != raw {
            return Err(serde::de::Error::custom(
                "canonical nonnegative decimal string required",
            ));
        }
        Ok(Self(n))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSettingsValues {
    /// Report mtime tolerance in seconds (v40). Absence stays unconfigured on
    /// historical installations; zero requires mtime >= the attempt start.
    /// This affects relay selection, never author authentication or acceptance.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_count")]
    pub agent_report_clock_skew_seconds: Option<SettingCount>,
    /// Recent Agent rows used by status presentation only (v39). Outer absence
    /// is unconfigured; explicit null includes all rows, zero hides all rows.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub work_view_status_recent_agents: Option<SettingCap>,
    /// Raw events inspected for changes since visit (v39), before category
    /// filtering. Never limits retention or accounting; null means all events.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub work_view_status_change_events: Option<SettingCap>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_count")]
    pub runtime_storage_stop_bytes: Option<SettingCount>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_count")]
    pub runtime_storage_warning_bytes: Option<SettingCount>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_count")]
    pub runtime_storage_stop_inodes: Option<SettingCount>,
    #[serde(deserialize_with = "required_cap")]
    pub task_max_attempts: Option<SettingCount>,
    #[serde(deserialize_with = "required_cap")]
    pub task_max_tool_calls: Option<SettingCount>,
    #[serde(deserialize_with = "required_cap")]
    pub task_max_criteria: Option<SettingCount>,
    #[serde(deserialize_with = "required_cap")]
    pub validation_max_controls: Option<SettingCount>,
    #[serde(deserialize_with = "required_cap")]
    pub validation_max_arguments: Option<SettingCount>,
    #[serde(deserialize_with = "required_cap")]
    pub validation_max_inputs: Option<SettingCount>,
    pub validation_min_justification_chars: SettingCount,
    #[serde(deserialize_with = "required_cap")]
    pub validation_max_justification_chars: Option<SettingCount>,
    pub validation_default_timeout_seconds: SettingCount,
    #[serde(deserialize_with = "required_cap")]
    pub validation_max_timeout_seconds: Option<SettingCount>,
    #[serde(deserialize_with = "required_cap")]
    pub validation_max_total_timeout_seconds: Option<SettingCount>,
    pub validation_allowed_programs: BTreeSet<String>,
    pub storage_busy_timeout_ms: SettingCount,
    #[serde(deserialize_with = "required_cap")]
    pub storage_busy_retries: Option<SettingCount>,
    pub storage_busy_retry_delay_ms: SettingCount,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub validation_max_output_bytes: Option<SettingCap>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub quality_delivery_min_percent: Option<SettingCount>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub quality_domain_min_percent: Option<SettingCount>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub quality_band_ok_min_percent: Option<SettingCount>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub quality_band_warn_min_percent: Option<SettingCount>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub quality_band_blocked_min_percent: Option<SettingCount>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub project_profile_max_name_bytes: Option<SettingCap>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub project_profile_max_sources: Option<SettingCap>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub project_profile_max_path_bytes: Option<SettingCap>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub project_profile_max_document_bytes: Option<SettingCap>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub project_context_max_bytes: Option<SettingCap>,
    /// v42: saved draft editing policy; absence requires explicit adoption.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_count")]
    pub preparation_write_enabled: Option<SettingCount>,
    /// v42: saved draft editing policy; absence requires explicit adoption.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub preparation_document_max_bytes: Option<SettingCap>,
    /// v42: saved draft editing policy; absence requires explicit adoption.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub preparation_title_max_bytes: Option<SettingCap>,
    /// v42: saved draft editing policy; absence requires explicit adoption.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_string")]
    pub preparation_default_method: Option<String>,
    /// v41: explicit preparation read policy; absence requires adoption.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub preparation_list_limit: Option<SettingCap>,
    /// v41: explicit preparation read policy; absence requires adoption.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub preparation_history_limit: Option<SettingCap>,
    /// v41: explicit preparation read policy; absence requires adoption.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub preparation_method_max_bytes: Option<SettingCap>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub project_worker_context_max_bytes: Option<SettingCap>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub review_delivery_max_bytes: Option<SettingCap>,
    /// v5: absent in older journals; explicit null disables the Git deadline.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub git_command_timeout_seconds: Option<SettingCap>,
    /// v6: root depth is zero; null explicitly disables the baseline depth cap.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub managed_review_max_baseline_depth: Option<SettingCap>,
    /// v7: minimum trimmed UTF-8 bytes for both reply rationale and criterion evidence.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub review_reply_min_text_bytes: Option<SettingCount>,
    /// v7: maximum raw UTF-8 bytes per text field; explicit null disables this cap.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub review_reply_max_text_bytes: Option<SettingCap>,
    /// v7: minimum normalized Markdown quote bytes; literal matching is unaffected.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub review_quote_min_normalized_bytes: Option<SettingCount>,
    /// v8: longest measured controls shown; null shows all, zero shows none.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub evidence_longest_measured_count: Option<SettingCap>,
    /// v8: recent OODA events shown; null shows all, zero shows none.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub work_view_recent_decisions: Option<SettingCap>,
    /// v8: Unicode scalar wrapping after tab expansion; null disables wrapping.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub work_view_wrap_characters: Option<SettingCap>,
    /// v9: explicit future manual phase defaults; empty is valid, null is not.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_gate_phases"
    )]
    pub task_manual_required_gate_phases: Option<crate::ManualGatePhaseDefaults>,
    /// v10: machine receipt bytes; absent legacy policy is not implicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub managed_receipt_max_bytes: Option<SettingCap>,
    /// v11: nonblank operator resume reason bytes; absence is not adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_resume_max_reason_bytes: Option<SettingCap>,
    /// v12: canonical serialized planning intent bytes; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_request_max_bytes: Option<SettingCap>,
    /// v12: requirements at explicit planning enable; zero forbids enable.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_max_requirements: Option<SettingCap>,
    /// v13: compiled base workflow body bytes; not project/metadata/model capacity.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_workflow_max_body_bytes: Option<SettingCap>,
    /// v14: complete planning report bytes, before context/model composition.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_report_max_bytes: Option<SettingCap>,
    /// v15: saved runtime default; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub runtime_silence_seconds_default: Option<SettingCount>,
    /// v15: saved runtime ceiling; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub runtime_silence_seconds_ceiling: Option<SettingCap>,
    /// v15: saved runtime default; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub runtime_tool_seconds_default: Option<SettingCount>,
    /// v15: saved runtime ceiling; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub runtime_tool_seconds_ceiling: Option<SettingCap>,
    /// v15: saved runtime default; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub runtime_max_tool_calls_default: Option<SettingCount>,
    /// v15: saved runtime ceiling; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub runtime_max_tool_calls_ceiling: Option<SettingCap>,
    /// v15: saved runtime default; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub runtime_max_repeated_calls_default: Option<SettingCount>,
    /// v15: saved runtime ceiling; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub runtime_max_repeated_calls_ceiling: Option<SettingCap>,
    /// v15: saved runtime default; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub runtime_max_consecutive_errors_default: Option<SettingCount>,
    /// v15: saved runtime ceiling; absent historical values require explicit adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub runtime_max_consecutive_errors_ceiling: Option<SettingCap>,
    /// v16: explicit saved launch preparation policy; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub agent_launch_timeout_seconds_default: Option<SettingCount>,
    /// v16: explicit saved launch preparation policy; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_launch_timeout_seconds_ceiling: Option<SettingCap>,
    /// v16: explicit saved launch preparation policy; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_launch_instruction_max_bytes: Option<SettingCap>,
    /// v16: explicit saved launch preparation policy; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_provider_config_max_bytes: Option<SettingCap>,
    /// v16: explicit saved launch preparation policy; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_launch_context_max_bytes: Option<SettingCap>,
    /// v16: explicit saved launch preparation policy; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub agent_preflight_ttl_seconds_default: Option<SettingCount>,
    /// v16: explicit saved launch preparation policy; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_preflight_ttl_seconds_ceiling: Option<SettingCap>,
    /// v16: explicit saved launch preparation policy; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub agent_preflight_min_free_bytes: Option<SettingCount>,
    /// v16: explicit saved launch preparation policy; absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_string_array"
    )]
    pub agent_environment_allow: Option<Vec<String>>,
    /// v16: saved compatibility choice, 0 requires verified provider context.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub agent_allow_unverified_provider_context: Option<SettingCount>,
    /// v17: explicit history-window multiplier; null retains all observed failures.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_interleaved_failure_lookback_multiplier: Option<SettingCap>,
    /// v18: poll interval ms; absent in historical documents.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub agent_supervision_poll_interval_ms: Option<SettingCount>,
    /// v18: heartbeat stale ms; absent in historical documents.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub agent_supervision_heartbeat_stale_ms: Option<SettingCount>,
    /// v18: term grace ms; absent in historical documents.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub agent_supervision_term_grace_ms: Option<SettingCount>,
    /// v18: output drain ms; absent in historical documents.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_supervision_output_drain_ms: Option<SettingCap>,
    /// v18: event max bytes; absent in historical documents.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_provider_event_max_bytes: Option<SettingCap>,
    /// v18: capture max bytes; absent in historical documents.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_capture_max_bytes: Option<SettingCap>,
    /// v18: capture max lines; absent in historical documents.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_capture_max_lines: Option<SettingCap>,
    /// v18: public message max bytes; absent in historical documents.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_public_message_max_bytes: Option<SettingCap>,
    /// v18: log message max bytes; absent in historical documents.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub agent_log_message_max_bytes: Option<SettingCap>,
    /// v19: saved eligible levels for automatic choice; legacy absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_string_array"
    )]
    pub agent_model_automatic_levels: Option<Vec<String>>,
    /// v19: saved eligible levels for flagged models; legacy absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_string_array"
    )]
    pub agent_model_restricted_levels: Option<Vec<String>>,
    /// v19: saved explicit exact/prefix/contains model predicates; legacy absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_string_array"
    )]
    pub agent_model_restriction_rules: Option<Vec<String>>,
    /// v20: configured native HTTP bind_address; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_string"
    )]
    pub web_bind_address: Option<String>,
    /// v20: configured native HTTP http_header_max_bytes; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub web_http_header_max_bytes: Option<SettingCount>,
    /// v20: configured native HTTP http_max_headers; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub web_http_max_headers: Option<SettingCount>,
    /// v20: configured native HTTP http_header_timeout_ms; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub web_http_header_timeout_ms: Option<SettingCap>,
    /// v20: configured native HTTP http_body_max_bytes; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub web_http_body_max_bytes: Option<SettingCap>,
    /// v20: configured native HTTP http_body_timeout_ms; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub web_http_body_timeout_ms: Option<SettingCap>,
    /// v20: configured native HTTP http_max_connections; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub web_http_max_connections: Option<SettingCap>,
    /// v20: configured native HTTP root_worker_threads; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub web_root_worker_threads: Option<SettingCount>,
    /// v20: configured native HTTP admin_worker_threads; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub web_admin_worker_threads: Option<SettingCount>,
    /// v20: configured native HTTP worker_idle_keep_alive_ms; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub web_worker_idle_keep_alive_ms: Option<SettingCount>,
    /// v20: configured native HTTP cookie_max_age_seconds; absent in historical documents.
    /// Network fields apply on server restart, not as a running-state observation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub web_cookie_max_age_seconds: Option<SettingCap>,
    /// v21: explicit positive scheduling cadence, in milliseconds. Historical absence
    /// remains absent; this value grants no identity, launch or acceptance right.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub mission_supervision_poll_interval_ms: Option<SettingCount>,
    /// v21: explicit positive maximum lease age, in milliseconds. Historical absence
    /// remains absent; this value grants no identity, launch or acceptance right.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub mission_supervision_stale_after_ms: Option<SettingCount>,
    /// v21: explicit nonnegative future clock tolerance, in milliseconds. Historical absence
    /// remains absent; this value grants no identity, launch or acceptance right.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub mission_supervision_future_tolerance_ms: Option<SettingCount>,
    /// v22: explicit positive initial slot count, not consent or an autonomy default.
    /// Historical absence remains absent; saved controls are never clamped.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub mission_dispatch_default_slots: Option<SettingCount>,
    /// v22: explicit optional operator slot cap; null removes only this cap.
    /// Historical absence remains absent; saved controls are never clamped.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub mission_dispatch_max_slots: Option<SettingCap>,
    /// v32: explicit per-task reported-cost ceiling as percent of launch reserve.
    /// Null disables only this guard; zero rejects positive reported costs when
    /// reserve is positive. Historical absence is not installation adoption.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub mission_dispatch_cost_reserve_percent: Option<SettingCap>,
    /// v23: explicit initial reviewer configuration value; zero means no cap.
    /// Existing reviewer snapshots remain unchanged; historical absence is not a default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub reviewer_default_delivery_files: Option<SettingCount>,
    /// v23: explicit initial reviewer configuration value; zero means no cap.
    /// Existing reviewer snapshots remain unchanged; historical absence is not a default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub reviewer_default_context_files: Option<SettingCount>,
    /// v23: explicit initial reviewer configuration value; zero means no cap.
    /// Existing reviewer snapshots remain unchanged; historical absence is not a default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub reviewer_default_source_bytes: Option<SettingCount>,
    /// v23: explicit initial reviewer configuration value; zero means no cap.
    /// Existing reviewer snapshots remain unchanged; historical absence is not a default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub reviewer_default_context_bytes: Option<SettingCount>,
    /// v27: explicit structured-provider wait/lease-maintenance policy.
    /// Historical absence is preserved; null is invalid, never a default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub planning_provider_silence_seconds: Option<SettingCount>,
    /// v27: explicit structured-provider wait/lease-maintenance policy.
    /// Historical absence is preserved; null is invalid, never a default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub planning_provider_max_duration_seconds: Option<SettingCount>,
    /// v27: explicit structured-provider wait/lease-maintenance policy.
    /// Historical absence is preserved; null is invalid, never a default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub planning_default_lease_seconds: Option<SettingCount>,
    /// v27: explicit structured-provider wait/lease-maintenance policy.
    /// Historical absence is preserved; null is invalid, never a default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub planning_lease_renew_after_percent: Option<SettingCount>,
    /// v26: complete structured planner reply ceiling; null removes the cap,
    /// zero forbids nonempty replies. Absence is historical, never a fallback.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_response_max_bytes: Option<SettingCap>,
    /// v25: explicit decision boundary; null removes the operator cap.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_decision_max_inputs: Option<SettingCap>,
    /// v25: explicit decision boundary; null removes the operator cap.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_decision_max_operations: Option<SettingCap>,
    /// v25: explicit decision boundary; null removes the operator cap.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_decision_max_reason_bytes: Option<SettingCap>,
    /// v25: explicit decision boundary; null removes the operator cap.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_decision_max_title_characters: Option<SettingCap>,
    /// v25: explicit decision boundary; null removes the operator cap.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_decision_max_instruction_bytes: Option<SettingCap>,
    /// v25: explicit decision boundary; null removes the operator cap.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_decision_max_dependencies: Option<SettingCap>,
    /// v25: explicit decision boundary; null removes the operator cap.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_decision_max_criterion_bytes: Option<SettingCap>,
    /// v25: positive worker creation default, never an implicit retry grant.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub planning_worker_default_attempts: Option<SettingCount>,
    /// v24: explicit scope activation policy; historical absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_count"
    )]
    pub planning_scope_lease_min_seconds: Option<SettingCount>,

    /// v24: explicit scope activation policy; historical absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_scope_lease_max_seconds: Option<SettingCap>,

    /// v24: explicit scope activation policy; historical absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_context_max_events: Option<SettingCap>,

    /// v24: explicit scope activation policy; historical absence requires adoption.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_cap"
    )]
    pub planning_context_max_bytes: Option<SettingCap>,
    /// v28: quota audit minimum UTF8 bytes after trimming; zero removes the extra
    /// minimum while nonblank remains structural. Historical absence refuses.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_count")]
    pub planning_quota_reason_min_bytes: Option<SettingCount>,
    /// v28: quota audit maximum original UTF8 bytes; null explicitly unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub planning_quota_reason_max_bytes: Option<SettingCap>,
    /// v29: complete reviewer reply ceiling in UTF8 bytes; explicit null removes
    /// only this cap, zero forbids nonempty replies. Historical absence refuses
    /// at the consumer, never falling back to planner policy or installer values.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub review_response_max_bytes: Option<SettingCap>,
    /// v30: providers whose launches receive a per-attempt OTLP ingest token
    /// and exporter environment (LIVE-UX-03). Explicit empty list = disabled;
    /// historical absence also means never opted in. Only `claude` is
    /// supported; other names refuse instead of being silently ignored.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_string_array"
    )]
    pub agent_telemetry_providers: Option<Vec<String>>,
    /// v31 (LIVE-UX L3, cache health): a model call is flagged as a cache miss
    /// (context re-read) when its cache-read share of total input is below this
    /// percent and its total input reaches `agent_cache_min_input_tokens`.
    /// Display policy for the live console only; it gates nothing.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_count")]
    pub agent_cache_miss_ratio_percent: Option<SettingCount>,
    /// v31: minimum total input tokens (input + cache read + cache creation)
    /// for a call to be judged at all; zero judges every call.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_count")]
    pub agent_cache_min_input_tokens: Option<SettingCount>,
    /// v33 (LIVE-UX L1, retention): days that telemetry records, resolved
    /// supervisor alerts and revoked token digests are kept in
    /// `.swarm/telemetry.db`; null keeps them without limit. Applied by the
    /// console store's writers, never by a read.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub agent_telemetry_retention_days: Option<SettingCap>,
    /// v33: telemetry records stored per agent attempt; beyond it points are
    /// rejected and counted (OTLP `partialSuccess`), never truncated silently.
    /// Null = no operator cap (the HTTP body limit still applies).
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub agent_telemetry_max_events_per_attempt: Option<SettingCap>,
    /// v34: tolerated gap, in percent of the declared output tokens, between
    /// a finished attempt's declared usage and the sum of its telemetry calls
    /// before the supervisor alert `agent-usage-mismatch` opens. Null turns
    /// the rule off. Output tokens only: input semantics differ by provider.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub agent_usage_mismatch_percent: Option<SettingCap>,
    /// v35: cadence, in seconds, of the supervisor alert evaluation run by
    /// the web server itself, so alerts fire without an open console. Null
    /// keeps evaluation on console requests only. Read at server start and
    /// at every cycle: null at a cycle stops the server loop until restart.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub agent_alert_evaluation_seconds: Option<SettingCap>,
    /// v35: share, in percent of a Work's positive budget limit, of committed
    /// estimates or declared cost at which the `work-budget-threshold` alert
    /// opens (1–100). Null turns the warning off; the overspend alert
    /// (declared cost above the limit) does not depend on it.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub work_budget_warning_percent: Option<SettingCap>,
    /// v36: longest operator annotation note accepted, in characters
    /// (LangWatch L10). Null leaves only the HTTP body limit. Zero refused.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub annotation_note_max_chars: Option<SettingCap>,
    /// v37: share, in percent (1–100), of finished attempts evaluated by the
    /// deterministic evaluators (LangWatch L4), chosen by a stable hash of
    /// the attempt id. Null turns evaluations off.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_cap")]
    pub agent_evaluation_sample_percent: Option<SettingCap>,
}
/// Preserve legacy absence separately from a validated explicit set; null and
/// unknown phase names refuse. No task mutation, IO or authorization occurs.
fn present_gate_phases<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<crate::ManualGatePhaseDefaults>, D::Error> {
    crate::ManualGatePhaseDefaults::deserialize(d).map(Some)
}
/// Preserve explicit string arrays, rejecting null rather than silently adopting
/// defaults. Missing historical fields remain absent; each consumer validates its
/// vocabulary and duplicates. This decoder grants no environment/model privilege.
fn present_string_array<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(d).map(Some)
}
/// Preserve explicit address strings; absent remains legacy, null refuses.
/// Parsing/loopback/canonical validation belongs to the authoritative policy builder.
fn present_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
fn present_count<'de, D: Deserializer<'de>>(d: D) -> Result<Option<SettingCount>, D::Error> {
    SettingCount::deserialize(d).map(Some)
}
fn present_cap<'de, D: Deserializer<'de>>(d: D) -> Result<Option<SettingCap>, D::Error> {
    Option::<SettingCount>::deserialize(d).map(|cap| Some(SettingCap(cap)))
}
fn required_cap<'de, D: Deserializer<'de>>(d: D) -> Result<Option<SettingCount>, D::Error> {
    Option::<SettingCount>::deserialize(d)
}
/// Persisted read bounds, never execution or conversion authority.
/// None is unlimited; row zero hides rows without deleting history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparationReadPolicy {
    pub list_limit: Option<usize>,
    pub history_limit: Option<usize>,
    pub method_max_bytes: Option<usize>,
}
pub enum EngineSettingsError {
    WebAddress(std::net::AddrParseError),
    WebCount(std::num::TryFromIntError),
    /// General policy-count representation, distinct from HTTP transport counts.
    Count(std::num::TryFromIntError),
    Format(serde_json::Error),
    Refused(&'static str),
}
impl EngineSettingsError {
    /// Static safe validation reason, excluding supplied addresses/counts/JSON.
    /// Classifying a failure neither mutates state nor authorizes a retry.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::WebAddress(_) => "settings-web-address-invalid",
            Self::WebCount(_) => "settings-web-count-representation",
            Self::Count(_) => "settings-count-representation",
            Self::Format(_) => "settings-format",
            Self::Refused(s) => s,
        }
    }
    pub fn code(&self) -> &'static str {
        "engine-settings"
    }
}
impl fmt::Display for EngineSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "engine-settings: {}", self.reason())
    }
}
impl fmt::Debug for EngineSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for EngineSettingsError {
    /// Retain original JSON/address/integer conversion causes. Semantic refusals
    /// have a static reason; public Display/Debug do not echo supplied values.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::WebAddress(e) => Some(e),
            Self::WebCount(e) | Self::Count(e) => Some(e),
            Self::Format(e) => Some(e),
            Self::Refused(_) => None,
        }
    }
}
fn refuse(reason: &'static str) -> EngineSettingsError {
    EngineSettingsError::Refused(reason)
}
fn nonnegative(value: SettingCount) -> Result<i64, EngineSettingsError> {
    if value.0 < 0 {
        Err(refuse("settings-negative-count"))
    } else {
        Ok(value.0)
    }
}
fn cap(value: Option<SettingCount>) -> Result<i64, EngineSettingsError> {
    value
        .map(nonnegative)
        .transpose()
        .map(|n| n.unwrap_or(i64::MAX))
}
fn size(value: Option<SettingCount>) -> Result<usize, EngineSettingsError> {
    match value {
        None => Ok(usize::MAX),
        Some(n) => usize::try_from(nonnegative(n)?)
            .map_err(|_| refuse("settings-count-platform-representation")),
    }
}
impl EngineSettingsValues {
    /// Decode the versioned installation profile embedded from the canonical
    /// configuration asset. Returns typed defaults or a safe policy/format error;
    /// it does not read installed settings, publish choices or mutate old tasks.
    pub fn installation_defaults() -> Result<Self, EngineSettingsError> {
        Self::decode(include_bytes!("../config/engine-settings.json"))
    }
    /// Decode exact setting keys/types and validate dependencies. Unknown fields,
    /// malformed values and incomplete groups return typed errors. Historical
    /// absence remains absent; no IO, default adoption or authority is implied.
    pub fn decode(raw: &[u8]) -> Result<Self, EngineSettingsError> {
        let value: Self = serde_json::from_slice(raw).map_err(EngineSettingsError::Format)?;
        value.validate()?;
        Ok(value)
    }
    /// Validate declared policies and version dependencies without adopting
    /// missing groups. Returns a safe typed format/policy error; no IO or state
    /// changes occur, and legacy absence never becomes implicit current defaults.
    /// Validate supplied values, technical representation and complete versioned
    /// policy groups through v42. Historical absence remains absent; network20
    /// requires prior groups and never adopts defaults, opens sockets, changes
    /// Work/history or establishes active-server state. Typed validation causes
    /// survive rejection; caller still owes authority and publication freshness.
    pub fn validate(&self) -> Result<(), EngineSettingsError> {
        if self.has_preparation_write_policy() { self.preparation_write_policy()?; self.preparation_read_policy()?; }
        if self.preparation_list_limit.is_some() || self.preparation_history_limit.is_some() || self.preparation_method_max_bytes.is_some() {
            self.preparation_read_policy()?;
            if self.agent_report_clock_skew_seconds.is_none() {
                return Err(refuse("settings-preparation-requires-report-clock-policy"));
            }
        }
        if self.agent_report_clock_skew_seconds.is_some() { self.report_clock_skew_seconds()?; }
        if self.work_view_status_recent_agents.is_some() || self.work_view_status_change_events.is_some() {
            self.mission_status_view_limits()?;
        }
        if self.has_runtime_storage_policy() { self.runtime_storage_policy()?; }
        if let Some(cap) = self.mission_dispatch_cost_reserve_percent {
            if let Some(value) = cap.0 { nonnegative(value)?; }
            if !self.has_mission_dispatch_policy() {
                return Err(refuse("settings-mission-cost-requires-dispatch-policy"));
            }
        }
        let limits = self.mutation_limits()?;
        if limits.task.max_attempts < 1
            || limits.task.max_tool_calls < 1
            || limits.task.max_criteria < 1
        {
            return Err(refuse("settings-task-cap-positive-or-unlimited"));
        }
        if self.validation_default_timeout_seconds.0 < 1
            || limits.policy.max_timeout_seconds < self.validation_default_timeout_seconds.0
            || limits.policy.max_total_timeout_seconds < 1
            || limits.policy.min_justification_chars > limits.policy.max_justification_chars
        {
            return Err(refuse("settings-validation-bounds-inconsistent"));
        }
        if nonnegative(self.storage_busy_timeout_ms)? > i64::from(i32::MAX) {
            return Err(refuse("settings-sqlite-busy-timeout-representation"));
        }
        nonnegative(self.storage_busy_retry_delay_ms)?;
        self.storage_busy_retries.map(nonnegative).transpose()?;
        if let Some(SettingCap(value)) = self.validation_max_output_bytes {
            size(value)?;
        }
        if self.has_quality_policy() {
            self.quality_policy_values()?;
            if self.validation_max_output_bytes.is_none() {
                return Err(refuse("settings-quality-profile-requires-output-policy"));
            }
        }
        if self.has_context_policy() {
            self.context_policy_limits()?;
            if !self.has_quality_policy() {
                return Err(refuse("settings-context-requires-quality-policy"));
            }
        }
        if self.git_command_timeout_seconds.is_some() {
            self.git_command_timeout()?;
            if !self.has_context_policy() {
                return Err(refuse("settings-git-requires-context-policy"));
            }
        }
        if self.managed_review_max_baseline_depth.is_some() {
            self.managed_review_baseline_depth()?;
            if self.git_command_timeout_seconds.is_none() {
                return Err(refuse("settings-review-baseline-requires-git-policy"));
            }
        }
        if self.has_review_reply_policy() {
            self.review_reply_limits()?;
            if self.managed_review_max_baseline_depth.is_none() {
                return Err(refuse("settings-review-reply-requires-baseline-policy"));
            }
        }
        if self.has_work_view_policy() {
            self.work_view_limits()?;
            if !self.has_review_reply_policy() {
                return Err(refuse("settings-work-view-requires-review-policy"));
            }
        }
        if self.task_manual_required_gate_phases.is_some() && !self.has_work_view_policy() {
            return Err(refuse("settings-task-gate-defaults-requires-view-policy"));
        }
        if self.managed_receipt_max_bytes.is_some() {
            self.managed_receipt_limit()?;
            if self.task_manual_required_gate_phases.is_none() {
                return Err(refuse("settings-managed-receipt-requires-phase-policy"));
            }
        }
        if self.planning_resume_max_reason_bytes.is_some() {
            self.planning_resume_reason_limit()?;
            if self.managed_receipt_max_bytes.is_none() {
                return Err(refuse("settings-planning-resume-requires-receipt-policy"));
            }
        }
        if self.planning_request_max_bytes.is_some() || self.planning_max_requirements.is_some() {
            self.planning_request_limit()?;
            self.planning_requirement_limit()?;
            if self.planning_resume_max_reason_bytes.is_none() {
                return Err(refuse("settings-planning-request-requires-resume-policy"));
            }
        }
        if self.agent_workflow_max_body_bytes.is_some() {
            self.agent_workflow_body_limit()?;
            if self.planning_request_max_bytes.is_none() {
                return Err(refuse("settings-workflow-requires-planning-policy"));
            }
        }
        if self.planning_report_max_bytes.is_some() {
            self.planning_report_limit()?;
            if self.agent_workflow_max_body_bytes.is_none() {
                return Err(refuse("settings-planning-report-requires-workflow-policy"));
            }
        }
        if self.has_runtime_limit_policy() {
            self.runtime_limit_policy()?;
            if self.planning_report_max_bytes.is_none() {
                return Err(refuse("settings-runtime-requires-planning-report-policy"));
            }
        }
        if self.has_agent_launch_policy() {
            self.agent_launch_policy()?;
            if !self.has_runtime_limit_policy() {
                return Err(refuse("settings-launch-requires-runtime-policy"));
            }
        }
        if self.agent_interleaved_failure_lookback_multiplier.is_some() {
            self.tool_guard_policy()?;
            if !self.has_agent_launch_policy() {
                return Err(refuse("settings-tool-guard-requires-launch-policy"));
            }
        }
        if self.has_agent_execution_policy() {
            self.agent_execution_policy()?;
            if self.agent_interleaved_failure_lookback_multiplier.is_none() {
                return Err(refuse("settings-supervision-requires-guard-policy"));
            }
        }
        if self.has_agent_model_routing_policy() {
            self.agent_model_routing_policy()?;
            if !self.has_agent_execution_policy() {
                return Err(refuse("settings-model-routing-requires-supervision-policy"));
            }
        }
        if self.has_web_transport_policy() {
            self.web_transport_policy()?;
            if !self.has_agent_model_routing_policy() {
                return Err(refuse("settings-web-requires-model-routing-policy"));
            }
        }
        if self.has_mission_supervision_policy() {
            self.mission_supervision_policy()?;
            if !self.has_web_transport_policy() {
                return Err(refuse("settings-mission-supervision-requires-web-policy"));
            }
        }
        if self.has_mission_dispatch_policy() {
            self.mission_dispatch_policy()?;
            if !self.has_mission_supervision_policy() {
                return Err(refuse(
                    "settings-mission-dispatch-requires-supervision-policy",
                ));
            }
        }
        if self.has_reviewer_creation_policy() {
            self.reviewer_creation_defaults()?;
            if !self.has_mission_dispatch_policy() {
                return Err(refuse(
                    "settings-reviewer-creation-requires-dispatch-policy",
                ));
            }
        }
        if self.has_planning_wait_policy() {
            self.planning_wait_policy()?;
            if self.planning_response_max_bytes.is_none() {
                return Err(refuse("settings-planning-wait-requires-response-policy"));
            }
        }
        if self.planning_response_max_bytes.is_some() {
            self.planning_response_limit()?;
            if !self.has_planning_decision_policy() {
                return Err(refuse(
                    "settings-planning-response-requires-decisions-policy",
                ));
            }
        }
        if self.has_planning_decision_policy() {
            self.planning_decision_policy()?;
            if !self.has_planning_action_policy() {
                return Err(refuse("settings-planning-decisions-require-actions-policy"));
            }
        }
        if let Some(cap) = self.agent_evaluation_sample_percent {
            if cap.0.is_some_and(|n| !(1..=100).contains(&n.0)) {
                return Err(refuse("settings-evaluation-sample-percent-range"));
            }
            if self.annotation_note_max_chars.is_none() {
                return Err(refuse("settings-evaluation-requires-annotation-policy"));
            }
        }
        if let Some(cap) = self.annotation_note_max_chars {
            if cap.0.is_some_and(|n| n.0 == 0) {
                return Err(refuse("settings-annotation-note-positive-or-unlimited"));
            }
            if self.agent_alert_evaluation_seconds.is_none() {
                return Err(refuse("settings-annotation-requires-alert-policy"));
            }
        }
        if self.agent_alert_evaluation_seconds.is_some() || self.work_budget_warning_percent.is_some() {
            if self.agent_alert_evaluation_seconds.is_none() || self.work_budget_warning_percent.is_none() {
                return Err(refuse("settings-alert-policy-incomplete"));
            }
            if self.alert_evaluation_seconds() == Some(0) {
                return Err(refuse("settings-alert-evaluation-positive-or-off"));
            }
            if self.budget_warning_percent().is_some_and(|p| !(1..=100).contains(&p)) {
                return Err(refuse("settings-budget-warning-percent-range"));
            }
            if self.agent_usage_mismatch_percent.is_none() {
                return Err(refuse("settings-alert-policy-requires-usage-mismatch-policy"));
            }
        }
        if self.agent_usage_mismatch_percent.is_some() && self.agent_telemetry_providers.is_none() {
            return Err(refuse("settings-usage-mismatch-requires-telemetry-policy"));
        }
        if self.agent_telemetry_retention_days.is_some() || self.agent_telemetry_max_events_per_attempt.is_some() {
            self.telemetry_retention_policy()?;
            if self.agent_telemetry_providers.is_none() {
                return Err(refuse("settings-telemetry-retention-requires-telemetry-policy"));
            }
        }
        if self.agent_cache_miss_ratio_percent.is_some() || self.agent_cache_min_input_tokens.is_some() {
            self.cache_health_policy()?
                .ok_or_else(|| refuse("settings-cache-health-incomplete"))?;
            if self.agent_telemetry_providers.is_none() {
                return Err(refuse("settings-cache-health-requires-telemetry-policy"));
            }
        }
        if self.agent_telemetry_providers.is_some() {
            self.telemetry_providers()?;
            if self.review_response_max_bytes.is_none() {
                return Err(refuse("settings-telemetry-requires-review-response-policy"));
            }
        }
        if self.review_response_max_bytes.is_some() {
            self.review_response_limit()?;
            if !self.has_quota_reason_policy() {
                return Err(refuse("settings-review-response-requires-quota-policy"));
            }
        }
        if self.has_quota_reason_policy() {
            self.quota_reason_policy()?;
            if !self.has_planning_wait_policy() {
                return Err(refuse("settings-quota-reason-requires-wait-policy"));
            }
        }
        if self.has_planning_action_policy() {
            self.planning_action_policy()?;
            if !self.has_reviewer_creation_policy() {
                return Err(refuse("settings-planning-actions-require-reviewer-policy"));
            }
        }
        if self
            .validation_allowed_programs
            .iter()
            .any(|p| !swarm_identity_contract::name(p.as_bytes()))
        {
            return Err(refuse("settings-program-name-invalid"));
        }
        Ok(())
    }
    /// Read explicit status windows (recent agents, raw change events). Each
    /// inner null is unlimited, zero is an empty view. Historical absence or a
    /// partial group refuses; invalid/unsupported platform counts retain typed
    /// causes. Pure: never adopts defaults, prunes history, or limits accounting.
    pub fn mission_status_view_limits(&self) -> Result<(Option<usize>, Option<usize>), EngineSettingsError> {
        let read = |value: Option<SettingCap>| -> Result<Option<usize>, EngineSettingsError> {
            let value = value.ok_or_else(|| refuse("settings-status-view-not-configured"))?;
            value.0.map(|count| size(Some(count))).transpose()
        };
        Ok((read(self.work_view_status_recent_agents)?, read(self.work_view_status_change_events)?))
    }
    /// Read the explicitly saved automatic report-relay tolerance, in seconds.
    /// Zero accepts mtime at/after attempt start; historical absence refuses
    /// without adopting defaults. Other policy groups validate independently;
    /// a schema number alone does not make display windows a relay prerequisite.
    /// Pure: typed setting errors survive; no IO, clock or author proof follows.
    pub fn report_clock_skew_seconds(&self) -> Result<i64, EngineSettingsError> {
        nonnegative(self.agent_report_clock_skew_seconds
            .ok_or_else(|| refuse("settings-report-clock-skew-not-configured"))?)
    }
    /// Detect supplied schema38 storage members; incomplete groups must refuse.
    fn has_runtime_storage_policy(&self) -> bool {
        self.runtime_storage_stop_bytes.is_some() || self.runtime_storage_warning_bytes.is_some() || self.runtime_storage_stop_inodes.is_some()
    }
    /// Read explicitly saved runtime storage thresholds (bytes, warning bytes,
    /// inodes). Zero disables only that threshold; unknown probes always block.
    /// Missing/partial groups and warning below stop refuse without IO/adoption.
    pub fn runtime_storage_policy(&self) -> Result<RuntimeStoragePolicy, EngineSettingsError> {
        let missing = || refuse("settings-runtime-storage-not-configured");
        let stop_bytes = nonnegative(self.runtime_storage_stop_bytes.ok_or_else(missing)?)? as u64;
        let warning_bytes = nonnegative(self.runtime_storage_warning_bytes.ok_or_else(missing)?)? as u64;
        let stop_inodes = nonnegative(self.runtime_storage_stop_inodes.ok_or_else(missing)?)? as u64;
        if warning_bytes < stop_bytes { return Err(refuse("settings-runtime-storage-warning-below-stop")); }
        Ok(RuntimeStoragePolicy { stop_bytes, warning_bytes, stop_inodes })
    }
    /// Read all persisted preparation bounds; missing, negative or unrepresentable
    /// values refuse. Null is unlimited; zero rows hides history without deleting
    /// it, zero bytes refuses nonempty method text. No adoption, IO or authority.
    pub fn preparation_read_policy(&self) -> Result<PreparationReadPolicy, EngineSettingsError> {
        let read = |value: Option<SettingCap>| {
            value.ok_or_else(|| refuse("settings-preparation-read-not-configured"))?
                .0.map(|value| size(Some(value))).transpose()
        };
        Ok(PreparationReadPolicy { list_limit: read(self.preparation_list_limit)?,
            history_limit: read(self.preparation_history_limit)?,
            method_max_bytes: read(self.preparation_method_max_bytes)? })
    }
    /// Whether any v42 editing field was supplied, without adopting defaults.
    fn has_preparation_write_policy(&self) -> bool {
        self.preparation_write_enabled.is_some() || self.preparation_document_max_bytes.is_some() || self.preparation_title_max_bytes.is_some() || self.preparation_default_method.is_some()
    }
    /// Require the complete saved draft editing group. Enabled is exactly 0/1;
    /// null size caps mean unlimited and zero rejects nonempty input. The default
    /// method must name a supported capability. No IO/adoption/launch occurs.
    pub fn preparation_write_policy(&self) -> Result<PreparationWritePolicy, EngineSettingsError> {
        let missing = || refuse("settings-preparation-write-not-configured");
        let enabled = self.preparation_write_enabled.ok_or_else(missing)?.0;
        if !matches!(enabled,0|1) { return Err(refuse("settings-preparation-write-enabled-invalid")); }
        let cap = |v: Option<SettingCap>| v.ok_or_else(missing)?.0.map(|n|size(Some(n))).transpose();
        let method = self.preparation_default_method.as_deref().ok_or_else(missing)?;
        if !matches!(method,"apex"|"debug"|"audit-pdca"|"ks-feature"|"ks-product") { return Err(refuse("settings-preparation-default-method-invalid")); }
        Ok(PreparationWritePolicy { enabled: enabled==1, document_max_bytes: cap(self.preparation_document_max_bytes)?, title_max_bytes: cap(self.preparation_title_max_bytes)?, default_method: method.into() })
    }
    /// Return the complete declared settings version, preserving historical
    /// absence. This does not adopt defaults, mutate history or grant authority.
    pub fn schema_version(&self) -> i64 {
        if self.has_preparation_write_policy() { return 42; }
        if self.preparation_list_limit.is_some() || self.preparation_history_limit.is_some() || self.preparation_method_max_bytes.is_some() {
            41
        } else if self.agent_report_clock_skew_seconds.is_some() {
            40
        } else if self.work_view_status_recent_agents.is_some() || self.work_view_status_change_events.is_some() {
            39
        } else if self.has_runtime_storage_policy() {
            38
        } else if self.agent_evaluation_sample_percent.is_some() {
            37
        } else if self.annotation_note_max_chars.is_some() {
            36
        } else if self.agent_alert_evaluation_seconds.is_some() || self.work_budget_warning_percent.is_some() {
            35
        } else if self.agent_usage_mismatch_percent.is_some() {
            34
        } else if self.agent_telemetry_retention_days.is_some() || self.agent_telemetry_max_events_per_attempt.is_some() {
            33
        } else if self.mission_dispatch_cost_reserve_percent.is_some() {
            32
        } else if self.agent_cache_miss_ratio_percent.is_some() || self.agent_cache_min_input_tokens.is_some() {
            31
        } else if self.agent_telemetry_providers.is_some() {
            30
        } else if self.review_response_max_bytes.is_some() {
            29
        } else if self.has_quota_reason_policy() {
            28
        } else if self.has_planning_wait_policy() {
            27
        } else if self.planning_response_max_bytes.is_some() {
            26
        } else if self.has_planning_decision_policy() {
            25
        } else if self.has_planning_action_policy() {
            24
        } else if self.has_reviewer_creation_policy() {
            23
        } else if self.has_mission_dispatch_policy() {
            22
        } else if self.has_mission_supervision_policy() {
            21
        } else if self.has_web_transport_policy() {
            20
        } else if self.has_agent_model_routing_policy() {
            19
        } else if self.has_agent_execution_policy() {
            18
        } else if self.agent_interleaved_failure_lookback_multiplier.is_some() {
            17
        } else if self.has_agent_launch_policy() {
            16
        } else if self.has_runtime_limit_policy() {
            15
        } else if self.planning_report_max_bytes.is_some() {
            14
        } else if self.agent_workflow_max_body_bytes.is_some() {
            13
        } else if self.planning_request_max_bytes.is_some() {
            12
        } else if self.planning_resume_max_reason_bytes.is_some() {
            11
        } else if self.managed_receipt_max_bytes.is_some() {
            10
        } else if self.task_manual_required_gate_phases.is_some() {
            9
        } else if self.has_work_view_policy() {
            8
        } else if self.has_review_reply_policy() {
            7
        } else if self.managed_review_max_baseline_depth.is_some() {
            6
        } else if self.git_command_timeout_seconds.is_some() {
            5
        } else if self.has_context_policy() {
            4
        } else if self.has_quality_policy() {
            3
        } else if self.validation_max_output_bytes.is_some() {
            2
        } else {
            1
        }
    }
    /// Detect any quota audit value without treating absence as a default.
    pub(crate) fn has_quota_reason_policy(&self) -> bool {
        self.planning_quota_reason_min_bytes.is_some() || self.planning_quota_reason_max_bytes.is_some()
    }
    /// Read both persisted v28 audit-text limits. Null maximum is explicit;
    /// missing/partial/negative/incoherent values refuse without fallback or IO.
    /// Zero minimum removes only the extra minimum; no actor is authenticated.
    pub fn quota_reason_policy(&self) -> Result<crate::QuotaReasonPolicy, EngineSettingsError> {
        let missing=||refuse("settings-quota-reason-not-configured");
        let policy=crate::QuotaReasonPolicy {
            minimum_trimmed_bytes:self.planning_quota_reason_min_bytes.ok_or_else(missing)?,
            maximum_raw_bytes:self.planning_quota_reason_max_bytes.ok_or_else(missing)?,
        };
        policy.validate()?;
        Ok(policy)
    }
    fn has_work_view_policy(&self) -> bool {
        self.evidence_longest_measured_count.is_some()
            || self.work_view_recent_decisions.is_some()
            || self.work_view_wrap_characters.is_some()
    }
    /// Read the complete explicit v8 presentation group, preserving legacy
    /// absence. Null means all items/no wrapping, zero hides items, but a zero
    /// wrap width refuses because it cannot make progress. No history is pruned.
    pub fn work_view_limits(&self) -> Result<crate::WorkViewLimits, EngineSettingsError> {
        let read = |cap: Option<SettingCap>| {
            cap.ok_or_else(|| refuse("settings-work-view-policy-not-configured"))?
                .0
                .map(|v| size(Some(v)))
                .transpose()
        };
        let result = crate::WorkViewLimits {
            longest_measured_count: read(self.evidence_longest_measured_count)?,
            recent_decisions: read(self.work_view_recent_decisions)?,
            wrap_characters: read(self.work_view_wrap_characters)?,
        };
        if result.wrap_characters == Some(0) {
            return Err(refuse("settings-work-view-wrap-positive"));
        }
        Ok(result)
    }
    /// Detect any v7 field so incomplete groups cannot masquerade as legacy policy.
    fn has_review_reply_policy(&self) -> bool {
        self.review_reply_min_text_bytes.is_some()
            || self.review_reply_max_text_bytes.is_some()
            || self.review_quote_min_normalized_bytes.is_some()
    }
    /// Read all v7 limits explicitly, rejecting absence, negatives, representation
    /// overflow and a maximum below the minimum. Zero removes a minimum; null
    /// removes only the maximum. No fallback, state mutation or authority is added.
    pub fn review_reply_limits(&self) -> Result<crate::ReviewReplyLimits, EngineSettingsError> {
        let min = self
            .review_reply_min_text_bytes
            .ok_or_else(|| refuse("settings-review-reply-policy-not-configured"))?;
        let max = self
            .review_reply_max_text_bytes
            .ok_or_else(|| refuse("settings-review-reply-policy-not-configured"))?;
        let normalized = self
            .review_quote_min_normalized_bytes
            .ok_or_else(|| refuse("settings-review-reply-policy-not-configured"))?;
        let result = crate::ReviewReplyLimits {
            min_text_bytes: size(Some(min))?,
            max_text_bytes: max.0.map(|v| size(Some(v))).transpose()?,
            min_normalized_quote_bytes: size(Some(normalized))?,
        };
        if result
            .max_text_bytes
            .is_some_and(|max| max < result.min_text_bytes)
        {
            return Err(refuse("settings-review-reply-bounds-inconsistent"));
        }
        Ok(result)
    }
    /// Return the explicitly configured Git deadline duration; no default is inferred.
    ///
    /// Zero expires immediately; explicit null disables this operator deadline.
    /// Legacy journals lacking v5 refuse with `settings-git-policy-not-configured`.
    /// Negative programmatic values are rejected. No process is started and no
    /// execution authority is established; consumers check monotonic representation.
    pub fn git_command_timeout(&self) -> Result<Option<std::time::Duration>, EngineSettingsError> {
        let SettingCap(value) = self
            .git_command_timeout_seconds
            .ok_or_else(|| refuse("settings-git-policy-not-configured"))?;
        value
            .map(|value| nonnegative(value).map(|n| std::time::Duration::from_secs(n as u64)))
            .transpose()
    }
    /// Return the configured number of allowed baseline edges below root depth0.
    /// Zero permits only the root, null disables this depth cap. Legacy absence
    /// refuses instead of silently inheriting128. Negative/unrepresentable values
    /// refuse. This reads data only and grants no review or execution authority.
    pub fn managed_review_baseline_depth(&self) -> Result<Option<usize>, EngineSettingsError> {
        let SettingCap(value) = self
            .managed_review_max_baseline_depth
            .ok_or_else(|| refuse("settings-review-baseline-policy-not-configured"))?;
        value.map(|v| size(Some(v))).transpose()
    }
    /// Return the explicit receipt read cap in bytes, separate from delivery text.
    /// Zero rejects nonempty receipts; null permits unlimited allocation. Legacy
    /// absence, negative counts or platform overflow refuse without fallback or IO.
    /// Consumers must refuse oversize input before hash/parse, never truncate it.
    /// This supplies no identity, review verdict or authority.
    pub fn managed_receipt_limit(&self) -> Result<Option<usize>, EngineSettingsError> {
        let SettingCap(value) = self
            .managed_receipt_max_bytes
            .ok_or_else(|| refuse("settings-managed-receipt-policy-not-configured"))?;
        value.map(|v| size(Some(v))).transpose()
    }
    /// Return the explicit cap for a nonblank operator resume reason, in bytes.
    /// Zero rejects such reasons, null removes this cap. Pause and a resume with
    /// no nonblank reason do not consume it; legacy absence must not infer4000.
    /// Negative/platform-overflow values refuse. No IO, adoption, operation,
    /// caller authority or process cancellation occurs here.
    pub fn planning_resume_reason_limit(&self) -> Result<Option<usize>, EngineSettingsError> {
        let SettingCap(value) = self
            .planning_resume_max_reason_bytes
            .ok_or_else(|| refuse("settings-planning-resume-policy-not-configured"))?;
        value.map(|v| size(Some(v))).transpose()
    }
    /// Read the saved canonical planning intent cap in bytes. Zero rejects every
    /// nonempty intent; null removes this policy cap, not the external transport
    /// bound. Missing legacy adoption, negative values or platform overflow refuse.
    /// No fallback, IO, publication or authority is established.
    pub fn planning_request_limit(&self) -> Result<Option<usize>, EngineSettingsError> {
        let SettingCap(value) = self
            .planning_request_max_bytes
            .ok_or_else(|| refuse("settings-planning-request-policy-not-configured"))?;
        value.map(|v| size(Some(v))).transpose()
    }
    /// Read the saved requirement count for a new hierarchy. Zero disables enable;
    /// null removes this cap. Existing hierarchy and task limits remain separate.
    /// Absence/negative/platform overflow refuse without adoption, IO or authority.
    pub fn planning_requirement_limit(&self) -> Result<Option<usize>, EngineSettingsError> {
        let SettingCap(value) = self
            .planning_max_requirements
            .ok_or_else(|| refuse("settings-planning-requirement-policy-not-configured"))?;
        value.map(|v| size(Some(v))).transpose()
    }
    /// Return saved base workflow body bytes: zero refuses nonempty framing,
    /// null removes this cap. Metadata, project context and model capacity remain
    /// separate. Legacy absence/negative/platform overflow refuse without fallback,
    /// adoption, IO, delegation or any proof of agent identity or authority.
    pub fn agent_workflow_body_limit(&self) -> Result<Option<usize>, EngineSettingsError> {
        let SettingCap(value) = self
            .agent_workflow_max_body_bytes
            .ok_or_else(|| refuse("settings-agent-workflow-policy-not-configured"))?;
        value.map(|v| size(Some(v))).transpose()
    }
    /// Read the persisted complete-report cap: zero rejects every nonempty
    /// report, null removes this cap; legacy absence requires explicit adoption.
    /// Negative/platform overflow refuses. No IO, fallback, mutation, model
    /// capacity, identity or authority is established by this conversion.
    pub fn planning_report_limit(&self) -> Result<Option<usize>, EngineSettingsError> {
        let SettingCap(value) = self
            .planning_report_max_bytes
            .ok_or_else(|| refuse("settings-planning-report-policy-not-configured"))?;
        value.map(|v| size(Some(v))).transpose()
    }
    /// Whether any v15 runtime field is present. Partial groups remain invalid;
    /// this never adopts missing defaults or grants observation authority.
    pub fn has_runtime_limit_policy(&self) -> bool {
        self.runtime_silence_seconds_default.is_some()
            || self.runtime_silence_seconds_ceiling.is_some()
            || self.runtime_tool_seconds_default.is_some()
            || self.runtime_tool_seconds_ceiling.is_some()
            || self.runtime_max_tool_calls_default.is_some()
            || self.runtime_max_tool_calls_ceiling.is_some()
            || self.runtime_max_repeated_calls_default.is_some()
            || self.runtime_max_repeated_calls_ceiling.is_some()
            || self.runtime_max_consecutive_errors_default.is_some()
            || self.runtime_max_consecutive_errors_ceiling.is_some()
    }
    /// Resolve the complete saved runtime defaults and nullable admission ceilings.
    /// Historical absence/partial policy, nonpositive defaults, negative ceilings
    /// and defaults above ceilings refuse. Null removes the definition ceiling,
    /// not a frozen attempt's own budget. No IO, timer, retry or launch occurs.
    pub fn runtime_limit_policy(&self) -> Result<crate::RuntimeLimitPolicy, EngineSettingsError> {
        let missing = || refuse("settings-runtime-policy-not-configured");
        let defaults = [
            self.runtime_silence_seconds_default.ok_or_else(missing)?.0,
            self.runtime_tool_seconds_default.ok_or_else(missing)?.0,
            self.runtime_max_tool_calls_default.ok_or_else(missing)?.0,
            self.runtime_max_repeated_calls_default
                .ok_or_else(missing)?
                .0,
            self.runtime_max_consecutive_errors_default
                .ok_or_else(missing)?
                .0,
        ];
        let ceilings = [
            self.runtime_silence_seconds_ceiling
                .ok_or_else(missing)?
                .0
                .map(|v| v.0),
            self.runtime_tool_seconds_ceiling
                .ok_or_else(missing)?
                .0
                .map(|v| v.0),
            self.runtime_max_tool_calls_ceiling
                .ok_or_else(missing)?
                .0
                .map(|v| v.0),
            self.runtime_max_repeated_calls_ceiling
                .ok_or_else(missing)?
                .0
                .map(|v| v.0),
            self.runtime_max_consecutive_errors_ceiling
                .ok_or_else(missing)?
                .0
                .map(|v| v.0),
        ];
        if defaults
            .iter()
            .zip(ceilings)
            .any(|(value, cap)| *value < 1 || cap.is_some_and(|c| c < *value))
        {
            return Err(refuse("settings-runtime-policy-bounds-inconsistent"));
        }
        Ok(crate::RuntimeLimitPolicy::new(defaults, ceilings))
    }
    /// Presence detects the version group only; partial policies remain invalid.
    /// No adoption or environment privilege is inferred from historical absence.
    pub fn has_agent_launch_policy(&self) -> bool {
        self.agent_launch_timeout_seconds_default.is_some()
            || self.agent_launch_timeout_seconds_ceiling.is_some()
            || self.agent_launch_instruction_max_bytes.is_some()
            || self.agent_provider_config_max_bytes.is_some()
            || self.agent_launch_context_max_bytes.is_some()
            || self.agent_preflight_ttl_seconds_default.is_some()
            || self.agent_preflight_ttl_seconds_ceiling.is_some()
            || self.agent_preflight_min_free_bytes.is_some()
            || self.agent_environment_allow.is_some()
            || self.agent_allow_unverified_provider_context.is_some()
    }
    /// Resolve saved preparation policy, requiring complete explicit adoption.
    /// Positive duration defaults must fit nullable definition ceilings; byte caps
    /// may be zero or unlimited, free space zero disables that host-resource check.
    /// Environment names are explicit, unique POSIX names; values are never stored.
    /// Invalid/incomplete policy refuses without IO, mutation or execution rights.
    pub fn agent_launch_policy(&self) -> Result<crate::AgentLaunchPolicy, EngineSettingsError> {
        let missing = || refuse("settings-agent-launch-policy-not-configured");
        let timeout = self
            .agent_launch_timeout_seconds_default
            .ok_or_else(missing)?
            .0;
        let timeout_cap = self
            .agent_launch_timeout_seconds_ceiling
            .ok_or_else(missing)?
            .0
            .map(|v| v.0);
        let ttl = self
            .agent_preflight_ttl_seconds_default
            .ok_or_else(missing)?
            .0;
        let ttl_cap = self
            .agent_preflight_ttl_seconds_ceiling
            .ok_or_else(missing)?
            .0
            .map(|v| v.0);
        if timeout < 1
            || ttl < 1
            || timeout_cap.is_some_and(|v| v < timeout)
            || ttl_cap.is_some_and(|v| v < ttl)
        {
            return Err(refuse("settings-agent-launch-durations-inconsistent"));
        }
        let compatibility = self
            .agent_allow_unverified_provider_context
            .ok_or_else(missing)?
            .0;
        if !matches!(compatibility, 0 | 1) {
            return Err(refuse("settings-agent-provider-compatibility-invalid"));
        }
        let environment = self
            .agent_environment_allow
            .as_ref()
            .ok_or_else(missing)?
            .clone();
        let mut seen = BTreeSet::new();
        for name in &environment {
            if !crate::agent_launch_policy::environment_name(name) || !seen.insert(name) {
                return Err(refuse("settings-agent-environment-name-invalid"));
            }
        }
        let bytes = |cap: Option<SettingCap>| {
            let SettingCap(v) = cap.ok_or_else(missing)?;
            v.map(|n| size(Some(n))).transpose()
        };
        Ok(crate::AgentLaunchPolicy {
            allow_unverified: compatibility == 1,
            timeout,
            timeout_cap,
            ttl,
            ttl_cap,
            instruction_bytes: bytes(self.agent_launch_instruction_max_bytes)?,
            provider_bytes: bytes(self.agent_provider_config_max_bytes)?,
            context_bytes: bytes(self.agent_launch_context_max_bytes)?,
            minimum_free_bytes: nonnegative(
                self.agent_preflight_min_free_bytes.ok_or_else(missing)?,
            )? as u64,
            environment,
        })
    }
    /// Resolve the explicitly saved detection window; missing v17 values refuse.
    /// Null means no expiry, positive factors multiply the frozen repetition cap.
    /// This pure getter does not adopt policy or grant execution; callers must
    /// validate the complete settings document before freezing a new intention.
    pub fn tool_guard_policy(&self) -> Result<crate::AgentGuardPolicy, EngineSettingsError> {
        let cap = self
            .agent_interleaved_failure_lookback_multiplier
            .ok_or_else(|| refuse("settings-tool-guard-policy-not-configured"))?;
        crate::AgentGuardPolicy::from_saved(cap.0.map(|v| v.0))
    }
    /// Detect any settings27 wait field, including incomplete supplied groups.
    /// This is historical presence, not a claim that a provider was started.
    pub(crate) fn has_planning_wait_policy(&self) -> bool {
        self.planning_provider_silence_seconds.is_some()
            || self.planning_provider_max_duration_seconds.is_some()
            || self.planning_default_lease_seconds.is_some()
            || self.planning_lease_renew_after_percent.is_some()
    }
    /// Read complete persisted settings27 without fallback, clamp, IO or timer.
    /// Silence/maximum/renewal0 each disable that mechanism; the positive default
    /// lease must satisfy current settings24 bounds. A maintained lease is not
    /// a maximum provider duration. Runtime must use suspend-aware observations,
    /// actual identity/lease fences and the same saved values; this getter grants
    /// neither launch permission, legacy-file adoption nor process termination.
    pub fn planning_wait_policy(&self) -> Result<crate::PlanningWaitPolicy, EngineSettingsError> {
        let missing = || refuse("settings-planning-wait-not-configured");
        let policy = crate::PlanningWaitPolicy {
            silence_seconds: self.planning_provider_silence_seconds.ok_or_else(missing)?,
            max_duration_seconds: self
                .planning_provider_max_duration_seconds
                .ok_or_else(missing)?,
            default_lease_seconds: self.planning_default_lease_seconds.ok_or_else(missing)?,
            renew_after_percent: self
                .planning_lease_renew_after_percent
                .ok_or_else(missing)?,
        };
        policy.validate(&self.planning_action_policy()?)?;
        Ok(policy)
    }
    /// Read the explicit v26 structured-reply cap in UTF8 bytes. Historical
    /// absence, negatives and platform-unrepresentable values refuse with safe
    /// causes. Null is unlimited, zero forbids nonempty output. No defaults, IO,
    /// truncation or provider/action authority are inferred; current runtime
    /// consumers must recheck saved values at their relevant boundaries.
    pub fn planning_response_limit(&self) -> Result<Option<usize>, EngineSettingsError> {
        self.planning_response_max_bytes
            .ok_or_else(|| refuse("settings-planning-response-not-configured"))?
            .0
            .map(|v| {
                usize::try_from(v.0).map_err(|_| refuse("settings-planning-response-value-invalid"))
            })
            .transpose()
    }
    /// Read the explicit v29 reviewer reply cap in UTF8 bytes. Null removes this
    /// cap and zero forbids nonempty replies; event limits remain independent.
    /// Missing/negative/unrepresentable values refuse with typed validation;
    /// no planner fallback, adoption, truncation, IO or invocation grant occurs.
    pub fn review_response_limit(&self) -> Result<Option<usize>, EngineSettingsError> {
        self.review_response_max_bytes
            .ok_or_else(|| refuse("settings-review-response-not-configured"))?
            .0
            .map(|v| usize::try_from(v.0).map_err(|_| refuse("settings-review-response-value-invalid")))
            .transpose()
    }
    /// Tolerated declared-vs-telemetry output-token gap in percent (v34);
    /// `None` when the rule is off (explicit null) or the document predates
    /// v34 (no implicit default).
    pub fn usage_mismatch_percent(&self) -> Option<i64> {
        self.agent_usage_mismatch_percent.and_then(|cap| cap.0).map(|p| p.0)
    }
    /// Server-side alert evaluation cadence in seconds (v35); `None` when
    /// off (explicit null) or the document predates v35 (no implicit default).
    pub fn alert_evaluation_seconds(&self) -> Option<i64> {
        self.agent_alert_evaluation_seconds.and_then(|cap| cap.0).map(|p| p.0)
    }
    /// Evaluation sample share in percent (v37); `None` when off (explicit
    /// null) or the document predates v37 (no implicit default).
    pub fn evaluation_sample_percent(&self) -> Option<i64> {
        self.agent_evaluation_sample_percent.and_then(|cap| cap.0).map(|p| p.0)
    }
    /// Annotation note bound in characters (v36): `Some(Some(n))` bounded,
    /// `Some(None)` explicitly unlimited, `None` when the document predates
    /// v36 (the annotation route then refuses: `annotation-policy-missing`).
    pub fn annotation_note_limit(&self) -> Option<Option<i64>> {
        self.annotation_note_max_chars.map(|cap| cap.0.map(|n| n.0))
    }
    /// Budget warning share in percent of the Work limit (v35); `None` when
    /// off (explicit null) or the document predates v35.
    pub fn budget_warning_percent(&self) -> Option<i64> {
        self.work_budget_warning_percent.and_then(|cap| cap.0).map(|p| p.0)
    }
    /// Telemetry store bounds (v33) as (retention days, records per attempt),
    /// each `None` when explicitly unlimited. `Ok(None)` when the document
    /// predates v33: no purge and no cap apply (historical absence is not an
    /// adopted default). Refusals: a partial group
    /// (`settings-telemetry-retention-incomplete`), zero days
    /// (`settings-telemetry-retention-positive-or-unlimited`) or a zero cap
    /// (`settings-telemetry-cap-positive-or-unlimited`).
    pub fn telemetry_retention_policy(&self) -> Result<Option<TelemetryBounds>, EngineSettingsError> {
        match (self.agent_telemetry_retention_days, self.agent_telemetry_max_events_per_attempt) {
            (None, None) => Ok(None),
            (Some(days), Some(cap)) => {
                let days = days.0.map(|d| d.0);
                let cap = cap.0.map(|c| c.0);
                if days == Some(0) {
                    return Err(refuse("settings-telemetry-retention-positive-or-unlimited"));
                }
                if cap == Some(0) {
                    return Err(refuse("settings-telemetry-cap-positive-or-unlimited"));
                }
                Ok(Some((days, cap)))
            }
            _ => Err(refuse("settings-telemetry-retention-incomplete")),
        }
    }
    /// Cache-health display thresholds (v31) as (miss ratio percent, minimum
    /// total input tokens). `Ok(None)` when the document predates v31 (the
    /// console then shows the feature as not configured); a partial group
    /// (`settings-cache-health-incomplete`) or a percent above 100
    /// (`settings-cache-health-ratio-invalid`) refuses.
    pub fn cache_health_policy(&self) -> Result<Option<(i64, i64)>, EngineSettingsError> {
        match (self.agent_cache_miss_ratio_percent, self.agent_cache_min_input_tokens) {
            (None, None) => Ok(None),
            (Some(ratio), Some(min)) if ratio.0 <= 100 => Ok(Some((ratio.0, min.0))),
            (Some(_), Some(_)) => Err(refuse("settings-cache-health-ratio-invalid")),
            _ => Err(refuse("settings-cache-health-incomplete")),
        }
    }
    /// Providers opted into launch telemetry (v30). Absence and an explicit
    /// empty list both yield an empty slice (opt-in feature, not a cap).
    /// Refuses duplicates (`settings-telemetry-provider-duplicate`) and any
    /// provider without a supported injection (`settings-telemetry-provider-unsupported`;
    /// see `TELEMETRY_PROVIDERS`).
    pub fn telemetry_providers(&self) -> Result<&[String], EngineSettingsError> {
        let providers = self.agent_telemetry_providers.as_deref().unwrap_or_default();
        let mut seen = BTreeSet::new();
        for provider in providers {
            if !TELEMETRY_PROVIDERS.contains(&provider.as_str()) {
                return Err(refuse("settings-telemetry-provider-unsupported"));
            }
            if !seen.insert(provider.as_str()) {
                return Err(refuse("settings-telemetry-provider-duplicate"));
            }
        }
        Ok(providers)
    }
    /// Detect partial v25 groups; historical absence must remain distinguishable.
    pub(crate) fn has_planning_decision_policy(&self) -> bool {
        self.planning_worker_default_attempts.is_some()
            || self.planning_decision_max_inputs.is_some()
            || self.planning_decision_max_operations.is_some()
            || self.planning_decision_max_reason_bytes.is_some()
            || self.planning_decision_max_title_characters.is_some()
            || self.planning_decision_max_instruction_bytes.is_some()
            || self.planning_decision_max_dependencies.is_some()
            || self.planning_decision_max_criterion_bytes.is_some()
    }
    /// Read complete saved decision caps and positive worker attempts. Missing,
    /// negative, zero attempts or platform-unrepresentable settings refuse. Null
    /// caps stay unlimited; no request, lease, Work or retry right is changed.
    pub fn planning_decision_policy(
        &self,
    ) -> Result<crate::PlanningDecisionPolicy, EngineSettingsError> {
        let missing = || refuse("settings-planning-decisions-not-configured");
        let cap = |value: Option<SettingCap>| -> Result<Option<usize>, EngineSettingsError> {
            value
                .ok_or_else(missing)?
                .0
                .map(|v| {
                    usize::try_from(v.0)
                        .map_err(|_| refuse("settings-planning-decision-cap-invalid"))
                })
                .transpose()
        };
        let attempts = self.planning_worker_default_attempts.ok_or_else(missing)?.0;
        if attempts < 1 || attempts > self.mutation_limits()?.task.max_attempts {
            return Err(refuse("settings-planning-worker-attempts-invalid"));
        }
        Ok(crate::PlanningDecisionPolicy {
            max_inputs: cap(self.planning_decision_max_inputs)?,
            max_operations: cap(self.planning_decision_max_operations)?,
            max_reason_bytes: cap(self.planning_decision_max_reason_bytes)?,
            max_title_characters: cap(self.planning_decision_max_title_characters)?,
            max_instruction_bytes: cap(self.planning_decision_max_instruction_bytes)?,
            max_dependencies: cap(self.planning_decision_max_dependencies)?,
            max_criterion_bytes: cap(self.planning_decision_max_criterion_bytes)?,
            worker_default_attempts: attempts,
        })
    }
    /// Detect any v24 member; partial groups must not become historical absence.
    fn has_planning_action_policy(&self) -> bool {
        self.planning_scope_lease_min_seconds.is_some()
            || self.planning_scope_lease_max_seconds.is_some()
            || self.planning_context_max_events.is_some()
            || self.planning_context_max_bytes.is_some()
    }
    /// Resolve complete explicit scope bounds/projection caps. Missing, negative
    /// or unrepresentable values refuse, preserving null-as-unlimited and zero
    /// projection caps. Does not mutate deadlines, reserve work or grant authority.
    pub fn planning_action_policy(
        &self,
    ) -> Result<crate::PlanningActionPolicy, EngineSettingsError> {
        let missing = || refuse("settings-planning-actions-not-configured");
        let count = self.planning_scope_lease_min_seconds.ok_or_else(missing)?;
        let maximum = self.planning_scope_lease_max_seconds.ok_or_else(missing)?.0;
        let convert = |value: Option<SettingCap>| -> Result<Option<usize>, EngineSettingsError> {
            value
                .ok_or_else(missing)?
                .0
                .map(|v| {
                    usize::try_from(v.0)
                        .map_err(|_| refuse("settings-planning-projection-cap-invalid"))
                })
                .transpose()
        };
        let policy = crate::PlanningActionPolicy {
            lease_min_seconds: count,
            lease_max_seconds: maximum,
            context_max_events: convert(self.planning_context_max_events)?,
            context_max_bytes: convert(self.planning_context_max_bytes)?,
        };
        policy.validate()?;
        Ok(policy)
    }
    /// Detect any v23 value; partial groups cannot become legacy absence.
    fn has_reviewer_creation_policy(&self) -> bool {
        [
            self.reviewer_default_delivery_files,
            self.reviewer_default_context_files,
            self.reviewer_default_source_bytes,
            self.reviewer_default_context_bytes,
        ]
        .iter()
        .any(Option::is_some)
    }
    /// Build explicit initial reviewer defaults, preserving zero-as-unlimited.
    /// Historical/partial sets refuse. This neither updates existing reviewers,
    /// reads sources, spends calls nor proves their runtime enforcement.
    pub fn reviewer_creation_defaults(
        &self,
    ) -> Result<crate::ReviewerCreationDefaults, EngineSettingsError> {
        let missing = || refuse("settings-reviewer-creation-not-configured");
        let policy = crate::ReviewerCreationDefaults {
            delivery_files: self.reviewer_default_delivery_files.ok_or_else(missing)?,
            context_files: self.reviewer_default_context_files.ok_or_else(missing)?,
            source_bytes: self.reviewer_default_source_bytes.ok_or_else(missing)?,
            context_bytes: self.reviewer_default_context_bytes.ok_or_else(missing)?,
        };
        policy.validate()?;
        Ok(policy)
    }
    /// Detect any v22 member; partial sets must not become historical absence.
    fn has_mission_dispatch_policy(&self) -> bool {
        self.mission_dispatch_default_slots.is_some() || self.mission_dispatch_max_slots.is_some()
    }
    /// Build the complete explicit slot policy, without default adoption or IO.
    /// Historical/partial groups, zero/negative defaults, invalid caps and defaults
    /// beyond their cap refuse. This neither changes controls nor grants consent,
    /// provider capacity, workspace isolation or permission to dispatch.
    pub fn mission_dispatch_policy(
        &self,
    ) -> Result<crate::MissionDispatchPolicy, EngineSettingsError> {
        let missing = || refuse("settings-mission-dispatch-not-configured");
        let policy = crate::MissionDispatchPolicy {
            default_slots: self.mission_dispatch_default_slots.ok_or_else(missing)?,
            max_slots: self.mission_dispatch_max_slots.ok_or_else(missing)?.0,
        };
        policy.validate()?;
        Ok(policy)
    }
    /// Detect a partial v21 group without silently adopting installation values.
    fn has_mission_supervision_policy(&self) -> bool {
        self.mission_supervision_poll_interval_ms.is_some()
            || self.mission_supervision_stale_after_ms.is_some()
            || self.mission_supervision_future_tolerance_ms.is_some()
    }
    /// Build the complete explicit mission lease policy without IO or fallback.
    /// Historical absence/partial groups refuse. A conductor freezes this policy
    /// at claim; saving new values neither revokes that handle nor authenticates it.
    pub fn mission_supervision_policy(
        &self,
    ) -> Result<crate::MissionSupervisionPolicy, EngineSettingsError> {
        let missing = || refuse("settings-mission-supervision-not-configured");
        let policy = crate::MissionSupervisionPolicy {
            poll_interval_ms: self
                .mission_supervision_poll_interval_ms
                .ok_or_else(missing)?,
            stale_after_ms: self
                .mission_supervision_stale_after_ms
                .ok_or_else(missing)?,
            future_tolerance_ms: self
                .mission_supervision_future_tolerance_ms
                .ok_or_else(missing)?,
        };
        policy.validate()?;
        Ok(policy)
    }
    /// Detect any v20 member, refusing partial supplied sets during validation.
    /// Presence neither adopts defaults nor applies a runtime network policy.
    fn has_web_transport_policy(&self) -> bool {
        crate::web_policy::supplied(self)
    }
    /// Build the complete configured network group without default adoption or IO.
    /// Requires supplied members; containing-document validation owns previous
    /// group prerequisites and authority. Configured values take effect at server
    /// restart, not immediately. Technical bounds and typed parse/conversion causes
    /// are retained; this opens no listener and authenticates no actor/model.
    pub fn web_transport_policy(&self) -> Result<crate::WebTransportPolicy, EngineSettingsError> {
        crate::web_policy::build(self)
    }
    /// Detect a supplied v19 member so a partial set cannot masquerade as legacy.
    /// No default adoption, IO or authority follows from presence.
    fn has_agent_model_routing_policy(&self) -> bool {
        self.agent_model_automatic_levels.is_some()
            || self.agent_model_restricted_levels.is_some()
            || self.agent_model_restriction_rules.is_some()
    }
    /// Build routing restrictions from all three persisted arrays. Legacy absence
    /// and partial groups refuse; explicit empty sets keep their deny/no-rule
    /// meaning. Validation retains no default models and performs no mutation,
    /// catalogue discovery or process execution. See the policy's selection check
    /// for the further availability/identity/cost checks still required.
    pub fn agent_model_routing_policy(
        &self,
    ) -> Result<crate::AgentModelRoutingPolicy, EngineSettingsError> {
        let missing = || refuse("settings-model-routing-not-configured");
        crate::AgentModelRoutingPolicy::from_saved(
            self.agent_model_automatic_levels
                .as_deref()
                .ok_or_else(missing)?,
            self.agent_model_restricted_levels
                .as_deref()
                .ok_or_else(missing)?,
            self.agent_model_restriction_rules
                .as_deref()
                .ok_or_else(missing)?,
        )
    }
    /// Detect any supplied v18 member so partial groups cannot evade validation.
    /// This neither adopts installation values nor authorizes an execution.
    fn has_agent_execution_policy(&self) -> bool {
        self.agent_supervision_poll_interval_ms.is_some()
            || self.agent_supervision_heartbeat_stale_ms.is_some()
            || self.agent_supervision_term_grace_ms.is_some()
            || self.agent_supervision_output_drain_ms.is_some()
            || self.agent_provider_event_max_bytes.is_some()
            || self.agent_capture_max_bytes.is_some()
            || self.agent_capture_max_lines.is_some()
            || self.agent_public_message_max_bytes.is_some()
            || self.agent_log_message_max_bytes.is_some()
    }
    /// Read the complete explicitly saved supervision group. Missing historical
    /// or partial fields refuse; null is an explicit cap choice, never fallback.
    /// Pure validation performs no IO, timers, process or consumption reset.
    /// Callers must validate the full document and preserve the selected policy
    /// revision before actual native runtime use; this getter grants no authority.
    pub fn agent_execution_policy(
        &self,
    ) -> Result<crate::AgentExecutionPolicy, EngineSettingsError> {
        let missing = || refuse("settings-supervision-policy-not-configured");
        let policy = crate::AgentExecutionPolicy {
            poll_interval_ms: self
                .agent_supervision_poll_interval_ms
                .ok_or_else(missing)?,
            heartbeat_stale_ms: self
                .agent_supervision_heartbeat_stale_ms
                .ok_or_else(missing)?,
            term_grace_ms: self.agent_supervision_term_grace_ms.ok_or_else(missing)?,
            output_drain_ms: self.agent_supervision_output_drain_ms.ok_or_else(missing)?,
            event_max_bytes: self.agent_provider_event_max_bytes.ok_or_else(missing)?,
            capture_max_bytes: self.agent_capture_max_bytes.ok_or_else(missing)?,
            capture_max_lines: self.agent_capture_max_lines.ok_or_else(missing)?,
            public_message_max_bytes: self.agent_public_message_max_bytes.ok_or_else(missing)?,
            log_message_max_bytes: self.agent_log_message_max_bytes.ok_or_else(missing)?,
        };
        policy.validate()?;
        Ok(policy)
    }
    /// Legacy v1 is readable, but supplies no implicit execution policy.
    pub fn validation_output_limit(&self) -> Result<Option<usize>, EngineSettingsError> {
        let SettingCap(value) = self
            .validation_max_output_bytes
            .ok_or_else(|| refuse("settings-output-policy-not-configured"))?;
        value.map(|v| size(Some(v))).transpose()
    }
    /// Resolve persisted task/control limits, retaining absent historical phase
    /// defaults. Invalid counts/representation refuse; consumers must require
    /// phase adoption when creating a manual mapping. No IO or fallback occurs.
    pub fn mutation_limits(&self) -> Result<EngineMutationLimits, EngineSettingsError> {
        Ok(EngineMutationLimits {
            task: TaskDefinitionLimits {
                max_attempts: cap(self.task_max_attempts)?,
                max_tool_calls: cap(self.task_max_tool_calls)?,
                max_criteria: size(self.task_max_criteria)?,
                manual_gate_phases: self.task_manual_required_gate_phases.clone(),
            },
            policy: ValidationPolicyLimits {
                allowed_programs: self.validation_allowed_programs.clone(),
                max_controls: size(self.validation_max_controls)?,
                max_arguments: size(self.validation_max_arguments)?,
                max_inputs: size(self.validation_max_inputs)?,
                min_justification_chars: usize::try_from(nonnegative(
                    self.validation_min_justification_chars,
                )?)
                .map_err(|_| refuse("settings-count-platform-representation"))?,
                max_justification_chars: size(self.validation_max_justification_chars)?,
                default_timeout_seconds: nonnegative(self.validation_default_timeout_seconds)?,
                max_timeout_seconds: cap(self.validation_max_timeout_seconds)?,
                max_total_timeout_seconds: cap(self.validation_max_total_timeout_seconds)?,
            },
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSettingsChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: SettingCount,
    pub values: EngineSettingsValues,
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub preview_token: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSettingsEntry {
    pub revision: SettingCount,
    pub actor: String,
    pub at: String,
    pub change: EngineSettingsChange,
    pub previous_digest: String,
    pub digest: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSettingsDocument {
    pub schema_version: i64,
    pub revision: SettingCount,
    pub values: EngineSettingsValues,
    pub history: Vec<EngineSettingsEntry>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EngineSettingsPreview {
    pub current_revision: SettingCount,
    pub proposed_revision: SettingCount,
    pub values: EngineSettingsValues,
    pub preview_token: String,
}
fn hash(value: &impl Serialize) -> Result<String, EngineSettingsError> {
    let b = swarm_wire_json::encode(value).map_err(EngineSettingsError::Format)?;
    Ok(format!("{:x}", Sha256::digest(b)))
}
fn entry_digest(entry: &EngineSettingsEntry) -> Result<String, EngineSettingsError> {
    hash(&(
        &entry.revision,
        &entry.actor,
        &entry.at,
        &entry.change,
        &entry.previous_digest,
    ))
}
/// Validate a supported versioned change envelope before preview or application.
/// Reject invalid event/revision/reason or incomplete policy values, retaining
/// typed validation causes. This reads supplied values only; publication,
/// preview freshness and actor authority remain caller responsibilities.
fn validate_change(r: &EngineSettingsChange) -> Result<(), EngineSettingsError> {
    if !matches!(r.schema_version, 1..=42)
        || r.schema_version != r.values.schema_version()
        || !swarm_identity_contract::name(r.event_id.as_bytes())
        || r.expected_revision.0 < 0
        || crate::text::trim_space(&r.reason).is_empty()
    {
        return Err(refuse("settings-change-schema-event-revision-reason"));
    }
    r.values.validate()
}
impl EngineSettingsDocument {
    pub fn initial(
        values: EngineSettingsValues,
        actor: String,
        at: String,
    ) -> Result<Self, EngineSettingsError> {
        values.validate()?;
        let mut entry = EngineSettingsEntry {
            revision: SettingCount(1),
            actor,
            at,
            change: EngineSettingsChange {
                schema_version: values.schema_version(),
                event_id: "installation-defaults".into(),
                expected_revision: SettingCount(0),
                values: values.clone(),
                reason: "Installation defaults".into(),
                preview_token: String::new(),
            },
            previous_digest: String::new(),
            digest: String::new(),
        };
        entry.digest = entry_digest(&entry)?;
        let doc = Self {
            schema_version: values.schema_version(),
            revision: SettingCount(1),
            values,
            history: vec![entry],
        };
        doc.validate()?;
        Ok(doc)
    }
    pub fn decode(raw: &[u8]) -> Result<Self, EngineSettingsError> {
        let doc: Self = serde_json::from_slice(raw).map_err(EngineSettingsError::Format)?;
        doc.validate()?;
        Ok(doc)
    }
    /// Verify schema1–29 values, complete history, immutable digest chain and
    /// agreement between the head and last entry, preserving historical absence.
    /// Return typed format/policy errors without adoption, publication or IO.
    /// Logical integrity does not establish authentication or rollback protection.
    pub fn validate(&self) -> Result<(), EngineSettingsError> {
        if !matches!(self.schema_version, 1..=42) || self.revision.0 < 1 || self.history.is_empty()
        {
            return Err(refuse("settings-document-envelope"));
        }
        let mut previous = String::new();
        let mut events = BTreeSet::new();
        let mut previous_schema = 1;
        for (index, entry) in self.history.iter().enumerate() {
            validate_change(&entry.change)?;
            if entry.change.schema_version < previous_schema {
                return Err(refuse("settings-schema-downgrade"));
            }
            previous_schema = entry.change.schema_version;
            let ordinal = i64::try_from(index)
                .ok()
                .and_then(|n| n.checked_add(1))
                .ok_or_else(|| refuse("settings-history-representation"))?;
            if entry.revision.0 != ordinal
                || entry.change.expected_revision.0 != entry.revision.0 - 1
                || !events.insert(entry.change.event_id.as_str())
                || crate::text::trim_space(&entry.actor).is_empty()
                || swarm_wire_json::go_rfc3339::parse(&entry.at).is_err()
                || entry.previous_digest != previous
                || entry.digest != entry_digest(entry)?
            {
                return Err(refuse("settings-history-inconsistent"));
            }
            previous = entry.digest.clone();
        }
        let last = self
            .history
            .last()
            .ok_or_else(|| refuse("settings-history-empty"))?;
        if self.revision != last.revision
            || self.values != last.change.values
            || self.schema_version != last.change.schema_version
        {
            return Err(refuse("settings-head-inconsistent"));
        }
        self.values.validate()
    }
    pub fn preview(
        &self,
        r: &EngineSettingsChange,
    ) -> Result<EngineSettingsPreview, EngineSettingsError> {
        self.validate()?;
        validate_change(r)?;
        if r.schema_version < self.schema_version {
            return Err(refuse("settings-schema-downgrade"));
        }
        if r.expected_revision != self.revision {
            return Err(refuse("settings-revision-conflict"));
        }
        let mut normalized = r.clone();
        normalized.preview_token.clear();
        Ok(EngineSettingsPreview {
            current_revision: self.revision,
            proposed_revision: SettingCount(
                self.revision
                    .0
                    .checked_add(1)
                    .ok_or_else(|| refuse("settings-revision-overflow"))?,
            ),
            values: r.values.clone(),
            preview_token: hash(&(self, normalized))?,
        })
    }
    /// Exact replay is checked before observations or stale-revision checks.
    pub fn replay(&self, r: &EngineSettingsChange) -> Result<bool, EngineSettingsError> {
        self.validate()?;
        validate_change(r)?;
        match self
            .history
            .iter()
            .find(|entry| entry.change.event_id == r.event_id)
        {
            Some(entry) if entry.change == *r => Ok(true),
            Some(_) => Err(refuse("settings-event-conflict")),
            None => Ok(false),
        }
    }
    /// Caller must hold the configuration's write reservation through durable
    /// publication; actor/time are attributed data, not authenticated here.
    pub fn apply(
        &self,
        r: &EngineSettingsChange,
        actor: String,
        at: String,
    ) -> Result<(Self, bool), EngineSettingsError> {
        if self.replay(r)? {
            return Ok((self.clone(), true));
        }
        let preview = self.preview(r)?;
        if r.preview_token.is_empty() || preview.preview_token != r.preview_token {
            return Err(refuse("settings-preview-stale-or-missing"));
        }
        let mut entry = EngineSettingsEntry {
            revision: preview.proposed_revision,
            actor,
            at,
            change: r.clone(),
            previous_digest: self
                .history
                .last()
                .ok_or_else(|| refuse("settings-history-empty"))?
                .digest
                .clone(),
            digest: String::new(),
        };
        entry.digest = entry_digest(&entry)?;
        let mut doc = self.clone();
        doc.schema_version = r.schema_version;
        doc.revision = preview.proposed_revision;
        doc.values = r.values.clone();
        doc.history.push(entry);
        doc.validate()?;
        Ok((doc, false))
    }
}

/// Validated saved draft editing policy, independent of execution authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparationWritePolicy { pub enabled: bool, pub document_max_bytes: Option<usize>, pub title_max_bytes: Option<usize>, pub default_method: String }
