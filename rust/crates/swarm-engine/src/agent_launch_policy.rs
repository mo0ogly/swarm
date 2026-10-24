//! Saved launch preparation choices; no runtime permission is represented.
/// Validated v16 policy constructed from settings, not a caller's claim of rights.
#[derive(Clone, Debug)]
pub struct AgentLaunchPolicy {
    pub(crate) allow_unverified: bool,
    pub(crate) timeout: i64,
    pub(crate) timeout_cap: Option<i64>,
    pub(crate) ttl: i64,
    pub(crate) ttl_cap: Option<i64>,
    pub(crate) instruction_bytes: Option<usize>,
    pub(crate) provider_bytes: Option<usize>,
    pub(crate) context_bytes: Option<usize>,
    pub(crate) minimum_free_bytes: u64,
    pub(crate) environment: Vec<String>,
}
/// POSIX environment-name syntax is a representation invariant, not an allowlist.
/// Permission comes from saved names; empty/assignment/NUL/duplicates are refused.
pub(crate) fn environment_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
