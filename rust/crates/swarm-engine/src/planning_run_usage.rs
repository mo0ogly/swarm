//! Normalized unverified provider declarations, separate from held estimates.
use crate::{agent_launch::query, AgentLaunchError as E};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Native receipt data with explicit unknown values; no serde default or JSON
/// blob can supply operational tokens/cost. All reported amounts remain provider
/// declarations, never invoices or permission to release a reserved estimate.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UsageRecord {
    pub claim_id: String,
    pub quality: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
    pub cached_input_tokens: Option<i64>,
    pub provider_reported_cost_usd: Option<f64>,
    pub source: Option<String>,
    pub at: Option<String>,
    pub reported_model: Option<String>,
    pub model_source: Option<String>,
    pub model_at: Option<String>,
}
const TABLE:&str="CREATE TABLE native_planning_usage(claim_id TEXT PRIMARY KEY REFERENCES native_planning_runs(claim_id),quality TEXT NOT NULL CHECK(quality IN ('not_reported','observed','invalid','incomplete')),input_tokens INTEGER,output_tokens INTEGER,cache_read_input_tokens INTEGER,cache_creation_input_tokens INTEGER,cached_input_tokens INTEGER,provider_reported_cost_usd REAL,source TEXT,at TEXT,reported_model TEXT,model_source TEXT,model_at TEXT)";

/// Observe/create this exact auxiliary schema in the owning transaction only.
/// Refuse replacement tables/triggers/indexes; absence under read is not usage0.
fn schema(db: &Connection, create: bool) -> Result<bool, E> {
    let entry: Option<(String, String)> = query(
        db,
        "SELECT type,sql FROM sqlite_schema WHERE name='native_planning_usage'",
        [],
        |rows| {
            Ok(rows
                .next()?
                .map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?)))
                .transpose()?)
        },
    )?;
    match entry {
        None if !create => return Ok(false),
        None => {
            if db.is_autocommit() {
                return Err(E::Refused("planning-usage-transaction-required"));
            }
            db.execute_batch(TABLE)?;
        }
        Some((kind, sql)) if kind == "table" && sql == TABLE => {}
        _ => return Err(E::Refused("planning-usage-schema-invalid")),
    }
    let extras:i64=query(db,"SELECT count(*) FROM sqlite_schema WHERE tbl_name='native_planning_usage' AND type!='table' AND substr(name,1,7)!='sqlite_'",[],|r|Ok(r.next()?.ok_or(E::Refused("planning-usage-schema-observation"))?.get(0)?))?;
    if extras != 0 {
        return Err(E::Refused("planning-usage-schema-invalid"));
    }
    Ok(true)
}
/// Initialize the exact empty normalized schema before provider execution.
/// Requires the owning transaction; no token, cost or estimate is fabricated.
pub(crate) fn initialize(db: &Connection) -> Result<(), E> {
    schema(db, true).map(|_| ())
}
impl UsageRecord {
    /// Freeze current typed facts only. Invalid/incomplete/absent usage never uses
    /// last-valid history as canonical0; model remains a separate unverified channel.
    pub(crate) fn observe(
        claim: &str,
        f: &crate::agent_provider_facts::ProviderFactsSnapshot,
    ) -> Result<Self, E> {
        use crate::agent_provider_facts::FactState;
        let quality = match f.usage_state {
            FactState::NotReported => "not_reported",
            FactState::Observed => "observed",
            FactState::Invalid => "invalid",
            FactState::Incomplete => "incomplete",
        };
        let u = f.usage.as_ref();
        let m = f.reported_model.as_ref();
        let result = Self {
            claim_id: claim.into(),
            quality: quality.into(),
            input_tokens: u.map(|u| u.input_tokens),
            output_tokens: u.map(|u| u.output_tokens),
            cache_read_input_tokens: u.and_then(|u| u.cache_read_input_tokens.as_deref().copied()),
            cache_creation_input_tokens: u
                .and_then(|u| u.cache_creation_input_tokens.as_deref().copied()),
            cached_input_tokens: u.and_then(|u| u.cached_input_tokens.as_deref().copied()),
            provider_reported_cost_usd: u
                .and_then(|u| u.provider_reported_cost_usd.as_deref().map(|v| v.0)),
            source: u.map(|u| u.source.clone()),
            at: u.map(|u| u.at.clone()),
            reported_model: m.map(|m| m.model.clone()),
            model_source: m.map(|m| m.source.clone()),
            model_at: m.map(|m| m.at.clone()),
        };
        result.validate()?;
        Ok(result)
    }
    /// Reject incomplete canonical tuples and invalid scalars without IO/defaults.
    /// Unknown remains NULL; an observed zero remains zero; signed counters/cost must be nonnegative and finite.
    pub(crate) fn validate(&self) -> Result<(), E> {
        if !swarm_identity_contract::name(self.claim_id.as_bytes())
            || !matches!(
                self.quality.as_str(),
                "not_reported" | "observed" | "invalid" | "incomplete"
            )
        {
            return Err(E::Refused("planning-usage-record-invalid"));
        }
        let observed = self.quality == "observed";
        if observed != self.input_tokens.is_some()
            || observed != self.output_tokens.is_some()
            || observed != self.source.is_some()
            || observed != self.at.is_some()
            || !observed
                && (self.cache_read_input_tokens.is_some()
                    || self.cache_creation_input_tokens.is_some()
                    || self.cached_input_tokens.is_some()
                    || self.provider_reported_cost_usd.is_some())
            || [
                self.input_tokens,
                self.output_tokens,
                self.cache_read_input_tokens,
                self.cache_creation_input_tokens,
                self.cached_input_tokens,
            ]
            .into_iter()
            .flatten()
            .any(|n| n < 0)
            || self
                .provider_reported_cost_usd
                .is_some_and(|n| !n.is_finite() || n < 0.)
            || self.reported_model.is_some() != self.model_source.is_some()
            || self.reported_model.is_some() != self.model_at.is_some()
            || [&self.source, &self.reported_model, &self.model_source]
                .into_iter()
                .flatten()
                .any(|s| {
                    !crate::text::nonempty(s) || crate::agent_terminal_text::terminal_text(s) != *s
                })
        {
            return Err(E::Refused("planning-usage-record-invalid"));
        }
        for at in [&self.at, &self.model_at].into_iter().flatten() {
            swarm_wire_json::go_rfc3339::parse(at)
                .map_err(|e| E::caused("planning-usage-clock", e))?;
        }
        Ok(())
    }
}
/// Read required scalar row after schema/type validation, preserving SQL causes.
/// Absence refuses, never a successful unknown/zero replacement measurement.
pub(crate) fn load(db: &Connection, claim: &str) -> Result<UsageRecord, E> {
    if !schema(db, false)? {
        return Err(E::Refused("planning-usage-required"));
    }
    let row=query(db,"SELECT quality,input_tokens,output_tokens,cache_read_input_tokens,cache_creation_input_tokens,cached_input_tokens,provider_reported_cost_usd,source,at,reported_model,model_source,model_at FROM native_planning_usage WHERE claim_id=?",[claim],|rows|{
  let r=rows.next()?.ok_or(E::Refused("planning-usage-required"))?;
  Ok(UsageRecord{claim_id:claim.into(),quality:r.get(0)?,input_tokens:r.get(1)?,output_tokens:r.get(2)?,cache_read_input_tokens:r.get(3)?,cache_creation_input_tokens:r.get(4)?,cached_input_tokens:r.get(5)?,provider_reported_cost_usd:r.get(6)?,source:r.get(7)?,at:r.get(8)?,reported_model:r.get(9)?,model_source:r.get(10)?,model_at:r.get(11)?})
 })?;
    row.validate()?;
    Ok(row)
}
/// Stage immutable scalar usage in the same caller transaction as run/decision.
/// An exact repeated row is harmless; conflicting observations refuse. Readback
/// catches ignored writes. No budget ledger, estimate, history or invoice changes.
pub(crate) fn record(db: &Connection, row: &UsageRecord) -> Result<(), E> {
    if db.is_autocommit() {
        return Err(E::Refused("planning-usage-transaction-required"));
    }
    row.validate()?;
    schema(db, true)?;
    let existing: i64 = query(
        db,
        "SELECT count(*) FROM native_planning_usage WHERE claim_id=?",
        [&row.claim_id],
        |r| {
            Ok(r.next()?
                .ok_or(E::Refused("planning-usage-observation"))?
                .get(0)?)
        },
    )?;
    if existing == 0
        && db.execute(
            "INSERT INTO native_planning_usage VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                row.claim_id,
                row.quality,
                row.input_tokens,
                row.output_tokens,
                row.cache_read_input_tokens,
                row.cache_creation_input_tokens,
                row.cached_input_tokens,
                row.provider_reported_cost_usd,
                row.source,
                row.at,
                row.reported_model,
                row.model_source,
                row.model_at
            ],
        )? != 1
    {
        return Err(E::Refused("planning-usage-not-recorded"));
    }
    if load(db, &row.claim_id)? != *row {
        return Err(E::Refused("planning-usage-conflict"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Isolated scalar-FK fixture only, not a genuine registered process/authority.
    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE native_planning_runs(claim_id TEXT PRIMARY KEY);INSERT INTO native_planning_runs VALUES('missing'),('zero'),('observed');").unwrap();
        db
    }
    /// Feed the actual bounded parser; never construct an observed usage channel
    /// from arbitrary Agent JSON. No process or billing account is involved.
    fn observed(claim: &str, input: i64, output: i64) -> UsageRecord {
        let values = crate::EngineSettingsValues::installation_defaults().unwrap();
        let mut stream = crate::PlanningOutputStream::new(&values).unwrap();
        let event = serde_json::json!({"type":"result","structured_output":{"input_events":["event"],"reason":"Owned fixture","operations":[]},"usage":{"input_tokens":input,"output_tokens":output}});
        stream
            .feed(&(event.to_string() + "\n").into_bytes())
            .unwrap();
        let result = stream.finish().unwrap();
        assert!(!result.raw_proposal.is_empty());
        UsageRecord::observe(claim, &stream.facts_snapshot()).unwrap()
    }
    /// Required token presence and SQLNULL must distinguish unknown from observed0.
    /// Roundtrip checks actual scalar types without modifying any financial ledger.
    #[test]
    fn planning_native_usage_distinguishes_unknown_and_observed_zero() {
        let mut db = fixture();
        let tx = db.transaction().unwrap();
        let absent = UsageRecord::observe(
            "missing",
            &crate::agent_provider_facts::ProviderFacts::default().snapshot(),
        )
        .unwrap();
        assert_eq!(absent.input_tokens, None);
        assert_eq!(absent.quality, "not_reported");
        record(&tx, &absent).unwrap();
        let zero = observed("zero", 0, 0);
        record(&tx, &zero).unwrap();
        assert_eq!(load(&tx, "missing").unwrap().input_tokens, None);
        assert_eq!(load(&tx, "zero").unwrap().input_tokens, Some(0));
        let cost:String=tx.query_row("SELECT typeof(provider_reported_cost_usd) FROM native_planning_usage WHERE claim_id='zero'",[],|r|r.get(0)).unwrap();
        assert_eq!(cost, "null");
        let mut malformed = zero;
        malformed.input_tokens = None;
        assert!(malformed.validate().is_err());
        tx.commit().unwrap();
    }
    /// A valid observation precedes conflicting/invalid candidates. Refusals must
    /// preserve original measurement and permit exact replay within the same TX.
    #[test]
    fn planning_native_usage_conflict_and_invalid_cost_refuse_without_mutation() {
        let mut db = fixture();
        let tx = db.transaction().unwrap();
        let good = observed("observed", 7, 8);
        record(&tx, &good).unwrap();
        let mut conflict = good.clone();
        conflict.output_tokens = Some(9);
        assert!(record(&tx, &conflict).is_err());
        assert!(load(&tx, "observed").unwrap() == good);
        record(&tx, &good).unwrap();
        for invalid in [-1., f64::INFINITY, f64::NAN] {
            let mut bad = good.clone();
            bad.provider_reported_cost_usd = Some(invalid);
            assert!(record(&tx, &bad).is_err());
            assert!(load(&tx, "observed").unwrap() == good);
            record(&tx, &good).unwrap();
        }
        tx.commit().unwrap();
    }
    /// Extra trigger rejection is structural, never successful empty observation.
    /// After removing only the owned injected trigger, the original row replays.
    #[test]
    fn planning_native_usage_rejects_schema_interference_and_recovers() {
        let mut db = fixture();
        let tx = db.transaction().unwrap();
        let good = observed("observed", 7, 8);
        record(&tx, &good).unwrap();
        tx.execute_batch("CREATE TRIGGER fixture_usage_ignore BEFORE INSERT ON native_planning_usage BEGIN SELECT RAISE(IGNORE); END;").unwrap();
        assert!(record(&tx, &good).is_err());
        tx.execute_batch("DROP TRIGGER fixture_usage_ignore")
            .unwrap();
        record(&tx, &good).unwrap();
        assert!(load(&tx, "observed").unwrap() == good);
        tx.commit().unwrap();
    }
}
