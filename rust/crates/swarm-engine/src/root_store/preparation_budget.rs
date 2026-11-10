//! Historical preparation estimates and provider declarations, never invoices.
use super::{budget_history, finished, preparation_read::cause, RootStoreError as E};
use crate::{Budget, GoFloat, Preparation, Usage};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Go-compatible preparation BudgetView. Actual costs remain null until at
/// least one turn declares them; tokens/counts disclose partial coverage. A
/// linked Work has its own shared envelope. This grants no spending authority.
#[derive(Serialize)]
pub struct PreparationBudgetView {
    #[serde(flatten)]
    pub budget: budget_history::HistoricalBudgetView,
    #[serde(rename = "work_budget", skip_serializing_if = "Option::is_none")]
    pub work: Option<budget_history::HistoricalBudgetView>,
    pub turns: i64,
    pub usage_turns: i64,
    pub cost_turns: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}
/// Only consumed turn fields are decoded; other retained turn fields are not
/// interpreted or validated. Row identity/status must agree with this projection.
#[derive(Default, Deserialize)]
#[serde(default)]
struct Turn {
    id: String,
    preparation_id: String,
    status: String,
    usage: Option<Box<Usage>>,
}
/// Require nonnegative finite retained USD values and sums, never coercing
/// corrupt commitments into a zero-cost observation. No tolerance or price lookup.
fn amount(value: f64) -> Result<f64, E> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(E::Refused("preparation-budget-amount-invalid"))
    }
}
/// Add a retained nonnegative count, rejecting representation overflow rather
/// than truncating coverage or applying an operator cutoff.
fn add(total: &mut i64, n: i64) -> Result<(), E> {
    if n < 0 {
        return Err(E::Refused("preparation-budget-usage-invalid"));
    }
    *total = total
        .checked_add(n)
        .ok_or(E::Refused("preparation-budget-count-overflow"))?;
    Ok(())
}
/// Read all retained turns and commitments within the caller SQL snapshot. The
/// saved warning percentage is optional (None disables), never an implicit80%.
/// Amount/token/counter overflow, malformed consumed fields, row mismatches,
/// SQL and explicit statement cleanup errors propagate. No pagination drops
/// historical costs; no provider, reservation or document is changed. This is
/// declared provider coverage only, not authenticated billing or a freshness lease.
pub(super) fn read(
    db: &Connection,
    p: &Preparation,
    warning_percent: Option<i64>,
) -> Result<PreparationBudgetView, E> {
    let budget = p.budget.as_deref().cloned().unwrap_or_else(Budget::default);
    crate::validate_budget(&budget).map_err(|e| cause("preparation-budget-stored-invalid", e))?;
    let mut statement=db.prepare("SELECT amount,state FROM preparation_reservations WHERE preparation_id=?1 AND state IN ('reserved','estimated')").map_err(E::Sql)?;
    let totals = (|| {
        let mut reserved = 0.0;
        let mut estimated = 0.0;
        let mut rows = statement.query([&p.id]).map_err(E::Sql)?;
        while let Some(row) = rows.next().map_err(E::Sql)? {
            let value = amount(row.get::<_, f64>(0).map_err(E::Sql)?)?;
            let state: String = row.get(1).map_err(E::Sql)?;
            match state.as_str() {
                "reserved" => reserved = amount(reserved + value)?,
                "estimated" => estimated = amount(estimated + value)?,
                _ => return Err(E::Refused("preparation-budget-state-invalid")),
            }
        }
        Ok((reserved, estimated))
    })();
    let (reserved, estimated) = finished(totals, statement.finalize().map_err(E::Sql))?;
    let committed = amount(reserved + estimated)?;
    let mut result=PreparationBudgetView { budget:budget_history::HistoricalBudgetView {
        actual_cost_scope:"Somme des coûts rapportés par les échanges de cette préparation ; couverture indiquée par cost_turns et turns.",
        remaining_usd:GoFloat((budget.limit_usd.0-committed).max(0.0)),warning:budget.limit_usd.0>0.0&&warning_percent.is_some_and(|v|committed>=budget.limit_usd.0*(v as f64/100.0)),
        budget,reserved_usd:GoFloat(reserved),committed_estimate_usd:GoFloat(estimated),actual_cost_usd:None,
        policy:"Enveloppe estimative, pas une facture. Réservation avant envoi ; estimation engagée dès qu’un appel peut avoir démarré. Le coût rapporté et les jetons ne couvrent que les échanges qui les fournissent."
    },work:None,turns:0,usage_turns:0,cost_turns:0,input_tokens:0,output_tokens:0};
    let mut statement=db.prepare("SELECT id,status,CAST(body AS BLOB) FROM preparation_turns WHERE preparation_id=?1 ORDER BY id").map_err(E::Sql)?;
    let turns = (|| {
        let mut rows = statement.query([&p.id]).map_err(E::Sql)?;
        while let Some(row) = rows.next().map_err(E::Sql)? {
            use crate::model_wire::Kind;
            let raw: Vec<u8> = row.get(2).map_err(E::Sql)?;
            let turn: Turn = crate::work::decode(
                &raw,
                &[
                    ("id", Kind::String),
                    ("preparation_id", Kind::String),
                    ("status", Kind::String),
                    (
                        "usage",
                        Kind::Pointer(&Kind::Object(crate::models::shape_usage)),
                    ),
                ],
            )
            .map_err(|e| cause("preparation-budget-turn-invalid", e))?;
            if turn.id != row.get::<_, String>(0).map_err(E::Sql)?
                || turn.preparation_id != p.id
                || turn.status != row.get::<_, String>(1).map_err(E::Sql)?
            {
                return Err(E::Refused("preparation-budget-turn-row-mismatch"));
            }
            add(&mut result.turns, 1)?;
            if let Some(usage) = turn.usage {
                add(&mut result.usage_turns, 1)?;
                add(&mut result.input_tokens, usage.input_tokens)?;
                add(&mut result.output_tokens, usage.output_tokens)?;
                if let Some(cost) = usage.provider_reported_cost_usd {
                    let sum = amount(
                        result.budget.actual_cost_usd.as_ref().map_or(0.0, |v| v.0)
                            + amount(cost.0)?,
                    )?;
                    result.budget.actual_cost_usd = Some(GoFloat(sum));
                    add(&mut result.cost_turns, 1)?;
                }
            }
        }
        Ok(())
    })();
    finished(turns, statement.finalize().map_err(E::Sql))?;
    if !p.work_id.is_empty() {
        crate::read_recorded_work(db, &p.work_id).map_err(E::Work)?;
        result.work = Some(budget_history::view(db, &p.work_id, warning_percent)?);
    }
    Ok(result)
}
