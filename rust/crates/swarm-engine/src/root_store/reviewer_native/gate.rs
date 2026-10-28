//! Kernel-observed reviewer child and grant admission, never agent authentication.
use super::{
    guard,
    material::Material,
    store::{self, Job, Phase},
};
use crate::{
    root_store::{finished, RootStore, RootStoreError as E},
    AgentLaunchError as A, EngineSettingsDocument,
};
use rusqlite::Transaction;
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::Path,
};

/// Keep original typed causes behind a safe gate boundary, without private text.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}
/// Reobserve a registered direct child/group without signalling it. PID alone is
/// insufficient: immutable parent, group and process-start stamp must match.
fn child(job: &Job) -> Result<(), E> {
    let pid = job
        .child_pid
        .ok_or(E::Refused("review-gate-child-required"))?;
    let actual = crate::observe_linux_process(
        pid.try_into()
            .map_err(|error| caused("review-gate-pid", error))?,
    )
    .map_err(|error| caused("review-gate-child-observation", error))?;
    if i64::from(actual.parent_pid()) != job.owner_pid
        || job.parent_pid != Some(job.owner_pid)
        || i64::from(actual.process_group_id()) != pid
        || job.group_id != Some(pid)
        || job.child_stamp.as_deref() != Some(actual.process_stamp())
    {
        return Err(E::Refused("review-gate-child-changed"));
    }
    Ok(())
}
/// Stage an actual owned child registration in caller TX after source/owner
/// rechecks. Caller must commit and retain the tuple before any permission stage.
/// No socket byte, exec, budget change or task acceptance occurs. Errors require
/// rollback; observation failure never yields an empty child or successful row.
#[cfg(test)]
pub(super) fn register(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    job: &Job,
    pid: u32,
) -> Result<Job, E> {
register_using(root,tx,settings,job,pid,None)
}
/// Source-aware stage under caller snapshot; custody stays external and all
/// owner/child/current-input checks precede the unchanged ledger transition.
pub(super) fn register_using(root:&RootStore,tx:&Transaction<'_>,settings:EngineSettingsDocument,job:&Job, pid: u32,source:Option<&crate::root_store::task_validation::ReviewCandidate>)->Result<Job,E>{
    guard::owner(job)?;
    guard::fresh_using(root,tx,settings,job,source)?;
    if job.phase != Phase::Claimed || job.permission != 0 {
        return Err(E::Refused("review-gate-registration-state"));
    }
    let actual = crate::observe_linux_process(pid)
        .map_err(|error| caused("review-gate-child-observation", error))?;
    let mut next = job.clone();
    next.phase = Phase::Registered;
    next.child_pid = Some(i64::from(pid));
    next.parent_pid = Some(i64::from(actual.parent_pid()));
    next.group_id = Some(i64::from(actual.process_group_id()));
    next.child_stamp = Some(actual.process_stamp().into());
    child(&next)?;
    store::change(tx, job, &next).map_err(|error| E::Launch(Box::new(error)))?;
    Ok(next)
}
/// Stage permission separately from registration after current owner/source/child
/// checks. Caller must successfully commit before releasing its private channel;
/// this method never sends a byte, launches a provider or accepts a task. Any
/// failure requires rollback and owned process cleanup, without refunding calls.
#[cfg(test)]
pub(super) fn permit(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    job: &Job,
) -> Result<Job, E> {
permit_using(root,tx,settings,job,None)
}
/// Source-aware stage under caller snapshot; custody stays external and all
/// owner/child/current-input checks precede the unchanged ledger transition.
pub(super) fn permit_using(root:&RootStore,tx:&Transaction<'_>,settings:EngineSettingsDocument,job:&Job,source:Option<&crate::root_store::task_validation::ReviewCandidate>)->Result<Job,E>{
    guard::owner(job)?;
    guard::fresh_using(root,tx,settings,job,source)?;
    child(job)?;
    if job.phase != Phase::Registered || job.permission != 0 {
        return Err(E::Refused("review-gate-permission-state"));
    }
    let mut next = job.clone();
    next.phase = Phase::Permitted;
    next.permission = 1;
    store::change(tx, job, &next).map_err(|error| E::Launch(Box::new(error)))?;
    Ok(next)
}
/// Consume one private protocol grant from the actual owner socket and rebuild
/// source observations for this registered OS child. Parent-death signal and
/// kernel peer UID/PID precede the grant read; start stamps, host, parent/group,
/// committed permission and original TTL precede returned material. Root/settings
/// are explicitly closed. Caller still owns channel closure and fresh transport
/// checks before any exec; returned data alone is not execution or confinement.
/// Missing/EOF/invalid grant or any uncertain identity refuses with typed causes.
/// Shared raw credential validation rejects PID0/invalid UID-GID before grant or Root use.
fn admit_with_job(root: &Path, id: &str, channel: &mut UnixStream) -> Result<(Material, Job), E> {
    if !swarm_identity_contract::name(id.as_bytes()) {
        return Err(E::Refused("review-gate-id-invalid"));
    }
    channel
        .peer_addr()
        .map_err(|error| caused("review-gate-channel-kind", error))?;
    crate::supervisor_identity::arm_parent_death_signal()
        .map_err(|error| caused("review-gate-parent-death", error))?;
    let peer = crate::linux_peer_credentials::read(&*channel)
        .map_err(|error| caused("review-gate-peer-credentials", error))?;
    if peer.uid() != rustix::process::geteuid().as_raw() {
        return Err(E::Refused("review-gate-peer-user-changed"));
    }
    let mut grant = [0u8; 1];
    channel
        .read_exact(&mut grant)
        .map_err(|error| caused("review-gate-input", error))?;
    if grant != [1] {
        return Err(E::Refused("review-gate-release-invalid"));
    }
    let store = RootStore::open(root, false)?;
    let result = (|| {
    let selected=store::job(&store.db,id).map_err(|e|E::Launch(Box::new(e)))?
        .ok_or(E::Refused("review-gate-job-required"))?;
    super::sources::job(&store,&selected,|source| {

        let lock = store.directory.reserve(false, false).map_err(E::Storage)?;
        let operation = (|| {
            let settings = store.read_settings()?;
            let tx = store.db.unchecked_transaction().map_err(E::Sql)?;
            let result = (|| {
                let job = store::job(&tx, id)
                    .map_err(|error| E::Launch(Box::new(error)))?
                    .ok_or(E::Refused("review-gate-job-required"))?;
                let actual = crate::SupervisorIdentity::current()
                    .map_err(|error| caused("review-gate-self-observation", error))?;
                if job.phase != Phase::Permitted
                    || job.permission != 1
                    || job.child_pid != Some(actual.pid())
                    || job.child_stamp.as_deref() != Some(actual.process_stamp())
                    || job.owner_host != actual.host()
                    || job.owner_pid != i64::from(peer.pid())
                {
                    return Err(E::Refused("review-gate-identity-or-permission"));
                }
                child(&job)?;
                let parent = crate::observe_linux_process(
                    job.owner_pid
                        .try_into()
                        .map_err(|error| caused("review-gate-parent-pid", error))?,
                )
                .map_err(|error| caused("review-gate-parent-observation", error))?;
                if parent.process_stamp() != job.owner_stamp {
                    return Err(E::Refused("review-gate-parent-changed"));
                }
                guard::fresh_using(&store, &tx, settings, &job,source).map(|material| (material, job))
            })();
            finished(result, tx.rollback().map_err(E::Sql))
        })();
        finished(operation, lock.close().map_err(E::Storage))
    })
    })();
    finished(result, store.close())
}

/// Test/controller observation of the actual private child admission. Root and
/// snapshots close before return. This does not exec or return a reusable permit;
/// the production transport must perform its own final freshness/TTL checks.
#[cfg(test)]
pub(super) fn admit(root: &Path, id: &str, channel: &mut UnixStream) -> Result<Material, E> {
    admit_with_job(root, id, channel).map(|(material, _)| material)
}
/// Consume the exact admitted private child by sealed-input exec. Requires a
/// CLOEXEC channel, kernel-owned peer/child/parent and committed grant. Root is
/// closed before transport, then original TTL and host/model are checked again.
/// Success never returns and cannot invoke twice in this process. Failures retain
/// typed causes and close sealed stdin; caller must report/close its channel and
/// exit. This private helper establishes no OS sandbox, model identity,
/// complete wait, durable result, deadline supervisor or task acceptance.
pub(super) fn execute(
    root: &Path,
    id: &str,
    channel: &mut UnixStream,
) -> Result<std::convert::Infallible, E> {
    let flags = rustix::io::fcntl_getfd(&*channel)
        .map_err(|error| caused("review-gate-channel-flags", error))?;
    if !flags.contains(rustix::io::FdFlags::CLOEXEC) {
        return Err(E::Refused("review-gate-channel-cloexec-required"));
    }
    let (material, job) = admit_with_job(root, id, channel)?;
    let transport = crate::native_provider_exec::PreparedExec::prepare(
        material.model.provider(),
        &material.host.environment_names,
        &material.host.workspace,
        material.inputs.prompt.as_bytes(),
    )?;
    let fresh = material
        .fresh(root)
        .map_err(|error| E::Launch(Box::new(error)))
        .and_then(|_| guard::original_fresh(&job.preflight_until));
    if let Err(error) = fresh {
        return finished(Err(error), transport.close());
    }
    // LIVE-UX telemetry: same opt-in and injection as worker launches, after
    // the final freshness checks; the ingest token's subject is `review:<id>`.
    let telemetry = match crate::launch_telemetry_environment(
        root,
        &material.settings.values,
        &format!("review:{id}"),
        &material.launch.provider,
        crate::telemetry_endpoint::now_ms(),
    ) {
        Ok(t) => t,
        Err(error) => return finished(Err(caused("review-telemetry-setup-failed", error)), transport.close()),
    };
    transport.add_environment(telemetry.environment)?.add_leading_arguments(telemetry.arguments)?.execute()
}

/// Own the private CLOEXEC channel through one exec attempt. Success never
/// returns. A pre-exec failure emits only the shared safe engine diagnostic,
/// then explicitly closes the channel, retaining primary/encoding/write/close
/// causes. Provider stdout cannot emit this frame; EOF alone is not exec proof.
/// No refund, restart, process completion or business acceptance is inferred.
pub(super) fn execute_reported(
    root: &Path,
    id: &str,
    mut channel: UnixStream,
) -> Result<std::convert::Infallible, E> {
    let error = match execute(root, id, &mut channel) {
        Ok(never) => match never {},
        Err(error) => error,
    };
    let report = serde_json::to_vec(&crate::agent_execution_gate::diagnostic(&error))
        .map_err(|cause| caused("review-gate-diagnostic-encode", cause))
        .and_then(|raw| {
            channel
                .write_all(&raw)
                .map_err(|cause| caused("review-gate-diagnostic-write", cause))
        });
    let report = finished(
        report,
        nix::unistd::close(channel).map_err(|cause| caused("review-gate-diagnostic-close", cause)),
    );
    finished(Err(error), report)
}

/// Apply one child transition through the existing saved storage policy. The
/// caller's mutable row changes only after known commit, before settings release,
/// so cleanup can retain a registered tuple even when lock release later fails.
/// Commit uncertainty leaves the prior observation, never a fabricated permit;
/// caller must preserve that uncertainty and reconcile actual storage. No grant
/// byte, process invocation, monetary reservation or Work revision is produced.
fn transition(
    root: &RootStore,
    job: &mut Job,
    prepare: impl Fn(&RootStore, &Transaction<'_>, EngineSettingsDocument, &Job,Option<&crate::root_store::task_validation::ReviewCandidate>) -> Result<Job, E>,
) -> Result<(), E> {
    let original=job.clone();
    super::sources::job(root,&original,|source|crate::agent_storage_retry::execute_borrowed(&root.db, &root.directory, false, |_, settings| {
        let next = super::transaction::apply(root, |tx| {
            prepare(root, tx, settings.clone(), job,source).map(super::transaction::Change::Mutate)
        })?;
        *job = next;
        Ok(())
    }))
}
/// Record this actual owned PID under fresh saved storage policy. A known child
/// commit survives later settings-release failure in the caller's job value.
/// No socket release or provider execution occurs; failure requires owned cleanup.
pub(super) fn register_recorded(root: &RootStore, job: &mut Job, pid: u32) -> Result<(), E> {
    transition(root, job, |root, tx, settings, job,source| {
        register_using(root, tx, settings, job, pid,source)
    })
}
/// Record permission separately from registration under fresh saved storage
/// policy, without writing the grant. Caller must handle any error before
/// channel release; uncertain commits are never retried as a provider call.
pub(super) fn permit_recorded(root: &RootStore, job: &mut Job) -> Result<(), E> {
    permit_recorded_using(root,job,None)
}
/// Recheck the actual owning lease inside the permission writer before the
/// existing transition. No socket grant, fresh budget or acceptance occurs;
/// commit uncertainty and actual caller job observations remain unchanged.
pub(super) fn permit_recorded_using(root:&RootStore,job:&mut Job,lease:Option<&crate::MissionLease>)->Result<(),E>{
    transition(root,job,|root,tx,settings,job,source|{
        super::mission_authority::permission(root,tx,lease,job,&settings).map_err(|e|E::Launch(Box::new(e)))?;
        permit_using(root,tx,settings,job,source)
    })
}
