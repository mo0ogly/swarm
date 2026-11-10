//! Data binding for review instructions; no authority or evidence of execution.
use crate::{encode_model, Task};
use sha2::{Digest, Sha256};
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Hash the task's review instructions with Go-compatible JSON semantics.
///
/// The base binds title, deliverable, criteria, dependencies and planning-brief
/// hash. Nonempty memory selections add their ordered references to a second
/// hash layer. Nil/empty criteria or dependencies differ; empty/absent memory
/// selections are equivalent. No task mutation or external read occurs.
///
/// This digest proves neither memory access rights nor current identity,
/// execution, reviewer independence or acceptance.
///
/// # Errors
/// Returns [`crate::WorkError`] if compatible JSON encoding fails.
pub fn review_contract(task: &Task) -> Result<String, crate::WorkError> {
    let base = encode_model(&(
        &task.title,
        &task.deliverable,
        &task.criteria,
        &task.depends,
        &task.plan_brief_hash,
    ))?;
    let raw = if task
        .memory_selections
        .as_ref()
        .is_some_and(|v| !v.is_empty())
    {
        encode_model(&(hash(&base), &task.memory_selections))?
    } else {
        base
    };
    Ok(hash(&raw))
}

/// Hash the selected and already accepted tasks in their stored Work order.
///
/// Matches Go `managedReviewContract`: each entry binds task ID, latest attempt
/// ID (empty if absent), [`review_contract`] and the validation-policy data.
/// An absent selection is allowed; duplicate IDs are not validated here.
/// An empty cohort is encoded as `[]`. Older attempts and excluded tasks do not
/// participate. The function performs no external reads or mutation.
///
/// Neither policy validity, candidate Git, current identity nor acceptance
/// authority is established by this data digest; callers must verify them.
///
/// # Errors
/// Returns [`crate::WorkError`] if task or cohort JSON encoding fails.
///
/// # Examples
/// ```
/// use swarm_engine::{managed_review_contract, Work};
/// let digest = managed_review_contract(&Work::default(), "absent-task")?;
/// assert_eq!(digest, "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945");
/// # Ok::<(), swarm_engine::WorkError>(())
/// ```
pub fn managed_review_contract(
    work: &crate::Work,
    selected: &str,
) -> Result<String, crate::WorkError> {
    let mut entries = Vec::new();
    for task in work.tasks.iter().flatten() {
        if task.id != selected && task.status != "accepted" {
            continue;
        }
        let attempt = task
            .attempts
            .as_ref()
            .and_then(|v| v.last())
            .map_or("", |a| a.id.as_str());
        entries.push((
            &task.id,
            attempt,
            review_contract(task)?,
            &task.validation_policy,
        ));
    }
    Ok(hash(&encode_model(&entries)?))
}
