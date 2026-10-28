//! Explicit attachment edits inside the shared preparation transaction.
use super::{finished, preparation_capture as capture, RootStore, RootStoreError as E};
use crate::{EngineSettingsValues, Preparation, PreparationCommand};

/// Add/replace a freshly reread source, or remove an exact retained reference.
/// Caller holds the Root/settings reservation and SQL transaction. Removal needs
/// no source policy or filesystem read, so stale/private/over-limit retained data
/// stays removable. Add checks current caps, unique paths and the full-file hash
/// reviewed by the operator. It invalidates brief adoption/verdict, preserving
/// documents/conversion/history. No Work, reservation, agent or provider changes.
pub(super) fn edit(
    root: &RootStore,
    p: &mut Preparation,
    r: &PreparationCommand,
    settings: &EngineSettingsValues,
    at: &str,
) -> Result<(), E> {
    let reference = capture::reference(
        r.source
            .as_ref()
            .ok_or(E::Refused("preparation-request-invalid"))?,
    )?;
    let mut sources = p.sources.clone().unwrap_or_default();
    let index = sources.iter().position(|s| s.path == reference.path);
    if r.action == "source-remove" {
        let index = index.ok_or(E::Refused("preparation-source-absent"))?;
        sources.remove(index);
    } else {
        let policy = settings.preparation_source_policy().map_err(E::Settings)?;
        let mut seen = std::collections::BTreeSet::new();
        if sources.iter().any(|s| !seen.insert(&s.path)) {
            return Err(E::Refused("preparation-source-duplicate-path"));
        }
        let project = capture::project(&root.root, &root.instance()?)?;
        let source = finished(
            capture::excerpt(&project, &reference, &policy, at),
            capture::close(project),
        )?;
        if reference.sha256.is_empty() || source.sha256 != reference.sha256 {
            return Err(E::Refused("preparation-stale-source"));
        }
        if let Some(index) = index {
            sources[index] = source;
        } else {
            sources
                .try_reserve(1)
                .map_err(|e| super::preparation_read::cause("preparation-source-allocation", e))?;
            sources.push(source);
        }
        if policy.max_count.is_some_and(|n| sources.len() > n) {
            return Err(E::Refused("preparation-source-count-over-limit"));
        }
        let total = sources.iter().try_fold(0usize, |n, s| {
            n.checked_add(s.text.len())
                .ok_or(E::Refused("preparation-source-size-overflow"))
        })?;
        if policy.total_max_bytes.is_some_and(|n| total > n) {
            return Err(E::Refused("preparation-source-total-over-limit"));
        }
    }
    p.sources = (!sources.is_empty()).then_some(sources);
    p.adopted_brief = None;
    p.verdict = None;
    p.plan_ready = false;
    Ok(())
}
