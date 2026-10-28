//! Execution-relevant saved policy binding. Audit revision/history remains intact;
//! unrelated Admin edits do not become command authority or a permanent blocker.
use crate::{AgentLaunchError as E, EngineSettingsDocument, EngineSettingsValues};
use serde_json::Value;
use std::collections::BTreeMap;
type Projection = BTreeMap<String, Value>;
/// Select the actual validation-policy, retained-output and process-control
/// consumers. Missing historical fields remain absent; explicit unlimited stays
/// present/null. There is no adoption, normalization or policy default here.
/// The typed field references fail compilation on a rename. Consumer additions
/// still require semantic review and a corresponding binding/test update; this
/// selector does not automatically discover every future execution dependency.
fn projection(values: &EngineSettingsValues) -> Result<Projection, E> {
    let all = serde_json::to_value(values)
        .map_err(|e| E::caused("task-validation-policy-projection", e))?;
    let mut selected = Projection::new();
    macro_rules! fields {($($field:ident),* $(,)?)=>{$(
        let _=&values.$field;
        if let Some(value)=all.get(stringify!($field)){selected.insert(stringify!($field).into(),value.clone());}
    )*};}
    fields!(
        validation_allowed_programs,
        validation_max_controls,
        validation_max_arguments,
        validation_max_inputs,
        validation_min_justification_chars,
        validation_max_justification_chars,
        validation_default_timeout_seconds,
        validation_max_timeout_seconds,
        validation_max_total_timeout_seconds,
        validation_max_output_bytes,
        agent_supervision_poll_interval_ms,
        agent_supervision_term_grace_ms,
        agent_supervision_output_drain_ms
    );
    Ok(selected)
}
/// Count relevant effective changes, preserving withdrawal followed by restore.
/// Unrelated entries preserve generation; metadata alone never refreshes source.
/// Initial absent history uses the supplied current values, never installation
/// defaults. Counter overflow is a representation refusal, not a retry policy.
fn generation<'a>(
    current: &'a EngineSettingsValues,
    history: impl Iterator<Item = &'a EngineSettingsValues>,
) -> Result<(i64, Projection), E> {
    let mut previous = None;
    let mut generation = 0i64;
    for values in history.chain(std::iter::once(current)) {
        let next = projection(values)?;
        if previous.as_ref().is_some_and(|old| old != &next) {
            generation = generation
                .checked_add(1)
                .ok_or(E::Refused("task-validation-policy-generation-overflow"))?
        }
        previous = Some(next);
    }
    Ok((
        generation,
        previous.ok_or(E::Refused("task-validation-policy-projection-missing"))?,
    ))
}
/// Bind current relevant values and their derived generation from the already
/// validated saved document. No IO, mutation, authority, default or arbitrary
/// age limit is introduced. Historical metadata/revision stays separately in
/// audit records; policy withdrawal/restore cannot resurrect an old grant.
pub(super) fn binding(settings: &EngineSettingsDocument) -> Result<(i64, Projection), E> {
    generation(
        &settings.values,
        settings.history.iter().map(|h| &h.change.values),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Explicit data doubles establish projection semantics only; public preview/
    /// apply tests separately establish persistence/history and current readers.
    #[test]
    fn source_policy_ignores_unrelated_changes_but_preserves_relevant_epochs() {
        let base = EngineSettingsValues::installation_defaults().unwrap();
        let initial = generation(&base, std::iter::once(&base)).unwrap();
        let mut unrelated = base.clone();
        unrelated.storage_busy_retry_delay_ms =
            crate::SettingCount(base.storage_busy_retry_delay_ms.0 + 1);
        assert!(projection(&unrelated).unwrap() == projection(&base).unwrap());
        assert!(generation(&base, [&base, &unrelated, &base].into_iter()).unwrap() == initial);
        let mut relevant = base.clone();
        relevant.agent_supervision_poll_interval_ms = Some(crate::SettingCount(
            base.agent_supervision_poll_interval_ms.unwrap().0 + 1,
        ));
        let changed = generation(&relevant, [&base, &relevant].into_iter()).unwrap();
        assert_eq!(changed.0, 1);
        assert!(changed.1 != initial.1);
        let restored = generation(&base, [&base, &relevant, &base].into_iter()).unwrap();
        assert_eq!(restored.0, 2);
        assert!(restored.1 == initial.1);
        let mut absent = base.clone();
        absent.validation_max_output_bytes = None;
        let mut unlimited = base.clone();
        unlimited.validation_max_output_bytes = Some(crate::SettingCap(None));
        assert!(projection(&absent).unwrap() != projection(&unlimited).unwrap());
    }
}
