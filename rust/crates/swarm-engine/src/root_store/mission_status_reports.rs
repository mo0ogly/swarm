//! Real active/terminal report discovery and Go fallback diagnostics. No
//! provider/process/SQL writes or acceptance. Reports are filenames for inspection,
//! never evidence of completion. No Go5000-entry silent search cutoff is copied.
use super::{finished, RootStoreError as E};
use crate::{Agent, AttemptDiagnostic, DiagnosticItem};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags};
use std::{
    error::Error,
    fs::File,
    os::{fd::IntoRawFd, unix::fs::MetadataExt},
    path::Path,
};

/// Preserve the original typed IO/encoding cause behind the safe existing error.
fn cause(code: &'static str, cause: impl Error + Send + Sync + 'static) -> E {
    E::Mutation(crate::WorkEngineError::Mission(Box::new(
        crate::MissionSupervisionError::Cause {
            code,
            cause: Box::new(cause),
        },
    )))
}
/// Close the owned directory explicitly, preserving primary and close causes.
fn close<T>(result: Result<T, E>, file: File) -> Result<T, E> {
    finished(
        result,
        nix::unistd::close(file.into_raw_fd())
            .map_err(|e| cause("mission-status-reports-close", e)),
    )
}
/// Require an actual opened directory, allowing every OS-readable owner/mode.
/// Project-private datastore ownership remains the outer Root guard's contract.
fn directory(file: &File) -> Result<std::fs::Metadata, E> {
    let metadata = file
        .metadata()
        .map_err(|e| cause("mission-status-reports-metadata", e))?;
    if !metadata.is_dir() {
        return Err(E::Refused("mission-status-reports-not-directory"));
    }
    Ok(metadata)
}
/// Walk descriptor-anchored directories without following directory symlinks.
/// Selected file symlinks use the shared contained regular-artifact reader, so
/// an OS-readable regular target inside Root is accepted regardless of owner.
/// All selected filenames/entry-mtimes are retained as Go DirEntry.Info does
/// (lstat for a symlink). Target metadata establishes regular containment only;
/// no contents are read or trusted and content-read permission is unnecessary.
/// Directory device/inode/mtime/ctime checks detect observed traversal changes;
/// they are sequential observations, not an atomic snapshot or ABA protection.
fn walk(
    root: &Path,
    file: &File,
    prefix: &str,
    task: &str,
    found: &mut Vec<(String, i128)>,
) -> Result<(), E> {
    let before = directory(file)?;
    let mut buffer = vec![
        std::mem::MaybeUninit::<u8>::uninit();
        usize::from(u16::MAX) + 1 + std::mem::align_of::<u64>()
    ];
    let mut reader = fs::RawDir::new(file, &mut buffer);
    while let Some(entry) = reader.next() {
        let entry = entry.map_err(|e| cause("mission-status-reports-read", e))?;
        let raw = entry.file_name().to_bytes();
        if matches!(raw, b"." | b"..") {
            continue;
        }
        let name = std::str::from_utf8(raw).map_err(|e| cause("mission-status-reports-name", e))?;
        let selected = name == format!("{task}.md")
            || (name.starts_with(&format!("{task}-"))
                && name.contains("handoff")
                && name.ends_with(".md"));
        let stat = fs::statat(file, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|e| cause("mission-status-reports-stat", e))?;
        let kind = FileType::from_raw_mode(stat.st_mode);
        let path = format!("{prefix}/{name}");
        if kind == FileType::Directory {
            let child: File = fs::openat(
                file,
                entry.file_name(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|e| cause("mission-status-reports-open-directory", e))?
            .into();
            let result = (|| {
                let actual = directory(&child)?;
                if actual.dev() != stat.st_dev || actual.ino() != stat.st_ino {
                    return Err(E::Refused("mission-status-reports-directory-changed"));
                }
                walk(root, &child, &path, task, found)?;
                let after = fs::statat(file, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(|e| cause("mission-status-reports-restat", e))?;
                if stat.st_dev != after.st_dev || stat.st_ino != after.st_ino {
                    return Err(E::Refused("mission-status-reports-directory-changed"));
                }
                Ok(())
            })();
            close(result, child)?;
        } else if selected {
            let (_, metadata) = crate::gate_evaluation::artifact_metadata(root, &path)
                .map_err(|cause| E::Gate(crate::GateContractError::Evaluation(cause)))?;
            let after = fs::statat(file, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|e| cause("mission-status-reports-restat", e))?;
            if (
                stat.st_dev,
                stat.st_ino,
                stat.st_mode,
                stat.st_mtime,
                stat.st_mtime_nsec,
                stat.st_ctime,
                stat.st_ctime_nsec,
            ) != (
                after.st_dev,
                after.st_ino,
                after.st_mode,
                after.st_mtime,
                after.st_mtime_nsec,
                after.st_ctime,
                after.st_ctime_nsec,
            ) {
                return Err(E::Refused("mission-status-reports-file-changed"));
            }
            // A regular source path and its checked target must name the same
            // inode. For a symlink, local_file supplies the containment check.
            if kind == FileType::RegularFile
                && (metadata.dev() != stat.st_dev || metadata.ino() != stat.st_ino)
            {
                return Err(E::Refused("mission-status-reports-file-changed"));
            }
            found.push((
                path,
                i128::from(stat.st_mtime) * 1_000_000_000 + i128::from(stat.st_mtime_nsec),
            ));
        }
    }
    let after = directory(file)?;
    let signature = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if signature(&before) != signature(&after) {
        return Err(E::Refused("mission-status-reports-directory-changed"));
    }
    Ok(())
}
/// Observe actual docs reports, newest mtime first with lexical tie order. ENOENT
/// for docs alone proves an empty listing. Contained regular file symlinks are
/// included; directory symlinks are never traversed. Directory/file owners need
/// not match the process. Permission, outside-Root/nonregular targets, child
/// races, IO and explicit close failures preserve their causes and refuse.
/// No directory is created and no report content or attribution is inferred.
pub(super) fn task_reports(root: &Path, task: &str) -> Result<Vec<String>, E> {
    if !swarm_identity_contract::name(task.as_bytes()) {
        return Err(E::Refused("mission-status-report-task-invalid"));
    }
    let path = root.join("docs");
    let file: File = match fs::open(
        &path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd.into(),
        Err(rustix::io::Errno::NOENT) => return Ok(Vec::new()),
        Err(e) => return Err(cause("mission-status-reports-open", e)),
    };
    let result = (|| {
        let before = directory(&file)?;
        let mut found = Vec::new();
        walk(root, &file, "docs", task, &mut found)?;
        let current = std::fs::symlink_metadata(&path)
            .map_err(|e| cause("mission-status-reports-path", e))?;
        if !current.is_dir() || before.dev() != current.dev() || before.ino() != current.ino() {
            return Err(E::Refused("mission-status-reports-directory-changed"));
        }
        found.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Ok(found.into_iter().map(|(path, _)| path).collect())
    })();
    close(result, file)
}
/// Observe Go provenReport over Root docs, not the worker's workspace. The
/// supplied nonnegative margin comes from persisted settings; it is not a local
/// fallback. Nonempty regular candidates at/after start-minus-margin are fresh;
/// exactly one is attributable, many/old/empty candidates retain Go's reasons.
/// Invalid start retains its typed cause beside the explicit unavailable result.
/// Metadata/containment/scan/close errors propagate, never become empty success.
/// No content, authorship, quality, mutation or permission is established; these
/// sequential filesystem observations are not an atomic snapshot or an ABA fence.
fn proven_report(root: &Path, agent: &Agent, skew: i64, causes: &mut Vec<E>)
    -> Result<crate::mission_status_tasks::ReportAttribution, E> {
    use crate::mission_status_tasks::ReportAttribution::{Attributed, Unavailable};
    if skew < 0 { return Err(E::Refused("mission-status-report-clock-skew-invalid")); }
    let started = match swarm_wire_json::go_rfc3339::parse(&agent.started) {
        Ok(value) => value.unix_timestamp_nanos(),
        Err(error) => {
            causes.push(cause("mission-status-report-start-invalid", error));
            return Ok(Unavailable { reason: "date de départ de la tentative illisible".into() });
        }
    };
    // i64 seconds and the parsed timestamp fit i128 nanoseconds; this is a unit
    // conversion, never a business cutoff or configurable second policy ceiling.
    let since = started - i128::from(skew) * 1_000_000_000;
    let mut fresh = Vec::new();
    let mut stale = 0usize;
    for name in task_reports(root, &agent.task_id)? {
        let (_, metadata) = crate::gate_evaluation::artifact_metadata(root, &name)
            .map_err(|e| E::Gate(crate::GateContractError::Evaluation(e)))?;
        if metadata.len() == 0 { continue; }
        let modified = i128::from(metadata.mtime()) * 1_000_000_000 + i128::from(metadata.mtime_nsec());
        if modified < since { stale += 1; } else { fresh.push(name); }
    }
    match fresh.as_slice() {
        [name] => Ok(Attributed(name.clone())),
        [] => Ok(Unavailable { reason: if stale > 0 {
            format!("aucun rapport produit par cette tentative ; {stale} rapport(s) antérieur(s) ignoré(s)")
        } else { "aucun rapport lisible et non vide sous docs/ pour cette tâche".into() } }),
        _ => Ok(Unavailable { reason: format!("plusieurs rapports candidats pour cette tentative ({})", fresh.join(", ")) }),
    }
}
/// Narrow ordinary completion shape whose attribution producer is wired here.
/// Incomplete-delivery and current-review branches keep their separate evidence
/// requirements. This read-only shape is not validation of the original process.
fn unsubmitted_pair(task: &crate::Task, agent: &Agent) -> bool {
    task.id == agent.task_id && task.status == "blocked" && agent.status == "completed"
        && !task.blocker.starts_with("Livraison incomplète :")
}
/// Go has no incomplete-report source outside managed planning. Restrict this
/// port to ordinary Work: no planning, completed matching Agent, explicit blocked
/// incomplete delivery. This pure selector grants no acceptance or launch.
fn ordinary_incomplete(work: &crate::Work, task: &crate::Task, agent: &Agent) -> bool {
    work.planning.is_none() && task.id == agent.task_id && agent.status == "completed"
        && task.status == "blocked" && task.blocker.starts_with("Livraison incomplète :")
}
/// Go operationText masking and terminal-cell clipping for fallback traces.
/// Sanitization and sensitivity masking precede the Go-compatible 600-cell
/// representation bound. Whole clusters are preserved; checked width overflow
/// retains its typed cause. Pure presentation, no execution/permission policy.
fn trace(reason: &str) -> Result<String, E> {
    let text = crate::agent_terminal_text::terminal_text(reason);
    let lower = text.to_lowercase();
    if [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "api-key",
        "authorization",
        "private key",
    ]
    .iter()
    .any(|m| lower.contains(m))
    {
        return Ok("[détail masqué : donnée potentiellement sensible]".into());
    }
    crate::terminal_clip::clip(&text, 600)
        .map_err(|error| cause("mission-status-diagnostic-width", error))
}
/// Go failureCategory over the actual retained terminal cause, never task prose.
fn category(reason: &str) -> &'static str {
    let text = reason.to_lowercase();
    if [
        "permission denied",
        "operation not permitted",
        "read-only file system",
        "sandbox denied",
        "sandbox violation",
        "mount failed",
        "bwrap: can't bind mount",
        "unable to apply mount flags",
        "mount: permission denied",
        "cifs error",
        "socket: operation not permitted",
        "network is unreachable",
    ]
    .iter()
    .any(|m| text.contains(m))
    {
        "environment"
    } else if [
        "go test",
        "pytest",
        "unittest",
        "npm test",
        "node --test",
        "diff --check",
        "lint",
        "check_",
        "check-",
        "validation",
    ]
    .iter()
    .any(|m| text.contains(m))
    {
        "check"
    } else if [
        "not configured",
        "non configur",
        "configuration",
        "executable",
        "exécutable",
        "command not found",
        "no such file or directory",
    ]
    .iter()
    .any(|m| text.contains(m))
    {
        "configuration"
    } else {
        "unknown"
    }
}
/// Construct the single real fallback item (no structured tool failure supplied).
/// The text/category is the Go template, not a new execution policy or diagnosis.
fn item(category: &str, reason: &str, trace: String) -> DiagnosticItem {
    let (label,cause,consequence,action,kind)=match category {
        "configuration"=>("Configuration","La configuration nécessaire au lancement ou à l’outil est absente, invalide ou indisponible.","L’étape concernée n’a pas pu démarrer ou s’exécuter ; son résultat n’est pas démontré.","Corriger la configuration indiquée, vérifier qu’elle est relue, puis demander explicitement une nouvelle tentative.","correct_configuration"),
        "environment"=>("Environnement d’exécution","Les traces signalent un refus d’accès ou une ressource indisponible dans l’environnement.","La tentative ne peut pas prouver le résultat dans cet environnement ; répéter sans changement de précondition risque le même échec.","Faire vérifier les droits et l’accès aux ressources sur la machine qui exécute l’agent ; reprendre après correction vérifiée.","verify_environment"),
        "check"=>("Contrôle en échec","Un test ou contrôle observable a échoué.","Le critère couvert par ce contrôle reste non validé.","Examiner la trace, corriger la cause, puis rejouer le même contrôle avec les mêmes options avant toute validation.","fix_and_rerun_check"),
        "limit"=>("Limite atteinte",reason,"Le superviseur a interrompu la tentative ; cette interruption ne valide ni le livrable ni la tâche.","Examiner les erreurs précédentes et les préconditions ; reprendre explicitement sans relever arbitrairement les protections.","review_limit"),
        _=>("Cause inconnue","La cause exacte n’est pas disponible dans les événements structurés de cette tentative.","Le résultat de la tentative reste indéterminé ; aucune reprise automatique sûre ne peut être déduite.","Ouvrir les traces conservées et établir la cause avant de choisir une reprise.","inspect_traces"),
    };
    DiagnosticItem {
        category: category.into(),
        label: label.into(),
        count: 1,
        cause: cause.into(),
        consequence: consequence.into(),
        action: action.into(),
        action_kind: kind.into(),
        unknown: category == "unknown",
        traces: Some(vec![trace]),
    }
}
/// Produce Go fallbackAttemptDiagnostic from actual stored terminal fields.
/// Existing structured diagnostics take precedence. Explicit operator interruption
/// produces Go's empty diagnostic value; completed Agents without structured
/// diagnostics also produce Go's empty value, regardless of activity prose.
/// Failed/nonoperator-interrupted causes receive the real fallback.
/// Fallback traces use whole-cluster cell clipping; width overflow retains its cause.
pub(super) fn terminal_diagnostics(agents: &[Agent]) -> Result<Vec<AttemptDiagnostic>, E> {
    agents
        .iter()
        .map(|agent| {
            if !matches!(
                agent.status.as_str(),
                "failed" | "interrupted" | "completed"
            ) {
                return Err(E::Refused("mission-status-agent-classification-unported"));
            }
            if agent
                .diagnostic
                .items
                .as_ref()
                .is_some_and(|v| !v.is_empty())
                || agent.diagnostic.observed_errors > 0
            {
                return Ok(agent.diagnostic.clone());
            }
            if agent.status == "completed"
                || (agent.status == "interrupted" && agent.stop_kind == "operateur")
            {
                return Ok(AttemptDiagnostic::default());
            }
            let mut reason = crate::text::trim_space(&agent.activity);
            if reason.is_empty() {
                reason = "Tentative terminée sans motif technique enregistré";
            }
            let lower = reason.to_lowercase();
            let limited = ["limite", "délai", "budget de temps"]
                .iter()
                .any(|m| lower.contains(m));
            let consecutive = limited && lower.contains("erreurs d'outils consécutives");
            let category = if limited { "limit" } else { category(reason) };
            let item = item(category, &format!("{reason}."), trace(reason)?);
            let cap = agent.limits.max_consecutive_errors;
            let mut summary = "Aucune erreur d’outil structurée n’a été observée.".to_string();
            if cap > 0 {
                summary.push_str(&format!(
                    " Le plafond configuré est de {cap} erreurs consécutives"
                ));
                summary.push_str(if consecutive {
                    " et il a interrompu la tentative."
                } else {
                    " ; ce plafond est distinct du nombre total constaté."
                });
            }
            if limited && !consecutive {
                summary.push_str(&format!(
                    " Une autre limite d’exécution a interrompu la tentative : {reason}."
                ));
            }
            Ok(AttemptDiagnostic {
                agent_id: agent.id.clone(),
                attempt_id: agent.attempt_id.clone(),
                observed_errors: 0,
                consecutive_error_limit: cap,
                limit_reached: limited,
                consecutive_error_limit_reached: consecutive,
                limit_reason: if limited {
                    reason.into()
                } else {
                    String::new()
                },
                summary,
                items: Some(vec![item]),
            })
        })
        .collect()
}
/// Go recoveryCategoryFor environment branch and its terminal-limit precedence.
/// Other recovery categories are irrelevant to the supported stopped result path.
pub(super) fn environment_recovery(agent: &Agent, diagnostic: &AttemptDiagnostic) -> bool {
    agent.provider_cooldown.is_none()
        && !(agent.stop_kind == "garde" && diagnostic.limit_reached)
        && diagnostic.items.as_ref().is_some_and(|items| {
            items
                .iter()
                .any(|item| matches!(item.category.as_str(), "environment" | "configuration"))
        })
}

/// Preserve complete row order while observing active diagnostics without a
/// terminal fallback. Real structured items/errors are retained; active activity
/// prose never fabricates a failure. Nonactive rows use the strict terminal
/// producer, whose unsupported-state errors and original causes propagate.
pub(super) fn diagnostics(agents: &[Agent]) -> Result<Vec<AttemptDiagnostic>, E> {
    agents
        .iter()
        .map(|agent| {
            if agent.is_active() {
                Ok(
                    if agent
                        .diagnostic
                        .items
                        .as_ref()
                        .is_some_and(|v| !v.is_empty())
                        || agent.diagnostic.observed_errors > 0
                    {
                        agent.diagnostic.clone()
                    } else {
                        AttemptDiagnostic::default()
                    },
                )
            } else {
                terminal_diagnostics(std::slice::from_ref(agent)).map(|mut values| values.remove(0))
            }
        })
        .collect()
}
/// Supported nonmanaged planned lifecycle pairs, never liveness or permission.
/// Full recorded history must agree with its task; Root separately validates
/// original rows, organization, supported review shape and current process observations.
/// No IO, mutation, hidden policy or fallback to success for unknown states.
fn planned_pair(task: &crate::Task, agent: &Agent) -> bool {
    task.id == agent.task_id && match task.status.as_str() {
        "running" => agent.is_active(),
        "blocked" => matches!(agent.status.as_str(),"failed" | "interrupted") || (agent.status=="completed" && (matching_review(task,agent) || unsubmitted_pair(task,agent))),
        "submitted" | "accepted" => agent.status == "completed",
        _ => false,
    }
}


/// A non-passed current opinion requires the real review display producer.
/// Go skips passed opinions before projecting them; skipping proves
/// neither validity nor acceptance. Actual accepted freshness independently
/// rechecks current native originals, gate, report and dependency evidence.
fn review_requires_projection(task: &crate::Task) -> bool {
    task.independent_review.as_deref().is_some_and(|review| review.state != "passed")
}

/// Match the actual current attempt/producer for a non-passed opinion. A
/// matched row is not yet current evidence; the observed producer checks it.
fn matching_review(task:&crate::Task,agent:&Agent)->bool {
    task.independent_review.as_deref().is_some_and(|review| super::mission_status_review::applies(true,task.attempts.as_ref().and_then(|v|v.last()).map(|a|a.id.as_str()),&agent.id,&agent.attempt_id,review))
}

/// Bound the newly wired review branch to nonmanaged planning and a completed
/// current producer on submitted/blocked task. Earlier reviews/recovery are
/// still refused separately; this shape check never validates its artifacts.
fn review_shape(work:&crate::Work,task:&crate::Task,agents:&[Agent])->bool {
    !review_requires_projection(task) || (work.planning.is_some() && matches!(task.status.as_str(),"submitted"|"blocked") && agents.iter().find(|a|a.task_id==task.id).is_some_and(|a|a.status=="completed" && matching_review(task,a)))
}

/// Require only branches whose result/readiness producers are implemented here.
/// Retained profiles are accepted for stopped or completed/submitted results
/// because Go does not consume dispatch readiness in those branches. All Agent
/// rows must be terminal or active on an ordinary running task; earlier terminal
/// rows remain history. Active classification must be observed separately; a
/// saved running status alone proves no live process.
/// Submitted results without a recent Agent use retained task/gate facts and
/// unknown process/no Agent identity. Last retained Task attempt remains visible;
/// no readiness is consumed or invented.
/// Abandoned decisions without a matching Agent also bypass readiness; they
/// remain nonvalidated, retaining the same review/recovery refusals. Accepted
/// tasks with completed or no selected Agent consume actual gate/freshness facts;
/// they retain unsupported/prior review and recovery refusals. No acceptance follows from status
/// or from a terminal process alone. Abandoned
/// tasks with an Agent and completed results outside submitted/accepted still
/// refuse. Unstarted profiles require the separate real readiness observation;
/// review/recovery history on those profiled tasks remains refused. This read
/// never changes the recorded decision or grants a launch. Current non-passed
/// review shapes use matching latest attempt/producer and the real review reader.
/// Ready/waiting planning trees support claimed/renewed leases, empty decisions and
/// initial todo/abandoned tasks without attempt/review/recovery history after structural
/// organization validation. Missing review remains non-current, never acceptance. Holder/date pairing and parsed
/// lease shape are checked, never treated as liveness or launch authority. Saved
/// counters at their cap remain readable. Submitted nonmanaged tasks may retain
/// completed Agents and their actual attempts, with no prior review/recovery history.
/// Final passed opinions may fall through; accepted tasks consume the actual
/// freshness/gate reader and never become validated from an opinion alone.
/// Running tasks may retain queued/starting/running/stopping Agents; blocked
/// tasks may retain failed/interrupted Agents. Actual process observation stays
/// mandatory before a live classification, never inferred from these rows.
/// Completed blocked ordinary or nonmanaged planned tasks may display actual Root-scoped
/// attribution without a current review. Ordinary incomplete delivery uses an
/// explicit absent report source, never Root report attribution. Ordinary abandoned
/// decisions retain terminal history without consuming report/review observations. Other unsubmitted
/// shapes, recovery and closed scopes still refuse here. Managed workers require
/// supported_observed with real request/proof/SQL verification by Root; this
/// entry point keeps them refused. Full Agent history is required;
/// callers must validate before applying display windows. Root, parent,
/// ownership and depth invariants come from structural validation, not scope count.
/// Original validation causes survive; configuration establishes no executed review.
/// No Agent state, review, recovery or permission is inferred from missing data.
pub(super) fn supported(work: &crate::Work, agents: &[Agent]) -> Result<(), E> {
    supported_observed(work, agents, false)
}
/// Check projection shapes after the caller has verified each managed worker's
/// retained request/proof/SQL row. This flag is an internal prerequisite, not an
/// operator policy or proof by itself. Only first verified running/blocked attempts
/// without review are supported; submitted/accepted/retry branches still refuse.
/// No IO, state change, liveness, attribution or permission follows from this check.
pub(super) fn supported_observed(work: &crate::Work, agents: &[Agent], managed_verified: bool) -> Result<(), E> {
    if agents.iter().any(|a| {
        a.is_active()
            && !work
                .tasks
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .any(|t| t.id == a.task_id && t.status == "running")
    }) {
        return Err(E::Refused("mission-status-active-task-mismatch"));
    }
    if let Some(planning) = work.planning.as_deref() {
        // Only the initial ready/waiting tree lifecycle is assembled. These shape
        // checks delimit the port, never a new execution policy or saved cap.
        let scopes = planning.scopes.as_deref().unwrap_or_default();
        if (planning.repository.is_some() && !agents.is_empty() && (!managed_verified ||
            agents.iter().any(|agent| !((agent.origin == "operateur" && agent.native_mission.is_none())
                    || (agent.origin == "conducteur" && agent.native_mission.is_some()))
                || !work.tasks.iter().flatten().any(|task| task.id == agent.task_id
                    && matches!(task.status.as_str(), "running" | "blocked")
                    && task.independent_review.is_none()
                    && task.attempts.as_deref().is_some_and(|attempts| attempts.len() == 1 && attempts[0].id == agent.attempt_id)))))
            || agents.iter().any(|agent|
                !work.tasks.iter().flatten().any(|task| planned_pair(task,agent)))
            || scopes.iter().any(|scope| !matches!(scope.state.as_str(), "ready" | "waiting"))
        {
            return Err(E::Refused("mission-status-planning-unported"));
        }
        if work.tasks.iter().flatten().any(|task|
            !matches!(task.status.as_str(),"todo" | "abandoned" | "submitted" | "accepted" | "running" | "blocked")
            || (matches!(task.status.as_str(),"submitted" | "accepted" | "running" | "blocked")
                && !agents.iter().any(|agent| planned_pair(task,agent)))
            || (!matches!(task.status.as_str(),"submitted" | "accepted" | "running" | "blocked")
                && task.attempts.as_ref().is_some_and(|v| !v.is_empty()))
            || !review_shape(work,task,agents)
            || task.previous_reviews.as_ref().is_some_and(|v| !v.is_empty())
            || task.corrective_recovery.is_some() || task.planning_retry
            || task.review_coordination.is_some() || task.batch_review_resume.is_some()
            || task.recovered_result.is_some())
        {
            return Err(E::Refused("mission-status-planning-initial-tasks-unported"));
        }
        for scope in scopes {
            if scope.holder.is_empty() != scope.lease_until.is_empty()
                || (!scope.holder.is_empty() && !swarm_identity_contract::name(scope.holder.as_bytes()))
            {
                return Err(E::Refused("mission-status-planning-lease-shape-invalid"));
            }
            if !scope.lease_until.is_empty() {
                crate::mission_supervision::parse(&scope.lease_until)
                    .map_err(|error| cause("mission-status-planning-lease-invalid", error))?;
            }
        }
        crate::validate_mission_organization(work).map_err(|cause| {
            E::Mutation(crate::WorkEngineError::Mission(Box::new(
                crate::MissionSupervisionError::Cause {
                    code: "mission-status-organization-invalid", cause: Box::new(cause),
                },
            )))
        })?;
    }
    if agents.iter().any(|a| {
        !matches!(
            a.status.as_str(),
            "failed" | "interrupted" | "completed" | "queued" | "starting" | "running" | "stopping"
        )
    }) {
        return Err(E::Refused("mission-status-agent-classification-unported"));
    }
    // Older terminal rows remain retained history. The authoritative reader is
    // latest-first; consuming task/result branches select their first matching
    // Agent, while every row still receives ownership/shape validation upstream.
    for task in work.tasks.as_deref().unwrap_or(&[]) {
        let agent = agents.iter().find(|a| a.task_id == task.id);
        if agent.is_some() || matches!(task.status.as_str(), "submitted" | "abandoned" | "accepted") {
            // A saved ordinary abandonment precedes Agent reports in Go.
            // Only terminal history is admitted; active/task mismatch still refuses.
            let ordinary_abandoned = work.planning.is_none() && task.status == "abandoned"
                && agent.is_some_and(|a| matches!(a.status.as_str(), "completed" | "failed" | "interrupted"));
            let supported_state = ordinary_abandoned || match agent.map(|a| a.status.as_str()) {
                Some("queued" | "starting" | "running" | "stopping") => task.status == "running",
                Some("completed") => matches!(task.status.as_str(), "submitted" | "accepted") || (task.status=="blocked" && agent.is_some_and(|a|matching_review(task,a) || unsubmitted_pair(task,a) || ordinary_incomplete(work,task,a))),
                Some("failed" | "interrupted") => task.status == "blocked",
                None => matches!(task.status.as_str(), "submitted" | "abandoned" | "accepted"),
                _ => false,
            };
            if !supported_state
                || !review_shape(work,task,agents)
                || task
                    .previous_reviews
                    .as_ref()
                    .is_some_and(|v| !v.is_empty())
                || task.corrective_recovery.is_some()
                || task.planning_retry
                || task.review_coordination.is_some()
                || task.batch_review_resume.is_some()
                || task.recovered_result.is_some()
            {
                return Err(E::Refused("mission-status-terminal-task-branch-unported"));
            }
        } else {
            if task.status != "todo" || task.attempts.as_ref().is_some_and(|v| !v.is_empty()) {
                return Err(E::Refused("mission-status-task-history-unported"));
            }
            if (task.launch_profile.is_some() || work.launch_profile.is_some())
                && (!review_shape(work,task,agents)
                    || task.previous_reviews.as_ref().is_some_and(|v| !v.is_empty())
                    || task.corrective_recovery.is_some() || task.planning_retry
                    || task.review_coordination.is_some() || task.batch_review_resume.is_some()
                    || task.recovered_result.is_some())
            {
                return Err(E::Refused("mission-status-profile-task-branch-unported"));
            }
        }
    }
    Ok(())
}
/// Check only projection prerequisites after full-history supported() succeeds.
/// A display window may hide completed submitted/accepted history, keeping its
/// identity unknown. Running/blocked states require a displayed matching Agent;
/// otherwise the existing task-history refusal prevents invented readiness.
/// No IO, mutation, policy/default or process/launch authority is established.
/// Full history is still required separately; this cannot replace eligibility.
pub(super) fn supported_view(work: &crate::Work, agents: &[Agent]) -> Result<(), E> {
    if work.tasks.iter().flatten().any(|task|
        !matches!(task.status.as_str(),"todo" | "submitted" | "accepted" | "abandoned")
        && !agents.iter().any(|agent| agent.task_id == task.id))
    {
        return Err(E::Refused("mission-status-task-history-unported"));
    }
    Ok(())
}
/// Borrow recorded Agent states and real diagnostics in identical row order.
/// Active rows require the caller to replace observed_state using fresh activity
/// observations before projection; stored status alone proves no liveness.
/// Every diagnostic must correspond positionally to its original complete row.
/// Terminal-limit/provider cooldown precedence follows recoveryCategoryFor.
pub(super) fn agent_observations<'a>(
    agents: &'a [Agent],
    diagnostics: &'a [AttemptDiagnostic],
) -> Result<Vec<crate::mission_status_tasks::AgentObservation<'a>>, E> {
    if agents.len() != diagnostics.len() {
        return Err(E::Refused(
            "mission-status-diagnostic-observations-mismatch",
        ));
    }
    Ok(agents
        .iter()
        .zip(diagnostics)
        .map(
            |(agent, diagnostic)| crate::mission_status_tasks::AgentObservation {
                agent,
                observed_state: &agent.status,
                fallback_diagnostic: diagnostic,
                environment_recovery: environment_recovery(agent, diagnostic),
            },
        )
        .collect())
}
/// Observe only applicable result facts, including current review when consumed. Complete Work,
/// settings and full-history/view eligibility must already be checked by Root.
/// Semantic causes stay in the supplied sidecar; facts causes are also retained.
/// Completed/blocked fallthrough obtains attribution from Root docs and the
/// saved clock margin only after an observed NotApplicable review. A verified
/// first verified managed worker instead retains explicit unavailable attribution:
/// its mutable copy is never inferred from Root reports or accepted content.
/// Managed reports retain PublicationUnavailable; actual managed review remains unsupported. Other branches
/// do not scan for attribution. No new authority, defaults, launch or mutation follows. Unknown read/close
/// errors refuse, and caller still verifies its enclosing material snapshot.
/// Ordinary incomplete delivery observes an absent incomplete-report source;
/// ordinary abandonment consumes neither report nor review facts. Both preserve
/// incoming gate/freshness and original causes, never grant execution/acceptance.
pub(super) fn observed_result_inputs(root:&Path,work:&crate::Work,task:&crate::Task,agent:Option<&Agent>,settings:&crate::EngineSettingsDocument,causes:&mut Vec<E>,mut facts:super::TaskFreshnessEvidence)->Result<crate::mission_status_tasks::ResultObservation,E> {
    causes.append(&mut facts.causes);
    // Contract: abandoned tasks without planning return NotObserved observations;
    // the existing decision consumer terminates on these statuses, so these fields stay unconsumed.
    if work.planning.is_none() && task.status == "abandoned" {
        return Ok(crate::mission_status_tasks::ResultObservation {
            freshness: facts.freshness,
            gate: facts.gate,
            reports: crate::mission_status_tasks::ReportListing::NotObserved,
            attribution: crate::mission_status_tasks::ReportAttribution::NotObserved,
            incomplete_report: crate::mission_status_tasks::IncompleteReport::NotObserved,
            review: crate::mission_status_tasks::ReviewObservation::NotObserved,
        });
    }
    if agent.is_some_and(|a| ordinary_incomplete(work, task, a)) {
        // Go returns no incompleteDeliveryReport outside managed planning and
        // projects this branch before review/ordinary report attribution.
        return Ok(crate::mission_status_tasks::ResultObservation {
            freshness: facts.freshness,
            gate: facts.gate,
            reports: crate::mission_status_tasks::ReportListing::NotObserved,
            attribution: crate::mission_status_tasks::ReportAttribution::NotObserved,
            incomplete_report: crate::mission_status_tasks::IncompleteReport::Observed(None),
            review: crate::mission_status_tasks::ReviewObservation::NotObserved,
        });
    }
    let managed = work.planning.as_deref().is_some_and(|p| p.repository.is_some());
    let review=match agent {
        Some(_) if managed => Some(crate::mission_status_tasks::ReviewObservation::NotApplicable),
        Some(agent) if task.status!="accepted"=>Some(super::mission_status_review::observe(root,work,task,agent,settings,causes)?),
        Some(_)=>Some(crate::mission_status_tasks::ReviewObservation::NotApplicable),
        None=>None,
    };
    let attribution = match agent {
        Some(agent) if managed && unsubmitted_pair(task, agent) =>
            Some(crate::mission_status_tasks::ReportAttribution::Unavailable {
                reason: "la publication d’un résultat Git managed attribuable n’est pas encore raccordée".into(),
            }),
        Some(agent) if unsubmitted_pair(task, agent)
            && matches!(&review, Some(crate::mission_status_tasks::ReviewObservation::NotApplicable)) => {
                Some(proven_report(root, agent, settings.values.report_clock_skew_seconds()
                    .map_err(E::Settings)?, causes)?)
            }
        _ => None,
    };
    let mut result = result_inputs_with_review((!managed).then_some(root),task,agent,facts.freshness,facts.gate,review,attribution)?;
    if managed { result.reports = crate::mission_status_tasks::ReportListing::PublicationUnavailable; }
    Ok(result)
}

/// Test-only constructor adapter: unsupported non-passed opinions still refuse
/// without a real producer. Supplied facts establish no actual Root evidence.
#[cfg(test)]
pub(super) fn result_inputs(root:&Path,task:&crate::Task,agent:Option<&Agent>,freshness:crate::mission_status_tasks::FreshnessObservation,gate:crate::mission_status_tasks::GateObservation)->Result<crate::mission_status_tasks::ResultObservation,E> {
    result_inputs_with_review(Some(root),task,agent,freshness,gate,None,None)
}

/// Observe only facts consumed by the applicable ordinary terminal result branch.
/// Accepted results consume the supplied current gate/freshness before any
/// report/review/readiness projection. This producer never accepts a task.
/// Active and failed/interrupted results use the real docs walk; early reports
/// remain unverified. Completed/submitted results
/// consume the retained submitted report path and do not scan docs or attribute a
/// fresh report; reports/attribution/incomplete remain explicitly NotObserved.
/// Callers must run supported() on full history: ordinary Work or the bounded
/// nonmanaged planned active, failed/interrupted or submitted branches admitted
/// there, including completed/blocked without incomplete delivery when actual
/// Root-scoped attribution was supplied. Display-window absence of an Agent cannot establish eligibility;
/// supported_view() must also establish the selected branch's required facts.
/// Current review is absent or passed before NotApplicable is supplied, matching
/// Go’s passed-opinion fallthrough. Previous opinions still refuse; non-passed
/// opinions require the actual supplied projection.
/// This is no verdict check. Accepted freshness separately checks native originals;
/// no IO or permission is inferred from skipping the review presentation.
/// None for root means no applicable Root report reader (managed copy); the
/// caller must attach PublicationUnavailable before a consuming projection.
fn result_inputs_with_review(
    root: Option<&Path>,
    task: &crate::Task,
    agent: Option<&Agent>,
    freshness: crate::mission_status_tasks::FreshnessObservation,
    gate: crate::mission_status_tasks::GateObservation,
    observed_review: Option<crate::mission_status_tasks::ReviewObservation>,
    attribution: Option<crate::mission_status_tasks::ReportAttribution>,
) -> Result<crate::mission_status_tasks::ResultObservation, E> {
    use crate::mission_status_tasks::*;
    if let Some(agent) = agent {
        if (review_requires_projection(task) && observed_review.is_none())
            || task
                .previous_reviews
                .as_ref()
                .is_some_and(|rows| !rows.is_empty())
        {
            return Err(E::Refused("mission-status-terminal-review-unported"));
        }
        let completed_supported = matches!(task.status.as_str(), "submitted" | "accepted")
            || (task.status=="blocked" && matches!(&observed_review,Some(ReviewObservation::Presented(_))))
            || (unsubmitted_pair(task, agent) && attribution.is_some());
        if agent.status == "completed" && !completed_supported {
            return Err(E::Refused("mission-status-completed-unsubmitted-unported"));
        }
    }
    Ok(ResultObservation {
        freshness,
        gate,
        reports: match root {
            Some(root) if agent.is_some_and(|agent| {
                agent.is_active() || matches!(agent.status.as_str(), "failed" | "interrupted")
            }) => ReportListing::Observed(task_reports(root, &task.id)?),
            _ => ReportListing::NotObserved,
        },
        attribution: attribution.unwrap_or(ReportAttribution::NotObserved),
        incomplete_report: IncompleteReport::NotObserved,
        review: observed_review.unwrap_or_else(|| if agent.is_some() {
            ReviewObservation::NotApplicable
        } else {
            ReviewObservation::NotObserved
        }),
    })
}
/// Observe selected-provider active cooldowns at the caller's observation time.
/// Uses the same strict descriptor reader as admission, with provider names sorted
/// and deduplicated as Go missionProviderCooldowns. Expiry and clearance exclude
/// a record; absence is an empty observation, never permission to launch. Invalid
/// records/read/close failures keep their typed launch cause. No writes/provider
/// execution occur. Repeated reads detect observed changes, not an atomic snapshot.
pub(super) fn active_cooldowns(
    root: &Path,
    work: &crate::Work,
    at: i64,
) -> Result<Vec<crate::ProviderCooldown>, E> {
    let mut names = std::collections::BTreeSet::new();
    if let Some(planning) = work.planning.as_deref() {
        if !planning.provider.is_empty() {
            names.insert(planning.provider.as_str());
        }
        if let Some(reviewer) = planning.reviewer.as_deref() {
            if !reviewer.provider.is_empty() {
                names.insert(reviewer.provider.as_str());
            }
        }
    }
    if let Some(profile) = work.launch_profile.as_deref() {
        if !profile.provider.is_empty() {
            names.insert(profile.provider.as_str());
        }
    }
    for task in work.tasks.as_deref().unwrap_or(&[]) {
        if !matches!(task.status.as_str(), "accepted" | "abandoned" | "waived") {
            if let Some(profile) = task.launch_profile.as_deref() {
                if !profile.provider.is_empty() {
                    names.insert(profile.provider.as_str());
                }
            }
        }
    }
    let mut active = Vec::new();
    for name in names {
        let observed = crate::agent_launch::with_provider_cooldown(root, name, Ok)
            .map_err(|e| E::Launch(Box::new(e)))?;
        if let Some(value) = observed {
            if value.cleared_at.is_empty() && (value.resets_at == 0 || at < value.resets_at) {
                active.push(value);
            }
        }
    }
    Ok(active)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    /// Accepted observations consume real freshness, never launch readiness.
    /// Review/recovery history stays refused; terminal process alone grants none.
    #[test]
    fn mission_status_accepted_guard_keeps_review_and_recovery_refusals() {
        let mut work = crate::Work { tasks: Some(vec![crate::Task { id:"task".into(),status:"submitted".into(),..Default::default() }]),..Default::default() };
        let agent = Agent { task_id:"task".into(),status:"completed".into(),..Default::default() };
        supported(&work,std::slice::from_ref(&agent)).unwrap();
        work.tasks.as_mut().unwrap()[0].status = "accepted".into();
        supported(&work,std::slice::from_ref(&agent)).unwrap();
        supported(&work,&[]).unwrap();
        // Pure observation probes do not forge a persisted accepted Work.
        use crate::mission_status_tasks::*;
        let task = &work.tasks.as_ref().unwrap()[0];
        let facts = result_inputs(Path::new("/unused-read-free-fixture"),task,None,
            FreshnessObservation::NotCurrent { blockers: vec!["Gate manquante".into()] },GateObservation::Absent).unwrap();
        assert_eq!(result_presentation(task,None,&facts).unwrap().state,"validation_stale");
        let contradictory = ResultObservation { freshness:FreshnessObservation::Current,..facts };
        assert!(result_presentation(task,None,&contradictory).is_err());
        work.tasks.as_mut().unwrap()[0].planning_retry = true;
        assert!(supported(&work,std::slice::from_ref(&agent)).is_err());
        work.tasks.as_mut().unwrap()[0].planning_retry = false;
        work.tasks.as_mut().unwrap()[0].independent_review = Some(Box::default());
        assert!(supported(&work,std::slice::from_ref(&agent)).is_err());
    }
    /// An admitted ordinary queued launch must remain observable, never validated.
    /// Existing unsupported review/recovery and terminal guards stay in force.
    #[test]
    fn mission_status_active_guard_requires_running_task_and_real_reports() {
        let mut work = crate::Work {
            id: "work".into(),
            tasks: Some(vec![crate::Task {
                id: "task".into(),
                status: "running".into(),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let agent = Agent {
            task_id: "task".into(),
            work_id: "work".into(),
            status: "queued".into(),
            ..Default::default()
        };
        supported(&work, std::slice::from_ref(&agent)).unwrap();
        work.tasks.as_mut().unwrap()[0].planning_retry = true;
        assert!(supported(&work, std::slice::from_ref(&agent)).is_err());
        work.tasks.as_mut().unwrap()[0].planning_retry = false;
        work.tasks.as_mut().unwrap()[0].status = "submitted".into();
        assert!(supported(&work, std::slice::from_ref(&agent)).is_err());
    }
    use super::*;
    use std::{
        fs as disk,
        os::unix::fs::{symlink, PermissionsExt},
        path::PathBuf,
    };
    /// Valid absent read precedes active-record observation: status must present
    /// the selected provider refusal without clearing it or starting a process.
    #[test]
    fn mission_status_selected_active_cooldown_is_observable() {
        let fixture = Fixture::new();
        let work = crate::Work {
            launch_profile: Some(Box::new(crate::LaunchProfile {
            provider: "fixture".into(),
            ..Default::default()
        })),
            ..crate::Work::default()
        };
        assert!(active_cooldowns(&fixture.0, &work, 0).unwrap().is_empty());
        let dir = fixture.0.join(".swarm/provider-cooldowns");
        disk::create_dir_all(&dir).unwrap();
        disk::set_permissions(&dir, disk::Permissions::from_mode(0o700)).unwrap();
        let path = dir.join("fixture.json");
        let value = crate::ProviderCooldown {
            provider: "fixture".into(),
            observed_at: "2026-10-23T00:00:00Z".into(),
            source: "fixture".into(),
            signal: "backoff".into(),
            ..Default::default()
        };
        let bytes = crate::encode_model(&value).unwrap();
        disk::write(&path, &bytes).unwrap();
        disk::set_permissions(&path, disk::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            active_cooldowns(&fixture.0, &work, 0).unwrap(),
            vec![value.clone()]
        );
        assert_eq!(
            crate::agent_launch::cooldown(&fixture.0, "fixture")
                .unwrap_err()
                .code(),
            "provider_cooldown"
        );
        assert_eq!(disk::read(&path).unwrap(), bytes);
        let mut expired = value.clone();
        expired.resets_at = 10;
        disk::write(&path, crate::encode_model(&expired).unwrap()).unwrap();
        assert_eq!(
            active_cooldowns(&fixture.0, &work, 9).unwrap(),
            vec![expired]
        );
        assert!(active_cooldowns(&fixture.0, &work, 10).unwrap().is_empty());
        let mut cleared = value;
        cleared.cleared_at = "2026-10-23T00:01:00Z".into();
        disk::write(&path, crate::encode_model(&cleared).unwrap()).unwrap();
        assert!(active_cooldowns(&fixture.0, &work, 0).unwrap().is_empty());
        crate::agent_launch::cooldown(&fixture.0, "fixture").unwrap();
        cleared.cleared_at = "PRIVATE-MALFORMED".into();
        disk::write(&path, crate::encode_model(&cleared).unwrap()).unwrap();
        let error = active_cooldowns(&fixture.0, &work, 0).unwrap_err();
        assert!(matches!(error, E::Launch(_)));
        assert!(!format!("{error:?}").contains("PRIVATE-MALFORMED"));
        disk::write(&path, bytes).unwrap();
        assert_eq!(active_cooldowns(&fixture.0, &work, 0).unwrap().len(), 1);
    }
    /// ASCII valid control precedes the three-em dash counterexample: Unicode
    /// U+2E3B occupies four cells but only three UTF8 bytes in Go uniseg15.
    #[test]
    fn mission_status_trace_clips_terminal_cells_not_utf8_bytes() {
        assert_eq!(trace(&"x".repeat(600)).unwrap(), "x".repeat(600));
        assert_eq!(
            trace(&"\u{2e3b}".repeat(200)).unwrap(),
            format!("{}…", "\u{2e3b}".repeat(149))
        );
        assert_eq!(trace(&"é".repeat(600)).unwrap(), "é".repeat(600));
    }
    /// Fresh owned filesystem fixture, retained on failure for diagnosis.
    struct Fixture(PathBuf);
    impl Fixture {
        /// Create only this test's attributed root; no Work/Agent authority seed.
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(crate::generate_identifier("status-reports-").unwrap());
            disk::create_dir(&path).unwrap();
            disk::set_permissions(&path, disk::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        /// Remove only owned successful data; retain failed fixture originals.
        fn drop(&mut self) {
            if std::thread::panicking() {
                eprintln!("reports fixture retained: {}", self.0.display());
            } else {
                disk::remove_dir_all(&self.0).unwrap();
            }
        }
    }
    /// Actual missing directory, ordinary nested files and ordering form a valid
    /// witness before true outside-Root symlink refusal and restored recovery.
    #[test]
    fn mission_status_reports_contained_scan_symlinks_refusal_recovery() {
        let fixture = Fixture::new();
        assert!(task_reports(&fixture.0, "task").unwrap().is_empty());
        disk::create_dir(fixture.0.join("docs")).unwrap();
        disk::create_dir(fixture.0.join("docs/nested")).unwrap();
        disk::write(fixture.0.join("docs/task.md"), "A real partial report").unwrap();
        disk::write(
            fixture.0.join("docs/nested/task-handoff.md"),
            "Another partial report",
        )
        .unwrap();
        disk::write(fixture.0.join("docs/other.md"), "Unrelated").unwrap();
        let oldest = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
        let newest = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(2);
        File::open(fixture.0.join("docs/task.md"))
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(oldest))
            .unwrap();
        File::open(fixture.0.join("docs/nested/task-handoff.md"))
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(newest))
            .unwrap();
        let expected = vec![
            "docs/nested/task-handoff.md".to_owned(),
            "docs/task.md".to_owned(),
        ];
        assert_eq!(task_reports(&fixture.0, "task").unwrap(), expected);
        // Go discovery uses Stat rather than reading report contents. A valid
        // listing must remain valid when the selected file denies content reads.
        disk::set_permissions(
            fixture.0.join("docs/task.md"),
            disk::Permissions::from_mode(0o000),
        )
        .unwrap();
        if !nix::unistd::geteuid().is_root() {
            assert_eq!(
                File::open(fixture.0.join("docs/task.md"))
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::PermissionDenied
            );
        }
        assert_eq!(task_reports(&fixture.0, "task").unwrap(), expected);
        disk::set_permissions(
            fixture.0.join("docs/task.md"),
            disk::Permissions::from_mode(0o600),
        )
        .unwrap();
        disk::write(fixture.0.join("inside.md"), "Contained target is a report").unwrap();
        File::open(fixture.0.join("inside.md"))
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
            .unwrap();
        disk::set_permissions(
            fixture.0.join("inside.md"),
            disk::Permissions::from_mode(0o000),
        )
        .unwrap();
        symlink("../inside.md", fixture.0.join("docs/task-handoff-link.md")).unwrap();
        // The newly created link sorts ahead of 1970-dated regular entries,
        // despite its target having an older mtime and no content read permission.
        let linked = vec![
            "docs/task-handoff-link.md".to_owned(),
            "docs/nested/task-handoff.md".to_owned(),
            "docs/task.md".to_owned(),
        ];
        assert_eq!(task_reports(&fixture.0, "task").unwrap(), linked);
        // A symlinked directory is not traversed, even when its target is inside.
        symlink("nested", fixture.0.join("docs/linked-dir")).unwrap();
        assert_eq!(task_reports(&fixture.0, "task").unwrap(), linked);
        let outside = Fixture::new();
        disk::write(outside.0.join("report.md"), "Outside project").unwrap();
        disk::remove_file(fixture.0.join("docs/task-handoff-link.md")).unwrap();
        symlink(
            outside.0.join("report.md"),
            fixture.0.join("docs/task-handoff-link.md"),
        )
        .unwrap();
        let error = task_reports(&fixture.0, "task")
            .expect_err("outside project must refuse");
        let E::Gate(crate::GateContractError::Evaluation(error)) = error else {
            panic!("outside path must preserve Gate cause")
        };
        assert_eq!(error.reason(), "artifact-path");
        assert!(super::super::mission_status_evidence::artifact_cause_matches(&error, |cause| matches!(
            cause.downcast_ref::<crate::LocalPathError>(),
            Some(crate::LocalPathError::Refused(crate::PathRefusal::OutsideProject))
        )));
        disk::remove_file(fixture.0.join("docs/task-handoff-link.md")).unwrap();
        symlink("../inside.md", fixture.0.join("docs/task-handoff-link.md")).unwrap();
        assert_eq!(task_reports(&fixture.0, "task").unwrap(), linked);
        disk::remove_file(fixture.0.join("docs/task-handoff-link.md")).unwrap();
        assert_eq!(task_reports(&fixture.0, "task").unwrap(), expected);
        disk::remove_file(fixture.0.join("docs/task.md")).unwrap();
        disk::create_dir(fixture.0.join("docs/task.md")).unwrap();
        // A directory matching the filename is traversed, never a report.
        assert_eq!(
            task_reports(&fixture.0, "task").unwrap(),
            vec!["docs/nested/task-handoff.md"]
        );
        assert!(matches!(
            task_reports(&fixture.0, "../task"),
            Err(E::Refused("mission-status-report-task-invalid"))
        ));
    }
    /// A valid scan precedes an OS permission failure; only ENOENT may become
    /// empty. Root's bypass privilege cannot establish the permission assertion.
    #[test]
    fn mission_status_reports_permission_refusal_preserves_cause_and_recovery() {
        if nix::unistd::geteuid().is_root() {
            return;
        }
        let fixture = Fixture::new();
        disk::create_dir(fixture.0.join("docs")).unwrap();
        disk::write(fixture.0.join("docs/task.md"), "partial").unwrap();
        assert_eq!(
            task_reports(&fixture.0, "task").unwrap(),
            vec!["docs/task.md"]
        );
        disk::set_permissions(fixture.0.join("docs"), disk::Permissions::from_mode(0o000)).unwrap();
        let failed = task_reports(&fixture.0, "task");
        disk::set_permissions(fixture.0.join("docs"), disk::Permissions::from_mode(0o700)).unwrap();
        let error = failed
            .expect_err("permission denied must not become empty");
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(
            task_reports(&fixture.0, "task").unwrap(),
            vec!["docs/task.md"]
        );
    }
    /// Go terminal fallback preserves structured observations, explicit operator
    /// interruption emptiness, environment/check/limit categories and redaction.
    /// These supplied protocol fixtures prove no real process or public admission.
    #[test]
    fn mission_status_reports_terminal_diagnostic_real_fallback_and_refusals() {
        let mut agent = Agent {
            id: "agent".into(),
            attempt_id: "attempt".into(),
            status: "interrupted".into(),
            stop_kind: "operateur".into(),
            ..Default::default()
        };
        assert_eq!(
            terminal_diagnostics(&[agent.clone()]).unwrap()[0],
            AttemptDiagnostic::default()
        );
        agent.stop_kind = String::new();
        agent.status = "failed".into();
        agent.activity = "permission denied".into();
        let diagnostic = terminal_diagnostics(&[agent.clone()]).unwrap().remove(0);
        assert_eq!(
            diagnostic.items.as_ref().unwrap()[0].category,
            "environment"
        );
        assert!(environment_recovery(&agent, &diagnostic));
        assert_eq!(diagnostic.observed_errors, 0);
        agent.activity = "go test failed".into();
        assert_eq!(
            terminal_diagnostics(&[agent.clone()]).unwrap()[0]
                .items
                .as_ref()
                .unwrap()[0]
                .category,
            "check"
        );
        agent.activity = "limite erreurs d'outils consécutives".into();
        agent.limits.max_consecutive_errors = 4;
        let diagnostic = terminal_diagnostics(&[agent.clone()]).unwrap().remove(0);
        assert!(diagnostic.limit_reached && diagnostic.consecutive_error_limit_reached);
        assert_eq!(diagnostic.consecutive_error_limit, 4);
        agent.activity = "secret value".into();
        assert_eq!(
            terminal_diagnostics(&[agent.clone()]).unwrap()[0]
                .items
                .as_ref()
                .unwrap()[0]
                .traces
                .as_ref()
                .unwrap()[0],
            "[détail masqué : donnée potentiellement sensible]"
        );
        agent.diagnostic = diagnostic.clone();
        assert_eq!(
            terminal_diagnostics(&[agent.clone()]).unwrap()[0],
            diagnostic
        );
        agent.diagnostic = AttemptDiagnostic::default();
        agent.activity = "x".repeat(601);
        let clipped = terminal_diagnostics(&[agent.clone()]).unwrap();
        assert_eq!(
            clipped[0].items.as_ref().unwrap()[0]
                .traces
                .as_ref()
                .unwrap()[0],
            format!("{}…", "x".repeat(599))
        );
        agent.activity = "short valid reason".into();
        assert!(terminal_diagnostics(&[agent.clone()]).is_ok());
        agent.status = "running".into();
        assert!(terminal_diagnostics(&[agent]).is_err());
    }
    /// Supplied protocol observations exercise the completed/submitted branch,
    /// retaining older failed/completed history and proving that unused docs/attribution/
    /// readiness inputs are not consumed. This pure fixture grants no Agent
    /// identity, report authenticity or acceptance; the public CLI recipe supplies
    /// the real prepare/run/submit witness without a paid provider or SQL seed.
    #[test]
    fn mission_status_reports_completed_submitted_uses_retained_report_without_scan() {
        use crate::mission_status_tasks::*;
        let fixture = Fixture::new();
        disk::create_dir(fixture.0.join("docs")).unwrap();
        disk::write(fixture.0.join("docs/task.md"), "actual scanner control").unwrap();
        assert_eq!(
            task_reports(&fixture.0, "task").unwrap(),
            vec!["docs/task.md"]
        );
        let task = crate::Task {
            id: "task".into(),
            title: "Completed work".into(),
            status: "submitted".into(),
            next: "handoff : handoff.md".into(),
            ..Default::default()
        };
        let mut work = crate::Work {
            id: "work".into(),
            tasks: Some(vec![task]),
            ..Default::default()
        };
        let mut agents = vec![
            Agent {
                id: "current".into(),
                work_id: "work".into(),
                task_id: "task".into(),
                attempt_id: "current-attempt".into(),
                status: "completed".into(),
                activity: "x".repeat(601),
                ..Default::default()
            },
            Agent {
                id: "previous".into(),
                work_id: "work".into(),
                task_id: "task".into(),
                attempt_id: "previous-attempt".into(),
                status: "failed".into(),
                activity: "earlier real terminal cause".into(),
                ..Default::default()
            },
        ];
        supported(&work, &agents).unwrap();
        let diagnostics = terminal_diagnostics(&agents).unwrap();
        // Go does not interpret completed activity as a failure fallback.
        assert_eq!(diagnostics[0], AttemptDiagnostic::default());
        assert!(!diagnostics[1].items.as_ref().unwrap().is_empty());
        let observations = agent_observations(&agents, &diagnostics).unwrap();
        disk::set_permissions(fixture.0.join("docs"), disk::Permissions::from_mode(0o000)).unwrap();
        if !nix::unistd::geteuid().is_root() {
            assert!(task_reports(&fixture.0, "task").is_err());
        }
        let facts = result_inputs(
            &fixture.0,
            &work.tasks.as_ref().unwrap()[0],
            Some(&agents[0]),
            FreshnessObservation::NotCurrent {
                blockers: Vec::new(),
            },
            GateObservation::Absent,
        )
        .unwrap();
        disk::set_permissions(fixture.0.join("docs"), disk::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(&facts.reports, ReportListing::NotObserved));
        assert!(matches!(&facts.attribution, ReportAttribution::NotObserved));
        assert!(matches!(
            &facts.incomplete_report,
            IncompleteReport::NotObserved
        ));
        assert!(matches!(&facts.review, ReviewObservation::NotApplicable));
        let projected = project_tasks(
            &work,
            &crate::mission_status_contract::tests::status(),
            &[TaskObservation {
                task_id: "task".into(),
                result: facts,
                dependencies: Vec::new(),
                readiness: ReadinessObservation::NotObserved,
                prepared_launch: PreparedLaunchObservation::Absent,
                execution: ExecutionObservation::NotObserved,
            }],
            &observations,
        )
        .unwrap();
        assert_eq!(projected.review, 1);
        assert_eq!(projected.validated, 0);
        assert_eq!(projected.running, 0);
        let task = &projected.tasks[0];
        assert_eq!(task.state, "review");
        assert_eq!(task.action, "report");
        assert_eq!(task.result.state, "result_to_review");
        assert_eq!(task.result.process_state, "completed");
        assert_eq!(task.result.report_state, "submitted");
        assert_eq!(task.result.validation_state, "missing_gate");
        assert_eq!(task.result.report_id, "handoff.md");
        assert_eq!(task.result.attempt_id, "current-attempt");
        assert!(task.diagnostic.is_none());
        // Existing structured observations must remain visible even on success.
        agents[0].diagnostic = diagnostics[1].clone();
        assert_eq!(terminal_diagnostics(&agents).unwrap()[0], diagnostics[1]);
        agents[0].status = "running".into();
        assert!(supported(&work, &agents).is_err());
        agents[0].status = "completed".into();
        supported(&work, &agents).unwrap();
        work.tasks.as_mut().unwrap()[0].status = "blocked".into();
        // Ordinary completion now has its separate observed attribution reader.
        supported(&work, &agents).unwrap();
        work.tasks.as_mut().unwrap()[0].status = "submitted".into();
        supported(&work, &agents).unwrap();
        work.tasks.as_mut().unwrap()[0].independent_review = Some(Box::default());
        assert!(supported(&work, &agents).is_err());
        assert!(result_inputs(
            &fixture.0,
            &work.tasks.as_ref().unwrap()[0],
            Some(&agents[0]),
            FreshnessObservation::NotObserved,
            GateObservation::NotObserved
        )
        .is_err());
        work.tasks.as_mut().unwrap()[0].independent_review = None;
        supported(&work, &agents).unwrap();
        work.tasks.as_mut().unwrap()[0].previous_reviews = Some(vec![Default::default()]);
        assert!(supported(&work, &agents).is_err());
        work.tasks.as_mut().unwrap()[0].previous_reviews = None;
        supported(&work, &agents).unwrap();
        agents[1].status = "completed".into();
        supported(&work, &agents).unwrap();
        // Rebuild actual pure result inputs, selecting the first matching Agent;
        // old completed activity must not replace the newest report or identity.
        let project = |work: &crate::Work, agents: &[Agent]| {
            let diagnostics = terminal_diagnostics(agents).unwrap();
            let observations = agent_observations(agents, &diagnostics).unwrap();
            let facts = result_inputs(
                &fixture.0,
                &work.tasks.as_ref().unwrap()[0],
                agents.iter().find(|a| a.task_id == "task"),
                FreshnessObservation::NotCurrent {
                    blockers: Vec::new(),
                },
                GateObservation::Absent,
            )
            .unwrap();
            project_tasks(
                work,
                &crate::mission_status_contract::tests::status(),
                &[TaskObservation {
                    task_id: "task".into(),
                    result: facts,
                    dependencies: Vec::new(),
                    readiness: ReadinessObservation::NotObserved,
                    prepared_launch: PreparedLaunchObservation::Absent,
                    execution: ExecutionObservation::NotObserved,
                }],
                &observations,
            )
            .unwrap()
        };
        // Submitted results do not consume launch readiness. Hiding their
        // Agent must preserve review/ledger while making process identity unknown.
        let mut window_work = work.clone();
        let mut recent_task = window_work.tasks.as_ref().unwrap()[0].clone();
        recent_task.id = "recent-task".into();
        window_work.tasks.as_mut().unwrap().push(recent_task);
        let mut window_agents: Vec<_> = (0..200)
            .map(|index| {
                let mut agent = agents[0].clone();
                agent.id = format!("recent-{index}");
                agent.task_id = "recent-task".into();
                agent.attempt_id = format!("recent-attempt-{index}");
                agent
            })
            .collect();
        window_agents.push(agents[0].clone());
        supported(&window_work, &window_agents).unwrap();
        supported(&window_work, &window_agents[..200]).unwrap();
        supported(&window_work, &window_agents[..0]).unwrap();
        supported(&window_work, &window_agents).unwrap();
        supported(&work, &[]).unwrap();
        let hidden = project(&work, &[]);
        assert_eq!(hidden.tasks[0].state, "review");
        assert_eq!(hidden.tasks[0].result.state, "result_to_review");
        assert_eq!(hidden.tasks[0].result.process_state, "unknown");
        assert!(hidden.tasks[0].attempt.agent_id.is_empty());
        assert!(hidden.tasks[0].attempt.attempt_id.is_empty());
        assert_eq!(hidden.validated, 0);
        let mut refused = work.clone();
        refused.tasks.as_mut().unwrap()[0].planning_retry = true;
        assert!(supported(&refused, &[]).is_err());
        refused.tasks.as_mut().unwrap()[0].planning_retry = false;
        refused.tasks.as_mut().unwrap()[0].independent_review = Some(Box::default());
        assert!(supported(&refused, &[]).is_err());
        refused.tasks.as_mut().unwrap()[0].independent_review = None;
        refused.tasks.as_mut().unwrap()[0].status = "blocked".into();
        assert!(supported(&refused, &[]).is_err());
        refused.tasks.as_mut().unwrap()[0].status = "submitted".into();
        supported(&refused, &[]).unwrap();
        let completed = project(&work, &agents);
        assert_eq!(completed.tasks[0].attempt.agent_id, "current");
        assert_eq!(completed.tasks[0].attempt.attempt_id, "current-attempt");
        assert_eq!(completed.tasks[0].result.state, "result_to_review");
        assert_eq!(completed.tasks[0].result.process_state, "completed");
        assert_eq!(completed.validated, 0);
        assert_eq!(agents[1].id, "previous");
        assert_eq!(agents[1].status, "completed");
        work.tasks.as_mut().unwrap()[0].status = "blocked".into();
        agents[0].status = "failed".into();
        supported(&work, &agents).unwrap();
        let failed = project(&work, &agents);
        assert_eq!(failed.tasks[0].attempt.agent_id, "current");
        assert_eq!(failed.tasks[0].result.process_state, "failed");
        assert_ne!(failed.tasks[0].result.state, "result_to_review");
        assert_eq!(failed.validated, 0);
        agents[1].status = "running".into();
        assert!(supported(&work, &agents).is_err());
        agents[1].status = "completed".into();
        supported(&work, &agents).unwrap();
        agents[0].status = "completed".into();
        supported(&work, &agents).unwrap();
        work.tasks.as_mut().unwrap()[0].status = "submitted".into();
        supported(&work, &agents).unwrap();
        agents[1].status = "failed".into();
        supported(&work, &agents).unwrap();
    }
    /// Submitted control precedes ordinary completed/blocked attribution eligibility.
    /// Ordinary incomplete delivery is admitted; recovery/review remain refused. This pure
    /// admission test grants no launch or acceptance and never reads runtime state.
    #[test]
    fn mission_status_ordinary_completed_blocked_admission() {
        let mut work = crate::Work::default();
        let mut task = crate::Task { id: "task".into(), status: "submitted".into(), ..Default::default() };
        let agent = Agent { task_id: "task".into(), status: "completed".into(), ..Default::default() };
        work.tasks = Some(vec![task.clone()]);
        supported(&work, std::slice::from_ref(&agent)).unwrap();
        task.status = "blocked".into();
        work.tasks = Some(vec![task.clone()]);
        supported(&work, std::slice::from_ref(&agent)).unwrap();
        task.blocker = "Livraison incomplète : missing criterion".into();
        work.tasks = Some(vec![task.clone()]);
        supported(&work, std::slice::from_ref(&agent)).unwrap();
        task.blocker.clear(); task.planning_retry = true;
        work.tasks = Some(vec![task.clone()]);
        assert!(supported(&work, std::slice::from_ref(&agent)).is_err());
        task.planning_retry = false; task.independent_review = Some(Box::default());
        work.tasks = Some(vec![task.clone()]);
        assert!(supported(&work, std::slice::from_ref(&agent)).is_err());
        task.independent_review = None; work.tasks = Some(vec![task]);
        supported(&work, std::slice::from_ref(&agent)).unwrap();
        assert_eq!(agent.status, "completed");
    }
    /// Incomplete ordinary delivery has no applicable report source in Go.
    /// A nonexistent Root proves this branch does not scan or attribute docs;
    /// its explicit absent report is consumed before review, without validation.
    #[test]
    fn mission_status_ordinary_incomplete_observation() {
        use crate::mission_status_tasks::*;
        let fixture = Fixture::new();
        let root = fixture.0.join("absent-root");
        let task = crate::Task { id: "task".into(), status: "blocked".into(),
            blocker: "Livraison incomplète : missing evidence".into(), ..Default::default() };
        let work = crate::Work { tasks: Some(vec![task.clone()]), ..Default::default() };
        let agent = Agent { task_id: "task".into(), status: "completed".into(), ..Default::default() };
        supported(&work, std::slice::from_ref(&agent)).unwrap();
        let settings = crate::EngineSettingsDocument { schema_version: 1, revision: crate::SettingCount(0),
            values: crate::EngineSettingsValues::installation_defaults().unwrap(), history: Vec::new() };
        let mut causes = Vec::new();
        let facts = observed_result_inputs(&root, &work, &task, Some(&agent), &settings, &mut causes,
            super::super::TaskFreshnessEvidence { freshness: FreshnessObservation::NotObserved,
                gate: GateObservation::NotObserved, causes: Vec::new() }).unwrap();
        assert!(matches!(facts.incomplete_report, IncompleteReport::Observed(None)));
        assert!(matches!(facts.review, ReviewObservation::NotObserved));
        let diagnostic = crate::AttemptDiagnostic::default();
        let observation = AgentObservation { agent: &agent, observed_state: "completed",
            fallback_diagnostic: &diagnostic, environment_recovery: false };
        let result = result_presentation(&task, Some(&observation), &facts).unwrap();
        assert_eq!(result.state, "delivery_incomplete");
        assert_eq!(result.validation_state, "not_validated");
        assert_eq!(result.report_state, "absent");
        assert!(causes.is_empty());
    }
    /// Ordinary abandoned decisions keep terminal Agent history without granting
    /// validation. Active mismatches, planning and prior opinions remain refused.
    #[test]
    fn mission_status_ordinary_abandoned_admission() {
        let mut work = crate::Work { tasks: Some(vec![crate::Task {
            id: "task".into(), status: "abandoned".into(), ..Default::default()
        }]), ..Default::default() };
        let mut agent = Agent { task_id: "task".into(), ..Default::default() };
        supported(&work, &[]).unwrap();
        for state in ["completed", "failed", "interrupted"] {
            agent.status = state.into();
            supported(&work, std::slice::from_ref(&agent)).unwrap();
        }
        agent.status = "running".into();
        assert!(supported(&work, std::slice::from_ref(&agent)).is_err());
        agent.status = "completed".into();
        work.tasks.as_mut().unwrap()[0].previous_reviews = Some(vec![Default::default()]);
        assert!(supported(&work, std::slice::from_ref(&agent)).is_err());
        work.tasks.as_mut().unwrap()[0].previous_reviews = None;
        supported(&work, std::slice::from_ref(&agent)).unwrap();
        work.planning = Some(Box::default());
        assert!(supported(&work, std::slice::from_ref(&agent)).is_err());
    }
    /// Real filesystem control precedes stale/empty/ambiguous/invalid variants.
    /// Boundary equality uses an explicit test margin, never a product default.
    #[test]
    fn mission_status_unsubmitted_report_selection_and_recovery() {
        use crate::mission_status_tasks::ReportAttribution::{Attributed, Unavailable};
        let fixture = Fixture::new();
        let mut agent = Agent { task_id: "task".into(), started: "1970-01-01T00:01:40Z".into(), ..Default::default() };
        let mut causes = Vec::new();
        assert!(matches!(proven_report(&fixture.0,&agent,2,&mut causes).unwrap(),Unavailable { .. }));
        disk::create_dir(fixture.0.join("docs")).unwrap();
        let path=fixture.0.join("docs/task.md");disk::write(&path,"actual report").unwrap();
        let set_time=|seconds| { disk::File::options().write(true).open(&path).unwrap().set_times(
            disk::FileTimes::new().set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds))).unwrap(); };
        set_time(98);
        assert!(matches!(proven_report(&fixture.0,&agent,2,&mut causes).unwrap(),Attributed(name) if name=="docs/task.md"));
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Unavailable{reason} if reason.contains("1 rapport(s) antérieur(s)")));
        set_time(100);
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Attributed(_)));
        let second=fixture.0.join("docs/task-other-handoff.md");disk::write(&second,"second").unwrap();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Unavailable{reason} if reason.contains("plusieurs rapports") && reason.contains("docs/task.md") && reason.contains("docs/task-other-handoff.md")));
        disk::remove_file(second).unwrap();disk::write(&path,"").unwrap();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Unavailable{reason} if reason=="aucun rapport lisible et non vide sous docs/ pour cette tâche"));
        disk::write(&path,"restored").unwrap();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Attributed(_)));
        assert!(causes.is_empty());
        assert!(proven_report(&fixture.0,&agent,-1,&mut causes).is_err());
        agent.started="not-a-date".into();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Unavailable{reason} if reason=="date de départ de la tentative illisible"));
        assert_eq!(causes.len(),1);assert!(std::error::Error::source(&causes[0]).is_some());
    }
    /// A contained genuine candidate precedes an outside-root link refusal;
    /// restoring its exact owned path reestablishes the read-only attribution.
    #[test]
    fn mission_status_unsubmitted_report_path_error_is_not_absence() {
        use crate::mission_status_tasks::ReportAttribution::Attributed;
        let fixture=Fixture::new();let foreign=Fixture::new();
        disk::create_dir(fixture.0.join("docs")).unwrap();
        let path=fixture.0.join("docs/task.md");disk::write(&path,"owned").unwrap();
        let agent=Agent { task_id:"task".into(), started:"1970-01-01T00:00:00Z".into(), ..Default::default() };
        let mut causes=Vec::new();assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Attributed(_)));
        let original=disk::read(&path).unwrap();disk::write(foreign.0.join("report"),"foreign").unwrap();
        disk::remove_file(&path).unwrap();std::os::unix::fs::symlink(foreign.0.join("report"),&path).unwrap();
        assert!(proven_report(&fixture.0,&agent,0,&mut causes).is_err());
        disk::remove_file(&path).unwrap();disk::write(&path,original).unwrap();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Attributed(_)));
    }
}
