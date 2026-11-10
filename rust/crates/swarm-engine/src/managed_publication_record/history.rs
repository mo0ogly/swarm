//! Iterative native publication lineage; SQL custody is not current quality.
use super::{digest, matches_task, read, Record};
use crate::{AgentLaunchError as E, Task, Work};
use std::collections::{BTreeMap, BTreeSet};

/// Inventory exact accepted tasks, rejecting duplicate identities in the entire
/// Work. Returns borrowed tasks; the chain reader hashes them when comparing.
/// No SQL, filesystem access, encoding or operator policy is introduced here.
fn accepted(work: &Work) -> Result<BTreeMap<&str, &Task>, E> {
    let mut ids = BTreeSet::new();
    let mut result = BTreeMap::new();
    for task in work.tasks.iter().flatten() {
        if !ids.insert(task.id.as_str()) {
            return Err(E::Refused("managed-publication-task-ambiguous"));
        }
        if task.status == "accepted" { result.insert(task.id.as_str(), task); }
    }
    Ok(result)
}

/// Read every integrated native row in canonical revision order and verify its
/// exact event, worker base/result, immutable predecessor and accepted-task set.
/// Each publication adds exactly its newly accepted task, preserving all earlier
/// accepted task bytes. Terminal candidate/base and accepted tasks must match the
/// caller's actual Work. Empty is valid only at the initial base. No identifier
/// ordering, recursion limit, retry, IO fallback or operator cutoff is used.
///
/// This is a custody/lineage read only. Authority consumers must additionally
/// verify native originals, fresh saved policies, actual reports and current
/// whole-cohort controls/review. SQL/codec errors keep causes; no write occurs.
pub(crate) fn chain(db: &rusqlite::Connection, work: &Work) -> Result<Vec<Record>, E> {
    let repo = work.planning.as_deref().and_then(|p|p.repository.as_deref())
        .ok_or(E::Refused("managed-publication-repository-required"))?;
    let current_tasks = accepted(work)?;
    let rows = crate::agent_launch::query(db,
        "SELECT m.agent_id,m.task_id,m.base_commit,m.result_commit,m.detail FROM main.managed_attempts m LEFT JOIN main.events e ON e.id=m.detail AND e.work_id=m.work_id WHERE m.work_id=? AND m.state='integrated' ORDER BY e.revision",
        [&work.id], |rows| {
            let mut result = Vec::new();
            while let Some(row) = rows.next()? {
                result.push((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?));
            }
            Ok(result)
        })?;
    let mut records: Vec<Record> = Vec::new();
    let mut ids = BTreeSet::new();
    let mut candidates = BTreeSet::from([repo.base_commit.as_str().to_owned()]);
    let mut previous_tasks: BTreeMap<String, String> = BTreeMap::new();
    let mut previous_commit = repo.base_commit.clone();
    for (agent, task, base, result, event) in rows {
        let record = read(db,&event)?.ok_or(E::Refused("managed-publication-record-required"))?;
        if !ids.insert(event) || record.work_id != work.id || record.producer != agent
            || record.task_id != task || record.worker_base()? != base || record.result_commit != result
            || record.previous_candidate != previous_commit || !candidates.contains(&base)
            || !candidates.insert(record.candidate_commit.clone()) {
            return Err(E::Refused("managed-publication-chain-binding"));
        }
        match (records.last(), record.previous_publication.as_ref()) {
            (None, None) if record.schema_version == 1 => {},
            (Some(previous), Some(link)) if record.schema_version == 2 => {
                if link.event_id != previous.event_id || link.sha256 != digest(previous)?
                    || record.expected_revision < previous.accepted_work.revision {
                    return Err(E::Refused("managed-publication-chain-predecessor"));
                }
            },
            _ => return Err(E::Refused("managed-publication-chain-predecessor")),
        }
        let tasks = accepted(&record.accepted_work)?;
        if previous_tasks.contains_key(&record.task_id)
            || tasks.len().checked_sub(1) != Some(previous_tasks.len())
            || record.accepted_work.planning.as_deref().and_then(|p|p.repository.as_deref())
                .is_none_or(|r|r.base_commit != repo.base_commit) {
            return Err(E::Refused("managed-publication-chain-cohort"));
        }
        for (id, hash) in &previous_tasks {
            if tasks.get(id.as_str()).map(|t|digest(*t)).transpose()?.as_ref() != Some(hash) {
                return Err(E::Refused("managed-publication-chain-task-changed"));
            }
        }
        previous_tasks.insert(record.task_id.clone(),record.accepted_task_sha256.clone());
        previous_commit = record.candidate_commit.clone();
        records.push(record);
    }
    if previous_commit != repo.candidate_commit || previous_tasks.len() != current_tasks.len() {
        return Err(E::Refused("managed-publication-chain-current"));
    }
    for record in &records { matches_task(record, work)?; }
    Ok(records)
}
