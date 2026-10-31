//! Bell-LaPadula data-flow contracts. No deserialized label, principal name or
//! successful check establishes authentication, provenance or OS confinement.
//! Producers must bind actual source bytes and current authority; sinks must
//! revalidate under their engagement reservation. Integrity is a separate check.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, error::Error, fmt};

/// Content-free failure. Diagnostics intentionally disclose neither labels,
/// principals, paths, destinations nor the existence of an inaccessible object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfidentialityError {
    InvalidPolicy,
    InvalidClassification,
    Unclassified,
    AccessDenied,
    EmptyContext,
    PolicyChanged,
}
impl ConfidentialityError {
    /// Stable diagnostic code; no input bytes or grant inventory is rendered.
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidPolicy => "blp-policy-invalid",
            Self::InvalidClassification => "blp-classification-invalid",
            Self::Unclassified => "blp-classification-required",
            Self::AccessDenied => "blp-access-denied",
            Self::EmptyContext => "blp-context-required",
            Self::PolicyChanged => "blp-policy-changed",
        }
    }
}
impl fmt::Display for ConfidentialityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.code()) }
}
impl Error for ConfidentialityError {}

/// Proposed or attributed classification. Construction/deserialization is not
/// attribution: the protected producer owes provenance and current validation.
/// Compartments are a set semantically; duplicates are refused, never collapsed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Classification {
    pub level: String,
    pub compartments: Vec<String>,
}

/// Persisted presentation of one level. Vector order defines ascending
/// sensitivity, independently from Biba integrity or a user's role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Level {
    pub id: String,
    pub name_fr: String,
    pub name_en: String,
}

/// Confidentiality policy data, not an effective runtime protection. Empty
/// grants authorize nobody. There is no permissive missing-label fallback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema_version: u32,
    pub levels: Vec<Level>,
    pub compartments: Vec<String>,
    pub clearances: BTreeMap<String, Classification>,
}

/// Technical identifier grammar, not an operator limit on levels or compartments.
fn identifier(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}

/// Test for duplicate identifiers without discarding the original representation.
fn unique<'a>(mut items: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    items.all(|s| identifier(s) && seen.insert(s))
}

impl Policy {
    /// Validate all declared levels, compartments and clearances. No IO,
    /// authentication, mutation, implicit catalog or principal is introduced.
    pub fn validate(&self) -> Result<(), ConfidentialityError> {
        if self.schema_version != 1 || self.levels.is_empty()
            || !unique(self.levels.iter().map(|l| l.id.as_str()))
            || !unique(self.compartments.iter().map(String::as_str))
            || self.levels.iter().any(|l| [&l.name_fr, &l.name_en].iter().any(|s|
                s.trim().is_empty() || s.trim() != s.as_str() || s.chars().any(char::is_control)))
        { return Err(ConfidentialityError::InvalidPolicy); }
        for (principal, label) in &self.clearances {
            if !identifier(principal) { return Err(ConfidentialityError::InvalidPolicy); }
            self.rank(label).map_err(|_| ConfidentialityError::InvalidPolicy)?;
        }
        Ok(())
    }

    /// Validate a classification and return its catalog ordinal. This helper
    /// never turns an unknown level or compartment into the public level.
    fn rank(&self, label: &Classification) -> Result<usize, ConfidentialityError> {
        let rank = self.levels.iter().position(|l| l.id == label.level)
            .ok_or(ConfidentialityError::InvalidClassification)?;
        if !unique(label.compartments.iter().map(String::as_str))
            || label.compartments.iter().any(|c| !self.compartments.contains(c))
        { return Err(ConfidentialityError::InvalidClassification); }
        Ok(rank)
    }

    /// Check a proposed catalog transition. Existing level order and compartment
    /// identifiers cannot be removed/reordered: that would relabel history.
    /// Appending levels/compartments, renaming display text and changing grants
    /// are explicit policy updates; callers own revision checks and persistence.
    pub fn validate_successor(&self, next: &Self) -> Result<(), ConfidentialityError> {
        self.validate()?; next.validate()?;
        if next.levels.len() < self.levels.len()
            || !self.levels.iter().zip(&next.levels).all(|(a,b)| a.id == b.id)
            || self.compartments.iter().any(|c| !next.compartments.contains(c))
        { return Err(ConfidentialityError::InvalidPolicy); }
        Ok(())
    }

    /// True iff receiver dominates source (level >= and compartments superset).
    /// Both labels and policy are validated; this is a data-only computation,
    /// not a reusable authorization or evidence of actual content provenance.
    pub fn dominates(&self, receiver: &Classification, source: &Classification)
        -> Result<bool, ConfidentialityError>
    {
        self.validate()?;
        Ok(self.rank(receiver)? >= self.rank(source)?
            && source.compartments.iter().all(|c| receiver.compartments.contains(c)))
    }

    /// Read permission for a currently authenticated principal name. The caller
    /// owes that authentication and actual object attribution; missing labels
    /// and missing clearances refuse. No principal/role is inferred from data.
    pub fn check_read(&self, principal: &str, object: Option<&Classification>)
        -> Result<(), ConfidentialityError>
    {
        self.validate()?;
        // Do not reveal object classification status to an unrecognized reader.
        let clearance = self.clearances.get(principal).ok_or(ConfidentialityError::AccessDenied)?;
        let object = object.ok_or(ConfidentialityError::Unclassified)?;
        if self.dominates(clearance, object)? { Ok(()) } else { Err(ConfidentialityError::AccessDenied) }
    }

    /// Join actual source classifications conservatively. Unknown inputs and
    /// an empty context refuse; the engine must supply its explicitly classified
    /// instructions even for a request without user documents. Returns sorted
    /// compartments, never a declassification or a new integrity authority.
    pub fn join(&self, sources: &[Option<Classification>])
        -> Result<Classification, ConfidentialityError>
    {
        self.validate()?;
        if sources.is_empty() { return Err(ConfidentialityError::EmptyContext); }
        let mut rank = 0;
        let mut compartments = BTreeSet::new();
        for source in sources {
            let source = source.as_ref().ok_or(ConfidentialityError::Unclassified)?;
            rank = rank.max(self.rank(source)?);
            compartments.extend(source.compartments.iter().cloned());
        }
        Ok(Classification { level: self.levels[rank].id.clone(), compartments: compartments.into_iter().collect() })
    }
}

/// Fixed exposure domain. Its private fields prevent relabeling after reading
/// a secret. This value records data-only checks, not an authenticated process,
/// confinement, source provenance or a sink engagement capability.
#[derive(Clone, Debug)]
pub struct ExecutionDomain {
    label: Classification,
    policy: Policy,
    principal: String,
    revision: u64,
}
impl ExecutionDomain {
    /// Establish a fixed domain from every exposed source and the current
    /// principal's clearance. Inputs must come from protected producers. An
    /// unclassified, empty or unauthorized context refuses without side effects.
    pub fn prepare(policy: &Policy, revision: u64, principal: &str, sources: &[Option<Classification>])
        -> Result<Self, ConfidentialityError>
    {
        if revision == 0 { return Err(ConfidentialityError::InvalidPolicy); }
        let label = policy.join(sources)?;
        policy.check_read(principal, Some(&label))?;
        Ok(Self { label, policy: policy.clone(), principal: principal.into(), revision })
    }

    /// Classification of every output, including summaries, errors and empty
    /// results. Borrowing this label does not grant access to source contents.
    pub fn output_classification(&self) -> &Classification { &self.label }

    /// Check a subsequent read without raising or lowering this domain. Sources
    /// outside it require a new execution; maximum clearance alone is insufficient.
    pub fn check_input(&self, source: Option<&Classification>) -> Result<(), ConfidentialityError> {
        let source = source.ok_or(ConfidentialityError::Unclassified)?;
        if self.policy.dominates(&self.label, source)? { Ok(()) } else { Err(ConfidentialityError::AccessDenied) }
    }

    /// Check ordinary write/transmission to an attributed destination label
    /// using freshly loaded policy data. No write-down and no compartment loss.
    /// A changed policy refuses even if the new clearance is broader; rebuild
    /// and reauthorize in a fresh execution. Caller still owns engagement locking,
    /// real destination binding, identity, Biba and transport confinement.
    pub fn check_output(&self, current: &Policy, revision: u64, destination: Option<&Classification>)
        -> Result<(), ConfidentialityError>
    {
        current.validate()?;
        if revision != self.revision || current != &self.policy { return Err(ConfidentialityError::PolicyChanged); }
        current.check_read(&self.principal, Some(&self.label))?;
        let destination = destination.ok_or(ConfidentialityError::Unclassified)?;
        if current.dominates(destination, &self.label)? { Ok(()) } else { Err(ConfidentialityError::AccessDenied) }
    }
}

#[cfg(test)]
mod tests;
