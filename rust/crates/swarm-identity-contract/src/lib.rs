//! Native port of Go's `internal/identitycontract` data validation.
//!
//! Valid data does not authenticate a principal or grant authority. There is no
//! wire codec, journal, OS boundary or public Swarm command in this crate yet.
#![forbid(unsafe_code)]

/// Whether a raw Linux UID has an assignable numeric representation. The all-ones
/// UID (-1) is reserved by Linux; zero remains representable. This pure check does
/// no account lookup, namespace mapping, IO, authentication or authorization and
/// is not an operator policy ceiling. A true result is data only, never a grant.
pub fn linux_uid(value: u32) -> bool {
    value != u32::MAX
}

/// Existing identifier representation: 1..=100 ASCII bytes, not an execution policy.
pub fn name(value: &[u8]) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Opaque, nonempty UTF-8 without controls or leading/trailing Unicode whitespace.
pub fn text(value: &[u8]) -> bool {
    let Ok(value) = std::str::from_utf8(value) else {
        return false;
    };
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

/// A 32-byte digest's exact lowercase hexadecimal representation.
pub fn digest(value: &[u8]) -> bool {
    value.len() == 64
        && value
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

/// Unauthenticated principal data. `valid` is structural validation only.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Principal {
    #[cfg_attr(feature = "serde", serde(rename = "principal_id"))]
    pub id: String,
    #[cfg_attr(feature = "serde", serde(rename = "principal_kind"))]
    pub kind: String,
    pub issuer: String,
    pub subject: String,
}

impl Principal {
    pub fn valid(&self) -> bool {
        if !name(self.id.as_bytes())
            || !text(self.issuer.as_bytes())
            || !text(self.subject.as_bytes())
        {
            return false;
        }
        match self.kind.as_str() {
            "local" => self.issuer == "urn:swarm:local" && self.subject == self.id,
            "human" | "service" => self.issuer != "urn:swarm:local",
            _ => false,
        }
    }
}

/// Plan scope. i64 matches Go's int on the observed Linux/amd64 reference.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Scope {
    pub work_id: String,
    pub plan_version: i64,
    pub plan_digest: String,
}

impl Scope {
    pub fn valid(&self) -> bool {
        name(self.work_id.as_bytes())
            && self.plan_version > 0
            && digest(self.plan_digest.as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Permission {
    pub operation: String,
    pub resource: String,
}

impl Permission {
    pub fn valid(&self) -> bool {
        matches!(
            self.operation.as_str(),
            "agent.create"
                | "memory.read"
                | "memory.summarize"
                | "memory.transmit"
                | "tool.invoke"
                | "effect.execute"
        ) && text(self.resource.as_bytes())
            && !self
                .resource
                .bytes()
                .any(|byte| matches!(byte, b'*' | b'?' | b'[' | b']'))
    }
}

#[cfg(test)]
mod linux_uid_tests {
    /// Representation only: root and the maximum assignable value are not grants
    /// or existing account claims; the all-ones sentinel is the sole exclusion.
    #[test]
    fn linux_uid_representation_refuses_only_reserved_sentinel() {
        for uid in [0, 1, u32::MAX - 1] {
            assert!(super::linux_uid(uid));
        }
        assert!(!super::linux_uid(u32::MAX));
    }
}
