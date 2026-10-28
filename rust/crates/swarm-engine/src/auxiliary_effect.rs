//! Trusted observations of auxiliary Root writes, distinct from Work acceptance.
//! Values serialize for readback/diagnostics, never deserialize into authority.
use std::error::Error;

/// Outcome known to the writer; uncertain requires readback, never blind retry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuxiliaryEffectState {
    Recorded,
    Uncertain,
}
/// Go-compatible consultation record. Host/UID identifies the local operator
/// namespace only, never an authenticated human, agent or permission.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SessionVisit {
    pub revision: i64,
    pub at: String,
    pub operator: String,
}
/// Closed set of Root-derived auxiliary observations. No receipt proves a new
/// write, durability, peer receipt, business completion or future permission.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuxiliaryEffect {
    SessionVisit {
        state: AuxiliaryEffectState,
        work_id: String,
        visit: SessionVisit,
    },
}
/// Consolidate exact observation duplicates without upgrading uncertainty or
/// combining different operator/Work/time namespaces. This is data-only merging,
/// never acceptance, a history cap or evidence that an absent effect did not run.
pub fn extend_auxiliary_effects(
    into: &mut Vec<AuxiliaryEffect>,
    values: impl IntoIterator<Item = AuxiliaryEffect>,
) {
    for value in values {
        if !into.contains(&value) {
            into.push(value)
        }
    }
}

/// Collect typed auxiliary effects through the established primary/cleanup/IO
/// graph, including boxed Root errors. No error-message parsing, depth cutoff,
/// mutation or acceptance. Duplicate exact observations are consolidated; known
/// and uncertain observations remain distinct. Empty means no retained witness,
/// never proof that no write occurred. Consumers must also inspect trusted native
/// outcomes and layer-specific effect holders through their adapter accessors;
/// those observations need not themselves be Root errors.
pub fn auxiliary_effects(error: &(dyn Error + 'static)) -> Vec<AuxiliaryEffect> {
    let mut pending = vec![error];
    let mut seen: Vec<*const dyn Error> = vec![];
    let mut result = vec![];
    while let Some(error) = pending.pop() {
        let pointer = error as *const dyn Error;
        if seen.iter().any(|old| std::ptr::eq(*old, pointer)) {
            continue;
        }
        seen.push(pointer);
        if let Some(crate::RootStoreError::AuxiliaryRecorded { effect, .. }) =
            error.downcast_ref::<crate::RootStoreError>()
        {
            if !result.contains(effect.as_ref()) {
                result.push(effect.as_ref().clone())
            }
        }
        #[cfg(target_os = "linux")]
        for (_, cause) in crate::native_error_links(error).children.into_iter().rev() {
            pending.push(cause)
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Native transports are Linux-only. Other Unix callers retain the
            // new Root/IO/standard-source contract without implying bindings.
            if let Some(error) = error.downcast_ref::<Box<crate::RootStoreError>>() {
                pending.push(error.as_ref());
            } else if let Some(crate::RootStoreError::Cleanup { primary, cleanup }) =
                error.downcast_ref::<crate::RootStoreError>()
            {
                pending.push(cleanup.as_ref());
                if let Some(primary) = primary {
                    pending.push(primary.as_ref())
                }
            } else if let Some(error) = error.downcast_ref::<std::io::Error>() {
                if let Some(cause) = error.get_ref() {
                    pending.push(cause)
                }
            } else if let Some(cause) = error.source() {
                pending.push(cause)
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RootStoreError as E;
    /// Fixed observation data only, no Root/SQL/permission side effects.
    fn effect(state: AuxiliaryEffectState) -> AuxiliaryEffect {
        AuxiliaryEffect::SessionVisit {
            state,
            work_id: "PRIVATE_WORK".into(),
            visit: SessionVisit {
                revision: 9,
                at: "2026-01-01T00:00:00Z".into(),
                operator: "PRIVATE_OPERATOR".into(),
            },
        }
    }
    /// Fresh typed error for each established direct/boxed/custom-IO wrapper.
    fn marked(state: AuxiliaryEffectState) -> E {
        E::AuxiliaryRecorded {
            primary: Box::new(E::Refused("fixture-visit-failure")),
            effect: Box::new(effect(state)),
        }
    }
    /// Receipt detection must not depend on source() skipping transparent boxes,
    /// IO custom payloads or secondary cleanup; none is a business publication.
    #[test]
    fn typed_receipts_survive_direct_boxed_io_and_secondary_cleanup() {
        for wrapper in 0..4 {
            let state = AuxiliaryEffectState::Recorded;
            let e: Box<dyn Error> = match wrapper {
                0 => Box::new(marked(state)),
                1 => Box::new(Box::new(marked(state))),
                2 => Box::new(std::io::Error::other(marked(state))),
                _ => Box::new(E::Cleanup {
                    primary: Some(Box::new(E::Refused("fixture-primary"))),
                    cleanup: Box::new(E::Runtime(crate::RuntimeError::new(
                        crate::RuntimeOperation::Output,
                        std::io::Error::other(marked(state)),
                    ))),
                }),
            };
            assert_eq!(
                auxiliary_effects(e.as_ref()),
                vec![effect(state)],
                "wrapper {wrapper}"
            );
        }
    }
    /// Consolidate exact duplicates only; do not upgrade uncertain observations
    /// to known, or turn a missing retained witness into a no-write assertion.
    #[test]
    fn exact_duplicates_only_and_uncertain_remains_distinct() {
        let duplicate = E::Cleanup {
            primary: Some(Box::new(marked(AuxiliaryEffectState::Recorded))),
            cleanup: Box::new(marked(AuxiliaryEffectState::Recorded)),
        };
        assert_eq!(
            auxiliary_effects(&duplicate),
            vec![effect(AuxiliaryEffectState::Recorded)]
        );
        let distinct = E::Cleanup {
            primary: Some(Box::new(marked(AuxiliaryEffectState::Recorded))),
            cleanup: Box::new(marked(AuxiliaryEffectState::Uncertain)),
        };
        assert_eq!(
            auxiliary_effects(&distinct),
            vec![
                effect(AuxiliaryEffectState::Recorded),
                effect(AuxiliaryEffectState::Uncertain)
            ]
        );
        assert!(auxiliary_effects(&E::Refused("fixture-no-witness")).is_empty());
        assert!(!duplicate.to_string().contains("PRIVATE"));
    }
    /// Generic boxed Root traversal must retain the existing Work receipt too;
    /// the new auxiliary case may not hide a regression in earlier domains.
    #[test]
    fn boxed_root_exposes_existing_work_preservation_node() {
        let e = Box::new(E::WorkRecorded {
            primary: Box::new(E::Refused("fixture")),
            work: "fixture-work".into(),
            revision: 7,
        });
        let links = crate::native_error_links(&e);
        assert!(links.children.iter().any(|(_, e)| matches!(
            e.downcast_ref::<E>(),
            Some(E::WorkRecorded { revision: 7, .. })
        )));
    }
}
