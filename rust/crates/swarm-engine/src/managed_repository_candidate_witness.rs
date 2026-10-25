//! Owned candidate-custody witnesses that survive the managed lane close.
//!
//! [`CandidateWitness`] duplicates the copy, `.git` and workspace directory
//! descriptors of one freshly inspected candidate and binds them to a private
//! clone of the inspected [`ManagedCandidate`] record. While the witness
//! lives, the three recorded inodes cannot be reused by another allocation,
//! and the retained directories stay addressable through the held descriptors
//! even after their recorded paths are renamed, replaced or removed. The
//! parent keeps newly retained ownership outside generic cleanup wrappers;
//! it finalizes all enclosing lock/ancestor releases before returning a witness.
//! Any enclosing failure also explicitly releases the new witness, preserving
//! its close causes alongside the operation failure.
//!
//! Limits: holding a witness prevents inode reuse only. It is not same-UID OS
//! confinement, does not make a stale or replaced path executable or safe to
//! execute, and grants no Root permission, execution right, policy, retry or
//! acceptance. Waiting, execution, fresh-source revalidation and native
//! permissions remain separate parent/caller duties. Nothing here reopens a
//! path, clears CLOEXEC, implements `Drop`, retries or grants: the checked
//! release is [`CandidateWitness::close`], and dropping a witness without it
//! closes the descriptors through the standard unchecked file close. Every
//! operation uses the Linux-only `LocalDirectory` APIs, so this module
//! compiles only under the parent tree's `target_os = "linux"` gate.

use super::{E, LocalDirectory, ManagedCandidate};
use crate::agent_launch_error::finish;
use std::path::PathBuf;

/// Owned custody of one inspected candidate: the frozen record plus duplicated
/// copy, `.git` and workspace directory descriptors.
///
/// The duplicates are checked against the record's `(device, inode)` triplet
/// before retention succeeds and pin those inodes until every descriptor is
/// released. The witness is deliberately neither `Clone`, `Debug` nor
/// serializable; the record is readable through [`CandidateWitness::record`]
/// and the held descriptors have no other accessor.
#[must_use = "a retained witness owns three directory descriptors; call close() for the checked release"]
pub(crate) struct CandidateWitness {
    record: ManagedCandidate,
    copy: LocalDirectory,
    git: LocalDirectory,
    workspace: LocalDirectory,
}

impl CandidateWitness {
    /// Retain one inspected candidate under owned descriptors.
    ///
    /// Purpose: duplicate the caller's copy, `.git` and workspace descriptors
    /// and bind them to a private clone of `record`, so custody of the pinned
    /// inodes outlives the caller's managed lane and ancestor descriptors.
    ///
    /// Inputs: the inspected record and the three borrowed descriptors whose
    /// identities the record already pins — the same descriptors the parent's
    /// `inspect` opened and checked immediately before this call.
    ///
    /// Errors: duplication failures report
    /// `managed-candidate-witness-copy-duplicate`,
    /// `managed-candidate-witness-git-duplicate` or
    /// `managed-candidate-witness-workspace-duplicate`; per-handle metadata
    /// failures report `managed-candidate-witness-copy-identity`,
    /// `-git-identity` or `-workspace-identity`; any deviation of the
    /// duplicated triplet from the record refuses with
    /// `managed-candidate-witness-directory-replaced`. Every failure releases
    /// all duplicates owned at that point through the existing `finish`
    /// wrapper, preserving the primary and every cleanup close cause together.
    /// A duplication failure never closes a caller-owned handle: only
    /// duplicated descriptors are ever released here.
    ///
    /// Effects: on success the returned witness holds three new CLOEXEC
    /// descriptors and a private record clone; the caller's descriptors, the
    /// record and the filesystem are untouched. On failure no owned
    /// duplicate is handed to the checked close path; close errors remain errors.
    ///
    /// Limits: this compares the duplicated handles' metadata with the record
    /// only. It performs no path reopen, no freshness, Git or raw-source
    /// validation, no permission grant, no retry and no policy decision; the
    /// record is trusted exactly as far as the caller's `inspect` trusted it.
    pub(crate) fn retain(
        record: &ManagedCandidate,
        copy: &LocalDirectory,
        git: &LocalDirectory,
        workspace: &LocalDirectory,
    ) -> Result<Self, E> {
        let copy = copy
            .duplicate()
            .map_err(|e| E::caused("managed-candidate-witness-copy-duplicate", e))?;
        let git = match git.duplicate() {
            Ok(handle) => handle,
            Err(e) => {
                return finish(
                    Err(E::caused("managed-candidate-witness-git-duplicate", e)),
                    copy.close()
                        .map_err(|e| E::caused("managed-candidate-witness-copy-close", e)),
                );
            }
        };
        let workspace = match workspace.duplicate() {
            Ok(handle) => handle,
            Err(e) => {
                let primary = E::caused("managed-candidate-witness-workspace-duplicate", e);
                return finish(
                    finish(
                        Err(primary),
                        git.close()
                            .map_err(|e| E::caused("managed-candidate-witness-git-close", e)),
                    ),
                    copy.close()
                        .map_err(|e| E::caused("managed-candidate-witness-copy-close", e)),
                );
            }
        };
        let verified = verify_identities(record, &copy, &git, &workspace);
        if let Err(primary) = verified {
            return finish(Err(primary), release(copy, git, workspace));
        }
        Ok(Self {
            record: record.clone(),
            copy,
            git,
            workspace,
        })
    }

    /// Borrow the retained candidate record.
    ///
    /// Purpose: expose the frozen [`ManagedCandidate`] (binding digest,
    /// identity triplet, tracked sources, coordinates) to callers building
    /// commands or DTOs, without exposing the held descriptors.
    ///
    /// Inputs: none besides the witness. Errors: none. Effects: none.
    ///
    /// Limits: the record is the snapshot inspected at retention time; this
    /// never refreshes or revalidates it and grants nothing by itself.
    pub(crate) fn record(&self) -> &ManagedCandidate {
        &self.record
    }

    /// Return a path anchored to OUR held workspace descriptor.
    ///
    /// Purpose: supply a cwd/argument path for a command that must observe the
    /// pinned workspace inode while the witness lives, per the parent's
    /// `LocalDirectory::process_path` contract.
    ///
    /// Inputs: none besides the witness. Errors: none; this cannot fail.
    /// Effects: none beyond allocating the returned `PathBuf`.
    ///
    /// Limits: the path is anchored to this holder process. A child can resolve
    /// it while the holder lives, subject to normal procfs access. Keep the
    /// witness alive through the owned wait or helper exec; never persist or
    /// reuse the path after close. It is not the record's canonical workspace
    /// string. No execution permission, fresh-path check or confinement follows.
    pub(crate) fn workspace_path(&self) -> PathBuf {
        self.workspace.process_path()
    }

    /// Re-compare the held descriptors' actual identities with the record.
    ///
    /// Purpose: let a custodian confirm, at any point while the witness lives,
    /// that the three held inodes still match the inspected
    /// `(copy_identity, git_identity, workspace_identity)` triplet.
    ///
    /// Inputs: none besides the witness. Errors: metadata read failures keep
    /// their per-handle causes (`managed-candidate-witness-copy-identity`,
    /// `-git-identity`, `-workspace-identity`); any mismatch refuses with
    /// `managed-candidate-witness-directory-replaced`.
    ///
    /// Effects: none; purely observational.
    ///
    /// Limits: this compares held handle metadata only and is deliberately NOT
    /// a current-path freshness check: a different directory now living at the
    /// recorded path is invisible here, as are Git or raw-source changes,
    /// which remain the parent's rerun duty. It reruns nothing and grants
    /// nothing.
    pub(crate) fn verify_retained_identities(&self) -> Result<(), E> {
        verify_identities(&self.record, &self.copy, &self.git, &self.workspace)
    }

    /// Release every held descriptor and return the retained record.
    ///
    /// Purpose: the checked teardown used by the parent's prepare/verify paths
    /// (and later by the pin controller); custody ends exactly here.
    ///
    /// Inputs: the witness by value. Errors: close failures report
    /// `managed-candidate-witness-copy-close`, `-git-close` or
    /// `-workspace-close`; all three handles are attempted even when an
    /// earlier one fails, and every cause is preserved through the existing
    /// `finish` wrapper. The record is returned only when all closes
    /// succeeded.
    ///
    /// Effects: consumes the witness and closes all three descriptors; on
    /// failure all releases have been attempted and the record is not returned.
    /// An error never promises every close succeeded or that no other aliases
    /// exist; preserve and diagnose the actual close cause.
    ///
    /// Limits: this releases plain owned descriptors only — no unlock, path
    /// cleanup, permission change or retry. Dropping the witness without
    /// calling `close` releases the descriptors through the standard
    /// unchecked file close instead.
    pub(crate) fn close(self) -> Result<ManagedCandidate, E> {
        let Self {
            record,
            copy,
            git,
            workspace,
        } = self;
        release(copy, git, workspace).map(|()| record)
    }
}

/// Close the three owned duplicates, attempting every close and joining all
/// failures through `finish` (first as primary, later ones as cleanup). Only
/// descriptors this module duplicated are ever passed here.
fn release(copy: LocalDirectory, git: LocalDirectory, workspace: LocalDirectory) -> Result<(), E> {
    let copy = copy
        .close()
        .map_err(|e| E::caused("managed-candidate-witness-copy-close", e));
    let git = git
        .close()
        .map_err(|e| E::caused("managed-candidate-witness-git-close", e));
    let workspace = workspace
        .close()
        .map_err(|e| E::caused("managed-candidate-witness-workspace-close", e));
    finish(finish(copy, git), workspace)
}

/// Compare the duplicated handles' owned directory identities with the
/// record's private triplet. Metadata failures keep per-handle causes; any
/// difference refuses. Held descriptors only — no path is inspected.
fn verify_identities(
    record: &ManagedCandidate,
    copy: &LocalDirectory,
    git: &LocalDirectory,
    workspace: &LocalDirectory,
) -> Result<(), E> {
    if copy
        .owned_identity()
        .map_err(|e| E::caused("managed-candidate-witness-copy-identity", e))?
        != record.copy_identity
        || git
            .owned_identity()
            .map_err(|e| E::caused("managed-candidate-witness-git-identity", e))?
            != record.git_identity
        || workspace
            .owned_identity()
            .map_err(|e| E::caused("managed-candidate-witness-workspace-identity", e))?
            != record.workspace_identity
    {
        return Err(E::Refused("managed-candidate-witness-directory-replaced"));
    }
    Ok(())
}
