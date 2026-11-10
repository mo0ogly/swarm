//! Typed manual gate-default vocabulary. Selecting phases is persisted policy;
//! the wire phase names themselves are protocol, never permission to run checks.
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

/// Supported manual phase identifiers, not a selection of mandatory phases.
pub const MANUAL_GATE_PHASES: [&str; 3] = ["entry", "validation", "delivery"];

/// Explicit default phases for new manual task contracts. Empty means no extra
/// phase checks; declared criterion checks remain mandatory. Absent configuration
/// is represented separately by Option at the settings boundary, never by empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ManualGatePhaseDefaults(BTreeSet<String>);
impl ManualGatePhaseDefaults {
    /// Validate an explicit set against the phase wire vocabulary. Returns a
    /// canonical ordered set or a static invalid-phase reason; empty is valid.
    /// Performs no IO, mutation of existing tasks or authorization of any actor.
    pub fn new(phases: BTreeSet<String>) -> Result<Self, &'static str> {
        if phases
            .iter()
            .any(|p| !MANUAL_GATE_PHASES.contains(&p.as_str()))
        {
            return Err("task-gate-phase-default-invalid");
        }
        Ok(Self(phases))
    }
    /// Borrow selected phase names in canonical order. No IO or errors; this
    /// selection supplies future defaults and cannot remove adopted requirements.
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }
    /// Historical Go-reference fixture only; native Root uses persisted settings.
    /// This immutable compatibility profile neither reads nor overrides policy.
    pub(crate) fn legacy_go() -> Self {
        Self(MANUAL_GATE_PHASES.into_iter().map(str::to_owned).collect())
    }
}
impl<'de> Deserialize<'de> for ManualGatePhaseDefaults {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let phases = BTreeSet::<String>::deserialize(deserializer)?;
        Self::new(phases).map_err(serde::de::Error::custom)
    }
}
