//! Candidate construction inside an owned scratch clone, never publication.
use super::{commit, git, text, LocalDirectory, E};
use crate::EngineSettingsDocument;

/// Merge one immutable worker result into the current cumulative candidate.
///
/// The caller owns a clean standalone scratch clone and its Git descriptors,
/// with both commits already fetched, under the attempt's exclusive ownership.
/// It must check actual worker/Root/lease/consent before using this result. Git
/// runs under saved command policy, outside the Root SQL/settings reservation.
/// Inputs are canonical commit IDs and an actual validated agent ID; revision
/// expressions refuse before checkout. The generated commit has `previous` as
/// its single parent, matching the retained Go candidate contract, and the clone
/// is reset to it before any controls may run. Generated Swarm identity describes
/// technical objects only, not a human author or permission to execute a control.
///
/// Return the canonical candidate OID. Git/merge errors retain their typed cause
/// and may leave a checkout, index, objects or merge-conflict state. The caller
/// preserves that evidence; this function never aborts/retries a failed merge,
/// deletes a clone, publishes a ref, advances Work or accepts a task. Controls,
/// immutable checkpoint, independent review and protected publication remain
/// separate required consumers; this producer alone is not a usable delivery.
pub(super) fn merge_candidate(
    copy: &LocalDirectory,
    previous: &str,
    result: &str,
    agent: &str,
    settings: &EngineSettingsDocument,
) -> Result<String, E> {
    if !swarm_identity_contract::name(agent.as_bytes()) {
        return Err(E::Refused("managed-candidate-agent-invalid"));
    }
    let path = copy.process_path();
    commit(&path, previous, settings)?;
    commit(&path, result, settings)?;
    git(&path, &["checkout", "--detach", previous], settings)?;
    git(
        &path,
        &[
            "-c",
            "user.name=Swarm",
            "-c",
            "user.email=swarm@localhost",
            "merge",
            "--no-commit",
            "--no-ff",
            result,
        ],
        settings,
    )
    .map_err(|e| E::caused("managed-candidate-merge-failed", e))?;
    let tree = text(git(&path, &["write-tree"], settings)?)?;
    let oid = text(git(
        &path,
        &[
            "-c",
            "user.name=Swarm",
            "-c",
            "user.email=swarm@localhost",
            "commit-tree",
            &tree,
            "-p",
            previous,
            "-m",
            &format!("Candidat vérifié {agent}"),
        ],
        settings,
    )?)?;
    commit(&path, &oid, settings)?;
    git(&path, &["reset", "--hard", &oid], settings)?;
    Ok(oid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::fs::PermissionsExt, path::PathBuf};

    /// Own a fresh actual Git fixture; retain its path on any assertion failure.
    /// Generated technical commits are fixtures, never source checkout commits.
    fn fixture() -> (PathBuf, LocalDirectory, EngineSettingsDocument, String) {
        let path =
            std::env::temp_dir().join(crate::generate_identifier("managed-candidate-").unwrap());
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        println!("CANDIDATE_FIXTURE {}", path.display());
        let directory = LocalDirectory::open_owned(&path).unwrap();
        let values = crate::EngineSettingsValues::installation_defaults().unwrap();
        let settings = EngineSettingsDocument::initial(
            values,
            "owned-fixture".into(),
            "2026-10-25T00:00:00Z".into(),
        )
        .unwrap();
        git(&path, &["init", "--quiet"], &settings).unwrap();
        std::fs::write(path.join("a.txt"), b"base-a\n").unwrap();
        std::fs::write(path.join("b.txt"), b"base-b\n").unwrap();
        let base = record(&path, &settings, "base");
        (path, directory, settings, base)
    }

    /// Record only fixture edits with the inherited generated-object identity.
    fn record(path: &std::path::Path, settings: &EngineSettingsDocument, message: &str) -> String {
        git(path, &["add", "-A", "--", "."], settings).unwrap();
        git(
            path,
            &[
                "-c",
                "user.name=Swarm",
                "-c",
                "user.email=swarm@localhost",
                "commit",
                "--quiet",
                "-m",
                message,
            ],
            settings,
        )
        .unwrap();
        text(git(path, &["rev-parse", "HEAD"], settings).unwrap()).unwrap()
    }

    /// Delete only this successful owned fixture; failures keep their originals.
    fn close(path: PathBuf, directory: LocalDirectory) {
        directory.close().unwrap();
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn managed_candidate_merges_both_branches_before_any_controls() {
        let (path, directory, settings, base) = fixture();
        let branch = text(git(&path, &["symbolic-ref", "HEAD"], &settings).unwrap()).unwrap();
        std::fs::write(path.join("b.txt"), b"worker-b\n").unwrap();
        let result = record(&path, &settings, "worker");
        git(&path, &["checkout", "--detach", &base], &settings).unwrap();
        std::fs::write(path.join("a.txt"), b"accepted-a\n").unwrap();
        let previous = record(&path, &settings, "previous");
        let candidate =
            merge_candidate(&directory, &previous, &result, "owned-agent", &settings).unwrap();
        assert_eq!(
            text(
                git(
                    &path,
                    &["rev-list", "--parents", "-n", "1", &candidate],
                    &settings
                )
                .unwrap()
            )
            .unwrap(),
            format!("{candidate} {previous}")
        );
        assert_eq!(std::fs::read(path.join("a.txt")).unwrap(), b"accepted-a\n");
        assert_eq!(std::fs::read(path.join("b.txt")).unwrap(), b"worker-b\n");
        assert_eq!(
            text(git(&path, &["log", "-1", "--format=%s"], &settings).unwrap()).unwrap(),
            "Candidat vérifié owned-agent"
        );
        assert_eq!(
            text(git(&path, &["log", "-1", "--format=%an <%ae>"], &settings).unwrap()).unwrap(),
            "Swarm <swarm@localhost>"
        );
        assert!(
            git(&path, &["diff", "--exit-code", "HEAD", "--"], &settings)
                .unwrap()
                .is_empty()
        );
        // Result construction does not advance the fixture's existing branch.
        assert_eq!(
            text(git(&path, &["rev-parse", &branch], &settings).unwrap()).unwrap(),
            result
        );
        close(path, directory);
    }

    #[test]
    fn managed_candidate_valid_control_precedes_conflict_with_original_cause() {
        use std::error::Error;
        let (path, directory, settings, base) = fixture();
        // Same preconditions first prove a clean actual merge succeeds.
        let valid = merge_candidate(&directory, &base, &base, "owned-agent", &settings).unwrap();
        commit(&path, &valid, &settings).unwrap();
        git(&path, &["checkout", "--detach", &base], &settings).unwrap();
        std::fs::write(path.join("a.txt"), b"worker-conflict\n").unwrap();
        let result = record(&path, &settings, "worker");
        git(&path, &["checkout", "--detach", &base], &settings).unwrap();
        std::fs::write(path.join("a.txt"), b"previous-conflict\n").unwrap();
        let previous = record(&path, &settings, "previous");
        let error =
            merge_candidate(&directory, &previous, &result, "owned-agent", &settings).unwrap_err();
        assert_eq!(error.code(), "managed-candidate-merge-failed");
        assert_eq!(
            error.source().unwrap().downcast_ref::<E>().unwrap().code(),
            "managed-git-command"
        );
        assert_eq!(
            text(git(&path, &["rev-parse", "HEAD"], &settings).unwrap()).unwrap(),
            previous
        );
        assert!(!git(&path, &["ls-files", "--unmerged"], &settings)
            .unwrap()
            .is_empty());
        close(path, directory);
    }

    #[test]
    fn managed_candidate_invalid_identity_and_revision_refuse_before_checkout() {
        let (path, directory, settings, base) = fixture();
        let valid = merge_candidate(&directory, &base, &base, "owned-agent", &settings).unwrap();
        let before = text(git(&path, &["rev-parse", "HEAD"], &settings).unwrap()).unwrap();
        assert_eq!(before, valid);
        assert_eq!(
            merge_candidate(&directory, &base, &base, "../other", &settings)
                .unwrap_err()
                .code(),
            "managed-candidate-agent-invalid"
        );
        assert!(merge_candidate(&directory, "HEAD", &base, "owned-agent", &settings).is_err());
        assert!(merge_candidate(&directory, &base, "HEAD~0", "owned-agent", &settings).is_err());
        assert_eq!(
            text(git(&path, &["rev-parse", "HEAD"], &settings).unwrap()).unwrap(),
            before
        );
        close(path, directory);
    }
}
