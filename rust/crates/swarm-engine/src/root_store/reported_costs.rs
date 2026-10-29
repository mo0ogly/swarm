//! Complete retained provider-cost observations, never an invoice or budget debit.
use super::{finished, RootStore, RootStoreError as E};
use crate::{Agent, GoFloat};
use serde::Serialize;
use std::collections::BTreeMap;

/// Go-compatible totals. Missing cost is counted separately from reported zero.
/// These provider declarations are neither authenticated charges nor estimates.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CostTotal {
    pub reported_usd: GoFloat,
    pub attempts_with_cost: i64,
    pub attempts_without_cost: i64,
}
impl CostTotal {
    /// Render Go CostTotal.Text using only these already validated observations.
    /// Missing reports remain explicit, including a partial known sum. Two decimal
    /// places and USD are the existing wire presentation, never tariff estimation
    /// or rounded ledger mutation. No IO, reservation or invoice is produced.
    pub fn text(&self) -> String {
        if self.attempts_with_cost == 0 {
            return if self.attempts_without_cost == 0 { "aucune tentative" }
                else { "coût réel non rapporté" }.into();
        }
        let mut text = format!("{:.2} USD rapportés sur {} tentative(s)",
            self.reported_usd.0, self.attempts_with_cost);
        if self.attempts_without_cost > 0 {
            text.push_str(&format!(" · {} sans coût rapporté", self.attempts_without_cost));
        }
        text
    }
    /// Accumulate one retained declaration without rounding, hidden count caps,
    /// price lookup or side effects. Negative/nonfinite inputs, arithmetic and
    /// representation overflow refuse; absence never becomes a reported zero.
    fn add(&mut self, cost: Option<f64>) -> Result<(), E> {
        match cost {
            Some(value) => {
                if !value.is_finite() || value < 0.0 {
                    return Err(E::Refused("reported-cost-invalid"));
                }
                let sum = self.reported_usd.0 + value;
                if !sum.is_finite() {
                    return Err(E::Refused("reported-cost-sum-overflow"));
                }
                let count = self
                    .attempts_with_cost
                    .checked_add(1)
                    .ok_or(E::Refused("reported-cost-count-overflow"))?;
                self.reported_usd = GoFloat(sum);
                self.attempts_with_cost = count;
            }
            None => {
                self.attempts_without_cost = self
                    .attempts_without_cost
                    .checked_add(1)
                    .ok_or(E::Refused("reported-cost-count-overflow"))?;
            }
        }
        Ok(())
    }
}
/// Whole retained agent history in one SQL snapshot. Missing declarations remain
/// explicit in each group; totals are partial whenever attempts_without_cost>0.
/// This excludes planner/reviewer ledgers, reservations and inferred token prices.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CostSummary {
    #[serde(flatten)]
    pub total: CostTotal,
    pub by_task: BTreeMap<String, CostTotal>,
    pub by_provider: BTreeMap<String, CostTotal>,
}
impl CostSummary {
    /// Observe one existing Agent's canonical usage only, never opaque historical
    /// last-valid facts or cumulative session telemetry. On error the unfinished
    /// local projection must be discarded; this does not write any stored state.
    fn add(&mut self, agent: &Agent) -> Result<(), E> {
        let cost = agent
            .usage
            .as_deref()
            .and_then(|u| u.provider_reported_cost_usd.as_deref())
            .map(|v| v.0);
        self.total.add(cost)?;
        self.by_task
            .entry(agent.task_id.clone())
            .or_default()
            .add(cost)?;
        self.by_provider
            .entry(agent.provider.clone())
            .or_default()
            .add(cost)
    }
}
/// Read the complete validated projection inside the caller's SQL transaction.
/// Caller owns snapshot/writer fencing and cleanup; no transaction or mutation is
/// performed here. Unknown work, malformed ownership/costs or SQL errors refuse.
/// This is an observation, never a standalone permission to launch.
pub(crate) fn read(db: &rusqlite::Connection, work: &str) -> Result<CostSummary, E> {
    crate::read_recorded_work(db, work).map_err(E::Work)?;
    let agents = crate::list_agents(db, work).map_err(E::Agent)?;
    let mut summary = CostSummary::default();
    for agent in agents {
        crate::agent_store::verify_agent_ownership(db, &agent).map_err(E::Agent)?;
        summary.add(&agent)?;
    }
    Ok(summary)
}
impl RootStore {
    /// Read reported worker costs for an existing Work in one read transaction.
    /// All retained attempts count, including queued/cancelled/terminal history;
    /// no UI window, model call, tariff fallback, budget change or retry occurs.
    /// Validate every Agent's authoritative ownership columns before grouping.
    /// Unknown Work, malformed/incoherent rows, negative/nonfinite costs, numeric
    /// overflow and SQL/finalization/rollback failures refuse the whole result.
    /// Preserve caller Root close obligations. Stored provider declarations are
    /// observations, not authenticated invoices or a transferable launch permit.
    pub fn reported_costs(&self, work: &str) -> Result<CostSummary, E> {
        let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = read(&tx, work);
        finished(result, tx.rollback().map_err(E::Sql))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Valid witness precedes each invalid numeric input. Accumulation has no
    /// monetary rounding or arbitrary ceiling; failed scalar add is unchanged.
    #[test]
    fn reported_costs_scalar_invalid_overflow_and_zero_remain_distinct() {
        let mut total = CostTotal::default();
        total.add(Some(0.0)).unwrap();
        total.add(None).unwrap();
        assert_eq!(total.attempts_with_cost, 1);
        assert_eq!(total.attempts_without_cost, 1);
        for invalid in [-1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let before = total.clone();
            assert!(total.add(Some(invalid)).is_err());
            assert_eq!(total, before);
        }
        total.add(Some(f64::MAX)).unwrap();
        let before = total.clone();
        assert!(total.add(Some(f64::MAX)).is_err());
        assert_eq!(total, before);
        let mut count = CostTotal {
            attempts_with_cost: i64::MAX,
            ..Default::default()
        };
        let before = count.clone();
        assert!(count.add(Some(1.0)).is_err());
        assert_eq!(count, before);
        count = CostTotal {
            attempts_without_cost: i64::MAX,
            ..Default::default()
        };
        let before = count.clone();
        assert!(count.add(None).is_err());
        assert_eq!(count, before);
    }
}
