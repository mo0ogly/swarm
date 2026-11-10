//! Current reported-cost admission, separate from estimated budget reservations.
use crate::{EngineSettingsDocument, MissionSupervisionError as E};
use rusqlite::Connection;

/// Compare finite nonnegative f64 costs against an exact nonnegative integer
/// percentage without multiplying floats, rounding a large integer or overflowing.
/// Binary significands times i64 percent fit u128. Equal thresholds permit.
/// Private callers validate all inputs first; this function performs no IO.
fn exceeds(reported: f64, reserve: f64, percent: i64) -> bool {
    /// Decode a validated nonnegative binary float to scaled integer/exponent.
    fn parts(value: f64, multiplier: u128) -> (u128, i32) {
        let bits = value.to_bits();
        let exp = ((bits >> 52) & 0x7ff) as i32;
        let mantissa = bits & ((1u64 << 52) - 1);
        if exp == 0 {
            (u128::from(mantissa) * multiplier, -1074)
        } else {
            (u128::from(mantissa | (1u64 << 52)) * multiplier, exp - 1075)
        }
    }
    let (a, ae) = parts(reported, 100);
    let (b, be) = parts(reserve, percent as u128);
    if a == 0 || b == 0 {
        return a > b;
    }
    let al = 128 - a.leading_zeros() as i32;
    let bl = 128 - b.leading_zeros() as i32;
    match (ae + al).cmp(&(be + bl)) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            // Equal highest-bit exponents bound the alignment by significand
            // widths; neither shift can exceed the u128 representation.
            if ae >= be {
                (a << (ae - be)) > b
            } else {
                a > (b << (be - ae))
            }
        }
    }
}

/// Enforce current saved per-task threshold in the caller's authoritative SQL
/// admission transaction. Missing historical policy refuses explicit adoption;
/// null disables only this guard. Missing/zero reserve derives no ceiling.
/// Read all retained ownership-validated costs; missing declarations remain
/// unknown and known totals are lower bounds. Errors preserve underlying causes.
/// This writes nothing, refunds nothing and never stops already released workers.
/// Caller must still check mission authority, task eligibility and real budgets.
pub(super) fn admit(
    db: &Connection,
    work: &str,
    task: &str,
    settings: &EngineSettingsDocument,
) -> Result<(), E> {
    let cap = settings
        .values
        .mission_dispatch_cost_reserve_percent
        .ok_or(E::Refused("mission-cost-policy-not-configured"))?
        .0;
    let Some(percent) = cap else {
        return Ok(());
    };
    if percent.0 < 0 {
        return Err(E::Refused("mission-cost-policy-invalid"));
    }
    let budget = crate::financial_budget::read_budget(db, work)
        .map_err(|e| super::mission_supervision_store::caused("mission-cost-budget-read", e))?;
    let Some(budget) = budget else {
        return Ok(());
    };
    if budget.reserve_per_launch_usd.0 == 0.0 {
        return Ok(());
    }
    let costs = super::reported_costs::read(db, work).map_err(|e| E::Root(Box::new(e)))?;
    if let Some(total) = costs.by_task.get(task) {
        if exceeds(
            total.reported_usd.0,
            budget.reserve_per_launch_usd.0,
            percent.0,
        ) {
            return Err(E::Refused("mission-task-reported-cost-threshold"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Exact boundary/adjacent floats, subnormals and large integer percent
    /// distinguish representation constraints from any operator ceiling.
    #[test]
    fn reported_cost_threshold_exact_numeric_boundaries() {
        assert!(!exceeds(2.0, 1.0, 200));
        assert!(exceeds(f64::from_bits(2.0f64.to_bits() + 1), 1.0, 200));
        assert!(!exceeds(f64::from_bits(2.0f64.to_bits() - 1), 1.0, 200));
        let tiny = f64::from_bits(1);
        assert!(!exceeds(tiny, tiny, 100));
        assert!(exceeds(tiny, tiny, 99));
        assert!(!exceeds(f64::MAX, f64::MAX, 100));
        assert!(exceeds(f64::MAX, f64::MAX, 99));
        assert!(!exceeds(0.0, 1.0, 0));
        assert!(exceeds(tiny, 1.0, 0));
        assert!(exceeds(9007199254740992.0, 100.0, 9007199254740991));
        assert!(!exceeds(9007199254740992.0, 100.0, 9007199254740993));
        assert!(!exceeds(f64::MAX, f64::MAX, i64::MAX));
    }
}
