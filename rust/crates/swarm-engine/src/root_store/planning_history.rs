//! Recorded planning decisions, never current action/identity/acceptance authority.
use super::{finished, material_path, RootStore, RootStoreError as E};
use std::{error::Error, fmt};

/// Go PlanningDecisionView. Operations preserve nil versus empty and their
/// custom dependency-clear encoding. Recorded values are not validated as new
/// operator policy or executed commands; no private full request is returned.
#[derive(serde::Serialize)]
pub struct PlanningDecisionView {
    pub id: String,
    pub at: String,
    pub scope: String,
    pub reason: String,
    pub operations: Option<Vec<crate::PlanningOperation>>,
}
/// Historical request decoding failed; retain its original codec cause without
/// printing payload, path, actor or treating malformed history as an empty list.
pub struct PlanningHistoryError(crate::WorkError);
impl PlanningHistoryError {
    /// Stable safe family; no permission, mutation or retry follows.
    pub fn code(&self) -> &'static str {
        "planning-history-payload-invalid"
    }
}
impl fmt::Display for PlanningHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for PlanningHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PlanningHistoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}
/// Go json.Unmarshal history semantics, including unknown fields and raw invalid
/// UTF replacement. Full known request types are checked, but no new-command
/// schema/action/budget/lease admission is performed. Missing/empty JSON refuses.
fn payload(raw: &[u8]) -> Result<crate::PlanningRequest, PlanningHistoryError> {
    let text = crate::text::go_utf8(raw);
    crate::work::decode_mode(
        text.as_bytes(),
        crate::planning_request::shape_planning_request(),
        false,
    )
    .map_err(PlanningHistoryError)
}
impl RootStore {
    /// Read every retained planning.decide event in one readonly SQL snapshot
    /// under shared settings reservation and checked physical Root association.
    /// Require an existing Work; unknown scope is not an empty successful lookup.
    /// Go-compatible headers/operations remain data, not identity, current policy,
    /// event-chain validity, completeness or business completion. Memory is
    /// proportional to retained input; no hidden page cap/poll/retry is added.
    /// SQL/payload/finalize/rollback/post-check/release errors retain their causes;
    /// no Work/event/bookmark/provider is changed and no partial list is returned.
    pub fn planning_history(&self, work: &str) -> Result<Vec<PlanningDecisionView>, E> {
        if !swarm_identity_contract::name(work.as_bytes()) {
            return Err(E::Refused("planning-history-work-invalid"));
        }
        material_path::with_material_read(self, || {
            let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = (|| {
                crate::read_recorded_work(&tx, work).map_err(E::Work)?;
                let events = crate::list_recorded_events(&tx, work).map_err(E::Events)?;
                let mut views = Vec::new();
                for event in events {
                    if event.kind != b"planning.decide" {
                        continue;
                    }
                    let data = payload(event.payload.as_deref().unwrap_or_default())
                        .map_err(|e| E::PlanningHistory(Box::new(e)))?;
                    views.push(PlanningDecisionView {
                        id: crate::text::go_utf8(&event.id),
                        at: crate::text::go_utf8(&event.at),
                        scope: data.scope,
                        reason: data.reason,
                        operations: data.operations,
                    });
                }
                Ok(views)
            })();
            finished(result, tx.rollback().map_err(E::Sql))
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_is_data_not_current_command_admission() {
        let r=payload(br#"{"scope":"root","reason":"old","operations":[{"kind":"unknown-history-kind","id":"","title":"","requirements":null,"max_tasks":-1}],"future_field":true}"#).unwrap();
        assert_eq!(r.scope, "root");
        assert_eq!(r.operations.unwrap()[0].max_tasks, -1);
        assert!(payload(br#"{"max_tasks":"invalid-known-type"}"#).is_err());
        assert!(payload(b"").is_err());
    }
    #[test]
    fn nil_empty_casefold_merge_and_invalid_utf_history() {
        assert!(payload(b"null").unwrap().operations.is_none());
        assert_eq!(
            payload(br#"{"Scope":"first","scope":null,"operations":[]}"#)
                .unwrap()
                .scope,
            "first"
        );
        assert_eq!(
            payload(br#"{"operations":[]}"#)
                .unwrap()
                .operations
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            payload(b"{\"reason\":\"\xff\"}").unwrap().reason,
            "\u{fffd}"
        );
    }
}
