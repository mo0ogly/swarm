//! Bounded Git tree inventory for an already verified managed preview.
use super::{inputs, task_validation::ReviewCandidate, E};
use serde::Serialize;

/// Exact changes against the recorded parent, plus the size of the full
/// inventory used by review assembly. This projection grants no invocation.
#[derive(Serialize)]
pub struct ManagedReviewInventory {
    pub candidate_commit: String,
    pub parent_commit: String,
    pub tracked_count: usize,
    pub changed_count: usize,
    pub delivery_file_limit: Option<usize>,
    pub full_inventory_exceeds_limit: bool,
    pub changes: Vec<ManagedReviewPath>,
    pub control_inputs: Vec<ManagedReviewControlInput>,
}

impl ManagedReviewInventory {
    /// Select retained changed paths and declared control inputs in stable path
    /// order. Removed paths stay in the delta, with no candidate bytes. Only
    /// unchanged control inputs may be represented as non-text observations.
    /// A declared input missing from the candidate cannot be silently omitted.
    pub(in crate::root_store) fn focused_paths(&self) -> Result<std::collections::BTreeMap<&str, bool>, E> {
        let mut paths = std::collections::BTreeMap::new();
        for change in &self.changes {
            if change.operation != "deleted" {
                paths.insert(change.path.as_str(), false);
            }
        }
        for input in &self.control_inputs {
            if input.tracked_sha256.is_none() {
                return Err(E::Refused("review-focused-control-input-untracked"));
            }
            paths.entry(input.path.as_str()).or_insert(true);
        }
        Ok(paths)
    }
}

/// One path changed between the two retained trees, without file content.
#[derive(Serialize)]
pub struct ManagedReviewPath {
    pub path: String,
    pub operation: &'static str,
}

/// One input declared by a retained control, attributed to its task and slot.
#[derive(Serialize)]
pub struct ManagedReviewControlInput {
    pub task_id: String,
    pub control_id: String,
    pub path: String,
    /// Absent if the declared path is outside the retained tracked inventory.
    pub tracked_sha256: Option<String>,
    pub tracked_mode: Option<String>,
}

/// Compare verified commit objects in the held checkout. No diff driver,
/// worktree filter, rename inference or blob content is needed. Use the saved
/// Git command policy. Both object identities and final source refresh belong
/// to the caller. Missing limits, Git failures and invalid wire data refuse.
pub(in crate::root_store) fn observe(
    candidate: &ReviewCandidate,
    settings: &crate::EngineSettingsDocument,
    delivery_limit: Option<i64>,
    project_subdir: &str,
) -> Result<ManagedReviewInventory, E> {
    let checkpoint = candidate.candidate();
    let cap = inputs::cap(delivery_limit.as_ref())?;
    let tracked_count = checkpoint.review_entries().len();
    let raw = crate::managed_repository::git(
        &candidate.repository(),
        &[
            "diff-tree", "-r", "--no-commit-id", "--name-status", "-z",
            "--no-renames", "--no-ext-diff", "--no-textconv",
            &checkpoint.previous, &checkpoint.commit, "--",
        ],
        settings,
    )
    .map_err(|error| E::Launch(Box::new(error)))?;
    let changes = parse(&raw)?;
    let control_inputs = control_inputs(checkpoint, project_subdir)?;
    Ok(ManagedReviewInventory {
        candidate_commit: checkpoint.commit.clone(),
        parent_commit: checkpoint.previous.clone(),
        tracked_count,
        changed_count: changes.len(),
        delivery_file_limit: cap,
        full_inventory_exceeds_limit: cap.is_some_and(|limit| tracked_count > limit),
        changes,
        control_inputs,
    })
}

/// Report each normalized control input against the immutable Git checkpoint.
/// Inputs are relative to the retained project directory, independently of the
/// command's working directory. A source absent from Git is still listed, with
/// no tracked digest. Missing original cohort plans refuse. This does not open
/// inputs or imply that a hash is suitable evidence for a review criterion.
fn control_inputs(
    checkpoint: &crate::managed_repository::ManagedCandidate,
    project_subdir: &str,
) -> Result<Vec<ManagedReviewControlInput>, E> {
    let plan = checkpoint.control_plan.as_ref()
        .ok_or(E::Refused("review-managed-plan-required"))?;
    let mut inputs = Vec::new();
    for slot in &plan.slots {
        for name in slot.control.inputs.iter().flatten() {
            let path = if project_subdir.is_empty() {
                name.clone()
            } else {
                format!("{project_subdir}/{name}")
            };
            let entry = checkpoint.review_entry(&path);
            inputs.push(ManagedReviewControlInput {
                task_id: slot.task_id.clone(),
                control_id: slot.control.id.clone(),
                path,
                tracked_sha256: entry.map(|(_, sha)| sha.to_owned()),
                tracked_mode: entry.map(|(mode, _)| mode.to_owned()),
            });
        }
    }
    Ok(inputs)
}

/// Decode Git's NUL-separated status/path pairs without lossy UTF-8 conversion.
/// An incomplete frame, empty path or unrecognized status refuses the inventory.
fn parse(raw: &[u8]) -> Result<Vec<ManagedReviewPath>, E> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let framed = raw
        .strip_suffix(&[0])
        .ok_or(E::Refused("review-inventory-frame-invalid"))?;
    let mut fields = framed.split(|byte| *byte == 0);
    let mut changes = Vec::new();
    while let Some(status) = fields.next() {
        let operation = match status {
            b"A" => "added",
            b"D" => "deleted",
            b"M" => "modified",
            b"T" => "type_changed",
            _ => return Err(E::Refused("review-inventory-status-invalid")),
        };
        let path = fields
            .next()
            .filter(|value| !value.is_empty())
            .and_then(|value| std::str::from_utf8(value).ok())
            .ok_or(E::Refused("review-inventory-path-invalid"))?;
        changes.push(ManagedReviewPath { path: path.into(), operation });
    }
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserve exact path bytes and keep incomplete or ambiguous frames out.
    #[test]
    fn inventory_tree_delta_decodes_or_refuses() {
        assert!(parse(b"").unwrap().is_empty());
        let result = parse(b"A\0new file\0D\0old\0M\0line\nbreak\0T\0type\0").unwrap();
        assert_eq!(
            result.iter().map(|r| (r.operation, r.path.as_str())).collect::<Vec<_>>(),
            [("added", "new file"), ("deleted", "old"), ("modified", "line\nbreak"),
             ("type_changed", "type")],
        );
        for raw in [b"M\0file".as_slice(), b"M\0", b"M\0\0", b"X\0file\0", b"M\0\xff\0"] {
            assert!(parse(raw).is_err());
        }
    }

    /// A large held tree contributes only changed surviving files and declared
    /// controls. A changed control must remain readable as source, and a missing
    /// declared original refuses even though there is still another valid source.
    #[test]
    fn focused_review_sources_keep_delta_and_controls() {
        let control = |path: &str, tracked: bool| ManagedReviewControlInput {
            task_id: "task-a".into(),
            control_id: "check".into(),
            path: path.into(),
            tracked_sha256: tracked.then(|| "a".repeat(64)),
            tracked_mode: tracked.then(|| "100644".into()),
        };
        let mut inventory = ManagedReviewInventory {
            candidate_commit: "candidate".into(),
            parent_commit: "parent".into(),
            tracked_count: 101,
            changed_count: 2,
            delivery_file_limit: Some(100),
            full_inventory_exceeds_limit: true,
            changes: parse(b"M\0report.md\0D\0removed.md\0").unwrap(),
            control_inputs: vec![control("report.md", true), control("checked.docx", true)],
        };
        let paths: Vec<_> = inventory.focused_paths().unwrap().into_iter().collect();
        assert_eq!(paths, [("checked.docx", true), ("report.md", false)]);
        inventory.control_inputs.push(control("missing.md", false));
        assert!(matches!(
            inventory.focused_paths(),
            Err(E::Refused("review-focused-control-input-untracked"))
        ));
    }
}
