//! Complete retained inputs for the future legacy mission-status aggregate.
//! No clock, activity, acceptance, launch grant or operator window is inferred.
use super::{finished, RootStore, RootStoreError as E};
use crate::{Agent, RecordedEvent, Usage, Work};
use rusqlite::{Connection, OptionalExtension, Params, Row};
use serde::Deserialize;

/// Exact retained visit, or absence when this operator has never marked a visit.
/// Reading this value does not update it. No arbitrary user identity is selected.
#[derive(Clone, PartialEq, Eq)]
pub struct MissionStatusVisit {
    pub revision: i64,
    pub at: String,
}
/// Retained nonreleased planning reservation, matching Go spending selection.
/// A reservation and its optional declaration never prove inference was sent.
#[derive(Clone, PartialEq)]
pub struct MissionStatusPlanningCall {
    pub id: String,
    pub scope_id: String,
    pub state: String,
    pub created_at: String,
    pub usage: Option<Usage>,
}
/// Stored exchange artifact declaration; no filesystem/hash verification occurs.
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct MissionStatusExchangeArtifact {
    pub path: String,
    pub sha256: String,
}
/// Persisted exchange observation, never recipient permission or acknowledgement
/// authentication. Optional artifact-array null is preserved, not invented [].
#[derive(Clone, PartialEq, Eq)]
pub struct MissionStatusExchange {
    pub id: String,
    pub work_id: String,
    pub kind: String,
    pub source_agent_id: String,
    pub source_task_id: String,
    pub source_attempt_id: String,
    pub recipient_task_id: String,
    pub recipient_role: String,
    pub recipient_attempt_id: String,
    pub reply_to: String,
    pub state: String,
    pub result_state: String,
    pub need: String,
    pub artifacts: Option<Vec<MissionStatusExchangeArtifact>>,
    pub created_at: String,
    pub deadline_at: String,
    pub acknowledged_at: String,
    pub acknowledged_by: String,
    pub acknowledgement: String,
}
/// One SQL snapshot of complete retained data needed by the legacy aggregate.
/// This intentionally has no Serialize/Debug: private Agent/Work/history must be
/// projected deliberately, not accidentally exposed as a public status document.
/// Runtime/OS observations, settings, controls, global occupied workspaces and
/// native validation evidence still need their own explicit contracts.
pub struct MissionStatusInputs {
    pub work: Work,
    pub agents: Vec<Agent>,
    pub events: Vec<RecordedEvent>,
    pub visit: Option<MissionStatusVisit>,
    pub planning_calls: Vec<MissionStatusPlanningCall>,
    pub exchanges: Vec<MissionStatusExchange>,
}

/// Execute and fully finalize a read statement. SQL/decoder plus finalization
/// failures retain both causes through the established Root cleanup wrapper.
/// No mutation, transaction, retry, hidden row ceiling or silent skip is added.
fn collect<T>(
    db: &Connection,
    sql: &str,
    params: impl Params,
    mut decode: impl FnMut(&Row<'_>) -> Result<T, E>,
) -> Result<Vec<T>, E> {
    let mut statement = db.prepare(sql).map_err(E::Sql)?;
    let result = (|| {
        let mut rows = statement.query(params).map_err(E::Sql)?;
        let mut values = Vec::new();
        while let Some(row) = rows.next().map_err(E::Sql)? {
            values.push(decode(row)?);
        }
        Ok(values)
    })();
    finished(result, statement.finalize().map_err(E::Sql))
}

/// Decode the Go spending-usage contract: empty/null is absent, a measurement
/// requires both integer token counts. Invalid JSON/types/negative counters or
/// nonfinite/negative costs refuse, without turning a partial object into zero.
/// Unknown fields follow Go's retained forward-compatible object semantics.
fn spending_usage(raw: &[u8]) -> Result<Option<Usage>, E> {
    if raw.is_empty() {
        return Ok(None);
    }
    #[derive(Deserialize)]
    struct Required {
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    }
    let value: &serde_json::value::RawValue =
        serde_json::from_slice(raw).map_err(|e| E::Encode(crate::WorkError::Json(e)))?;
    if value.get() == "null" {
        return Ok(None);
    }
    use crate::model_wire::Kind;
    let required: Required = crate::work::decode(
        raw,
        &[
            ("input_tokens", Kind::Pointer(&Kind::Int64)),
            ("output_tokens", Kind::Pointer(&Kind::Int64)),
        ],
    )
    .map_err(E::Encode)?;
    if !matches!((required.input_tokens, required.output_tokens), (Some(i), Some(o)) if i >= 0 && o >= 0)
    {
        return Err(E::Refused("mission-status-usage-tokens-required"));
    }
    // Existing generated Go model supplies zero only for Go's optional scalar
    // metadata; required token counts above are never defaulted.
    let usage: Usage = crate::work::decode(raw, crate::models::shape_usage()).map_err(E::Encode)?;
    if [
        &usage.cache_read_input_tokens,
        &usage.cache_creation_input_tokens,
        &usage.cached_input_tokens,
    ]
    .iter()
    .any(|v| v.as_deref().is_some_and(|n| *n < 0))
    {
        return Err(E::Refused("mission-status-usage-cache-invalid"));
    }
    if usage
        .provider_reported_cost_usd
        .as_deref()
        .is_some_and(|n| !n.0.is_finite() || n.0 < 0.0)
    {
        return Err(E::Refused("mission-status-usage-cost-invalid"));
    }
    Ok(Some(usage))
}

/// Decode the historical artifact array with Go struct-zero/null semantics.
/// Unknown object fields remain compatible; malformed scalar/array/member types
/// retain codec causes. No file access, artifact validation or hash grant occurs.
fn exchange_artifacts(raw: &[u8]) -> Result<Option<Vec<MissionStatusExchangeArtifact>>, E> {
    use crate::model_wire::{Fields, Kind};
    fn artifact() -> Fields {
        &[("path", Kind::String), ("sha256", Kind::String)]
    }
    #[derive(Deserialize)]
    struct Envelope {
        artifacts: Option<Vec<MissionStatusExchangeArtifact>>,
    }
    let raw: &serde_json::value::RawValue =
        serde_json::from_slice(raw).map_err(|e| E::Encode(crate::WorkError::Json(e)))?;
    #[derive(serde::Serialize)]
    struct RetainedEnvelope<'a> {
        artifacts: &'a serde_json::value::RawValue,
    }
    // Serialize RawValue directly: a Value intermediary would collapse duplicate
    // keys before Go's ordered merge/type checks and could hide a bad value.
    let wrapped = serde_json::to_vec(&RetainedEnvelope { artifacts: raw })
        .map_err(|e| E::Encode(crate::WorkError::Json(e)))?;
    let envelope: Envelope = crate::work::decode(
        &wrapped,
        &[("artifacts", Kind::Array(&Kind::Object(artifact)))],
    )
    .map_err(E::Encode)?;
    Ok(envelope.artifacts)
}

/// Read exact retained visit/history/call/exchange rows in the caller snapshot.
/// Work and Agent ownership are validated; every planning usage/artifact is
/// decoded before returning. Unsupported/malformed storage refuses the entire
/// observation. This does not classify activity, task validation or budget.
fn read(db: &Connection, work: &str, operator: &str) -> Result<MissionStatusInputs, E> {
    let document = crate::read_recorded_work(db, work).map_err(E::Work)?;
    let agents = crate::list_agents(db, work).map_err(E::Agent)?;
    for agent in &agents {
        crate::agent_store::verify_agent_ownership(db, agent).map_err(E::Agent)?;
    }
    let events = crate::list_recorded_events(db, work).map_err(E::Events)?;
    let mut statement = db
        .prepare("SELECT revision,at FROM session_visits WHERE work_id=?1 AND operator=?2")
        .map_err(E::Sql)?;
    let visit = statement
        .query_row([work, operator], |r| {
            Ok(MissionStatusVisit {
                revision: r.get(0)?,
                at: r.get(1)?,
            })
        })
        .optional()
        .map_err(E::Sql);
    let visit = finished(visit, statement.finalize().map_err(E::Sql))?;
    let planning_calls = collect(db,
        "SELECT id,scope_id,state,created_at,usage FROM planning_calls WHERE work_id=?1 AND state!='released' ORDER BY created_at,id", [work], |row| {
        let raw: Option<Vec<u8>> = row.get(4).map_err(E::Sql)?;
        Ok(MissionStatusPlanningCall {
            id: row.get(0).map_err(E::Sql)?, scope_id: row.get(1).map_err(E::Sql)?,
            state: row.get(2).map_err(E::Sql)?, created_at: row.get(3).map_err(E::Sql)?,
            usage: spending_usage(raw.as_deref().unwrap_or_default())?,
        })
    })?;
    let exchanges = collect(db,
        "SELECT id,work_id,kind,source_agent_id,source_task_id,source_attempt_id,recipient_task_id,recipient_role,recipient_attempt_id,reply_to,state,result_state,need,artifacts,created_at,deadline_at,acknowledged_at,acknowledged_by,acknowledgement FROM agent_exchanges WHERE work_id=?1 ORDER BY created_at,id", [work], |row| {
        let artifacts: Vec<u8> = row.get(13).map_err(E::Sql)?;
        let artifacts = exchange_artifacts(&artifacts)?;
        Ok(MissionStatusExchange {
            id: row.get(0).map_err(E::Sql)?, work_id: row.get(1).map_err(E::Sql)?,
            kind: row.get(2).map_err(E::Sql)?, source_agent_id: row.get(3).map_err(E::Sql)?,
            source_task_id: row.get(4).map_err(E::Sql)?, source_attempt_id: row.get(5).map_err(E::Sql)?,
            recipient_task_id: row.get(6).map_err(E::Sql)?, recipient_role: row.get(7).map_err(E::Sql)?,
            recipient_attempt_id: row.get(8).map_err(E::Sql)?, reply_to: row.get(9).map_err(E::Sql)?,
            state: row.get(10).map_err(E::Sql)?, result_state: row.get(11).map_err(E::Sql)?,
            need: row.get(12).map_err(E::Sql)?, artifacts,
            created_at: row.get(14).map_err(E::Sql)?, deadline_at: row.get(15).map_err(E::Sql)?,
            acknowledged_at: row.get(16).map_err(E::Sql)?, acknowledged_by: row.get(17).map_err(E::Sql)?,
            acknowledgement: row.get(18).map_err(E::Sql)?,
        })
    })?;
    Ok(MissionStatusInputs {
        work: document,
        agents,
        events,
        visit,
        planning_calls,
        exchanges,
    })
}
impl RootStore {
    /// Observe complete retained status inputs for an existing Work and explicit
    /// operator in one read transaction. Never mark a visit or mutate a ledger,
    /// reservation, lease, exchange, Work or Agent. No UI/history cutoff applies.
    /// The returned private inputs are not MissionStatus and grant no permission.
    /// Unknown Work, invalid storage/usage/ownership and SQL/finalization/rollback
    /// failures retain their causes. Caller must still close Root before output.
    pub fn mission_status_inputs(
        &self,
        work: &str,
        operator: &str,
    ) -> Result<MissionStatusInputs, E> {
        let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = read(&tx, work, operator);
        finished(result, tx.rollback().map_err(E::Sql))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Real zero, absent measurement and malformed partial object stay distinct.
    /// Fixtures are protocol inputs, not operator policy or provider evidence.
    #[test]
    fn mission_status_usage_absence_zero_required_tokens_and_costs() {
        for raw in [b"".as_slice(), b"null", b" \n null "] {
            assert!(spending_usage(raw).unwrap().is_none());
        }
        let zero = spending_usage(
            br#"{"input_tokens":0,"output_tokens":0,"provider_reported_cost_usd":0}"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(zero.input_tokens, 0);
        assert!(matches!(
            spending_usage(
                br#"{"input_tokens":1,"input_tokens":2,"input_tokens":null,"output_tokens":0}"#,
            ),
            Err(E::Refused("mission-status-usage-tokens-required"))
        ));
        assert_eq!(zero.provider_reported_cost_usd.unwrap().0, 0.0);
        for raw in [
            br#"{}"#.as_slice(),
            br#"{"input_tokens":0}"#,
            br#"{"input_tokens":null,"output_tokens":0}"#,
            br#"{"input_tokens":-1,"output_tokens":0}"#,
            br#"{"input_tokens":0,"output_tokens":0,"cached_input_tokens":-1}"#,
            br#"{"input_tokens":0,"output_tokens":0,"provider_reported_cost_usd":-1}"#,
            br#"{"input_tokens":0.5,"output_tokens":0}"#,
            b"private malformed payload",
        ] {
            let error = match spending_usage(raw) {
                Ok(_) => panic!("invalid usage accepted"),
                Err(e) => e,
            };
            assert!(!error.to_string().contains("private malformed payload"));
        }
    }
    /// Go null/empty/struct-zero semantics remain distinct; invalid members
    /// refuse without echoing private payload or inspecting any artifact file.
    #[test]
    fn mission_status_artifacts_preserve_nil_empty_and_struct_zero() {
        assert!(exchange_artifacts(b"null").unwrap().is_none());
        assert!(exchange_artifacts(b"[]").unwrap().unwrap().is_empty());
        let values =
            exchange_artifacts(br#"[null,{"path":null},{"path":"a","sha256":"b","future":true}]"#)
                .unwrap()
                .unwrap();
        assert_eq!(values.len(), 3);
        assert!(values[0].path.is_empty());
        assert!(values[1].path.is_empty());
        assert_eq!(values[2].sha256, "b");
        for raw in [
            b"[7]".as_slice(),
            br#"[{"path":7}]"#,
            b"{}",
            b"private bad JSON",
        ] {
            let error = match exchange_artifacts(raw) {
                Ok(_) => panic!("invalid artifact accepted"),
                Err(e) => e,
            };
            assert!(!error.to_string().contains("private bad JSON"));
        }
    }
    /// Exercise the actual SQL producer over an isolated schema37 database.
    /// Direct SQL creates historical/corrupt reader fixtures only; it does not
    /// qualify public writes, launch authority, provider life or CLI status.
    #[test]
    fn mission_status_inputs_complete_order_no_write_refusal_and_recovery() {
        let db = Connection::open_in_memory().unwrap();
        crate::register_identity_plan_projection(&db).unwrap();
        db.execute_batch(crate::root_schema::INITIAL_SCHEMA)
            .unwrap();
        db.execute(
            "INSERT INTO works(id,revision,body) VALUES('fixture',1,?1)",
            [br#"{"schema_version":1,"id":"fixture","revision":1}"#.as_slice()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO session_visits VALUES('fixture','operator-a',1,'retained-visit')",
            [],
        )
        .unwrap();
        for id in ["z", "a"] {
            db.execute("INSERT INTO planning_calls VALUES(?1,'fixture','root',1,NULL,'estimated','same-time')",[id]).unwrap();
        }
        db.execute("INSERT INTO planning_calls VALUES('released','fixture','root',1,X'00','released','same-time')",[]).unwrap();
        let before = db.total_changes();
        let absent = read(&db, "fixture", "operator-b").unwrap();
        assert!(absent.visit.is_none());
        assert_eq!(
            absent
                .planning_calls
                .iter()
                .map(|v| v.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "z"]
        );
        assert!(
            absent.agents.is_empty() && absent.events.is_empty() && absent.exchanges.is_empty()
        );
        let known = read(&db, "fixture", "operator-a").unwrap();
        assert_eq!(known.visit.unwrap().at, "retained-visit");
        assert_eq!(db.total_changes(), before);
        assert!(matches!(
            read(&db, "missing", "operator-a"),
            Err(E::Work(crate::RecordedWorkError::Missing))
        ));
        db.execute(
            "UPDATE planning_calls SET usage=?1 WHERE id='a'",
            [br#"{"input_tokens":0}"#.as_slice()],
        )
        .unwrap();
        let before_failure = db.total_changes();
        assert!(matches!(
            read(&db, "fixture", "operator-a"),
            Err(E::Refused("mission-status-usage-tokens-required"))
        ));
        assert_eq!(db.total_changes(), before_failure);
        db.execute("UPDATE planning_calls SET usage=NULL WHERE id='a'", [])
            .unwrap();
        assert_eq!(
            read(&db, "fixture", "operator-a")
                .unwrap()
                .planning_calls
                .len(),
            2
        );
        db.close().unwrap();
    }
    /// Reproducer from independent source review: duplicate keys must reach the
    /// Go-compatible codec intact. A later null scalar preserves earlier text;
    /// a bad earlier scalar cannot be hidden by a later valid duplicate.
    #[test]
    fn mission_status_artifact_duplicate_keys_preserve_go_merge_and_errors() {
        let values = exchange_artifacts(br#"[{"path":"original","path":null}]"#)
            .unwrap()
            .unwrap();
        assert_eq!(values[0].path, "original");
        assert!(exchange_artifacts(br#"[{"path":7,"path":"ok"}]"#).is_err());
    }
    /// Statement decoding failure refuses the observation and leaves the same
    /// rows intact; a valid subsequent read demonstrates connection recovery.
    #[test]
    fn mission_status_collect_failure_preserves_rows_and_recovery() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE samples(value); INSERT INTO samples VALUES(7),(8)")
            .unwrap();
        let read_values = || {
            collect(&db, "SELECT value FROM samples ORDER BY rowid", [], |r| {
                r.get::<_, i64>(0).map_err(E::Sql)
            })
        };
        assert_eq!(read_values().unwrap(), vec![7, 8]);
        let error = collect(&db, "SELECT value FROM samples ORDER BY rowid", [], |r| {
            if r.get::<_, i64>(0).map_err(E::Sql)? == 8 {
                Err(E::Refused("fixture-decoder-refused"))
            } else {
                Ok(())
            }
        });
        assert!(matches!(error, Err(E::Refused("fixture-decoder-refused"))));
        assert_eq!(read_values().unwrap(), vec![7, 8]);
        db.close().unwrap();
    }
}
