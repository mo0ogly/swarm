//! Saved native HTTP policy. No listener, provider call, defaults adoption or live reload.
use crate::{EngineSettingsError as E, EngineSettingsValues, SettingCap, SettingCount};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};
/// v20 members; metadata/tests check exact agreement with the serialized defaults.
/// Presence is version information, never permission or implicit adoption.
pub const WEB_POLICY_FIELDS: [&str; 11] = [
    "web_bind_address",
    "web_http_header_max_bytes",
    "web_http_max_headers",
    "web_http_header_timeout_ms",
    "web_http_body_max_bytes",
    "web_http_body_timeout_ms",
    "web_http_max_connections",
    "web_root_worker_threads",
    "web_admin_worker_threads",
    "web_worker_idle_keep_alive_ms",
    "web_cookie_max_age_seconds",
];
/// Read-only representation constraints for the pinned HTTP libraries. These are
/// not operator defaults: HeaderMap1.5 uses raw32768 slots at3/4 load; Hyper1.12
/// requires an8192byte minimum buffer ceiling; Tokio1.54 permits usize::MAX>>3 slots.
/// The web crate must probe these against actual dependencies before delivery.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct WebTransportConstraints {
    pub max_header_fields: SettingCount,
    pub minimum_connection_buffer_bytes: SettingCount,
    pub max_header_bytes: SettingCount,
    pub native_body_max_bytes: SettingCount,
    pub max_semaphore_permits: SettingCount,
}
impl WebTransportConstraints {
    /// Current build's technical limits, not actual allocation; platform conversion is separate from
    /// runtime resource availability. This allocates no parser, socket or worker.
    pub fn current() -> Self {
        Self {
            max_header_fields: SettingCount(24_576),
            minimum_connection_buffer_bytes: SettingCount(8192),
            max_header_bytes: SettingCount(isize::MAX as i64),
            native_body_max_bytes: SettingCount(
                crate::command_input::COMMAND_INPUT_MAX_BYTES as i64,
            ),
            max_semaphore_permits: SettingCount((usize::MAX >> 3) as i64),
        }
    }
    /// Maximum representable value where one exists beyond canonical i64 input.
    /// Worker maximum assumes at least one worker in the other lane; their sum
    /// is also validated. Metadata never replaces authoritative value validation.
    pub(crate) fn maximum_for(&self, name: &str) -> Option<SettingCount> {
        match name {
            "web_http_max_headers" => Some(self.max_header_fields),
            "web_http_header_max_bytes" => Some(self.max_header_bytes),
            "web_http_body_max_bytes" => Some(self.native_body_max_bytes),
            "web_http_max_connections" => Some(self.max_semaphore_permits),
            "web_root_worker_threads" | "web_admin_worker_threads" => {
                Some(SettingCount(self.max_semaphore_permits.0 - 1))
            }
            _ => None,
        }
    }
}
/// Validated configured policy, not an observation of an active server. Values
/// take effect on server restart. Absence of a cap disables operator policy only;
/// native framing/OS/library representation limits remain disclosed separately.
/// No credentials, authentication or authority are created by this DTO.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebTransportPolicy {
    pub bind_address: SocketAddr,
    pub header_max_bytes: usize,
    pub max_headers: usize,
    /// Includes waiting between keep-alive requests, unlike Go's separate idle deadline.
    pub header_timeout: Option<Duration>,
    pub body_max_bytes: Option<usize>,
    /// Starts after headers; it never authorizes cancelling a Root commit.
    pub body_timeout: Option<Duration>,
    /// Some(0) disables network startup; None has no operator connection ceiling.
    pub max_connections: Option<usize>,
    pub root_worker_threads: usize,
    pub admin_worker_threads: usize,
    /// Zero disables retention of idle worker threads, not execution itself.
    pub worker_idle_keep_alive: Duration,
    /// None requests a browser-session cookie, not an infinite bearer lifetime.
    /// This does not expire/revoke the persisted server-side capability file.
    pub cookie_max_age_seconds: Option<i64>,
}
/// Detect any supplied v20 member; partial groups cannot masquerade as legacy.
/// No IO, fallback/default or policy application occurs.
pub(crate) fn supplied(v: &EngineSettingsValues) -> bool {
    v.web_bind_address.is_some()
        || v.web_http_header_max_bytes.is_some()
        || v.web_http_max_headers.is_some()
        || v.web_http_header_timeout_ms.is_some()
        || v.web_http_body_max_bytes.is_some()
        || v.web_http_body_timeout_ms.is_some()
        || v.web_http_max_connections.is_some()
        || v.web_root_worker_threads.is_some()
        || v.web_admin_worker_threads.is_some()
        || v.web_worker_idle_keep_alive_ms.is_some()
        || v.web_cookie_max_age_seconds.is_some()
}
/// Require a supplied member without treating historical absence as null/default.
fn required<T>(value: Option<T>) -> Result<T, E> {
    value.ok_or(E::Refused("settings-web-policy-incomplete"))
}
/// Preserve explicit null caps; canonical decoding and conversion reject negatives.
fn cap(value: Option<SettingCap>) -> Result<Option<i64>, E> {
    Ok(required(value)?.0.map(|n| n.0))
}
/// Convert a platform count with a typed original cause, not truncation/clamping.
fn size(value: i64) -> Result<usize, E> {
    usize::try_from(value).map_err(E::WebCount)
}
/// Positive timeout or explicit null. Zero is not a hidden unlimited convention.
fn timeout(value: Option<SettingCap>, reason: &'static str) -> Result<Option<Duration>, E> {
    cap(value)?
        .map(|n| {
            if n <= 0 {
                return Err(E::Refused(reason));
            }
            Ok(Duration::from_millis(
                u64::try_from(n).map_err(E::WebCount)?,
            ))
        })
        .transpose()
}
/// Build the complete network group from saved values. Other group/authority
/// preconditions belong to the validated containing document. No OS resources or
/// default values are created, and configured is not the active runtime state.
pub(crate) fn build(v: &EngineSettingsValues) -> Result<WebTransportPolicy, E> {
    if !supplied(v) {
        return Err(E::Refused("settings-web-policy-not-configured"));
    }
    let raw = v
        .web_bind_address
        .as_deref()
        .ok_or(E::Refused("settings-web-policy-incomplete"))?;
    let bind_address: SocketAddr = raw.parse().map_err(E::WebAddress)?;
    if !matches!(bind_address.ip(),IpAddr::V4(ip) if ip==Ipv4Addr::LOCALHOST)
        && !matches!(bind_address.ip(),IpAddr::V6(ip) if ip==Ipv6Addr::LOCALHOST)
    {
        return Err(E::Refused("settings-web-loopback-required"));
    }
    if bind_address.to_string() != raw {
        return Err(E::Refused("settings-web-address-canonical"));
    }
    let limits = WebTransportConstraints::current();
    let header_max_bytes = size(required(v.web_http_header_max_bytes)?.0)?;
    if header_max_bytes == 0 || header_max_bytes > size(limits.max_header_bytes.0)? {
        return Err(E::Refused("settings-web-header-bytes-transport"));
    }
    let max_headers = size(required(v.web_http_max_headers)?.0)?;
    if max_headers == 0 || max_headers > size(limits.max_header_fields.0)? {
        return Err(E::Refused("settings-web-header-count-transport"));
    }
    let body_max_bytes = cap(v.web_http_body_max_bytes)?.map(size).transpose()?;
    if body_max_bytes.is_some_and(|n| n > crate::command_input::COMMAND_INPUT_MAX_BYTES) {
        return Err(E::Refused("settings-web-body-transport"));
    }
    let max_connections = cap(v.web_http_max_connections)?.map(size).transpose()?;
    let max_permits = size(limits.max_semaphore_permits.0)?;
    if max_connections.is_some_and(|n| n > max_permits) {
        return Err(E::Refused("settings-web-connections-transport"));
    }
    let root_worker_threads = size(required(v.web_root_worker_threads)?.0)?;
    let admin_worker_threads = size(required(v.web_admin_worker_threads)?.0)?;
    if root_worker_threads == 0
        || admin_worker_threads == 0
        || root_worker_threads
            .checked_add(admin_worker_threads)
            .is_none_or(|n| n > max_permits)
    {
        return Err(E::Refused("settings-web-workers-transport"));
    }
    let worker_idle_keep_alive = Duration::from_millis(
        u64::try_from(required(v.web_worker_idle_keep_alive_ms)?.0).map_err(E::WebCount)?,
    );
    let cookie_max_age_seconds = cap(v.web_cookie_max_age_seconds)?;
    if cookie_max_age_seconds.is_some_and(|n| n <= 0) {
        return Err(E::Refused("settings-web-cookie-age-positive-or-null"));
    }
    Ok(WebTransportPolicy {
        bind_address,
        header_max_bytes,
        max_headers,
        header_timeout: timeout(
            v.web_http_header_timeout_ms,
            "settings-web-header-timeout-positive-or-null",
        )?,
        body_max_bytes,
        body_timeout: timeout(
            v.web_http_body_timeout_ms,
            "settings-web-body-timeout-positive-or-null",
        )?,
        max_connections,
        root_worker_threads,
        admin_worker_threads,
        worker_idle_keep_alive,
        cookie_max_age_seconds,
    })
}
