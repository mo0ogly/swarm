//! Read-only host storage and version observations. No SQL, reservation or launch
//! authority is created; unavailable measurements retain typed causes and block.
#![cfg(target_os = "linux")]
use crate::engine_settings::RuntimeStoragePolicy;
use crate::{mission_status_contract::*, EngineSettingsDocument, RootStoreError};
use serde::Deserialize;
use crate::version_identity::release_version;
use std::{collections::BTreeSet, io, path::Path};

/// Legacy projection plus original safe-wrapper causes for unavailable probes,
/// Git fields or embedded history. A ready state covers storage only.
pub(crate) struct RuntimeObservation {
    pub runtime: RuntimeHealth,
    pub causes: Vec<RootStoreError>,
}
fn runtime_error(cause: impl std::error::Error + Send + Sync + 'static) -> RootStoreError {
    RootStoreError::Runtime(crate::RuntimeError::new(
        crate::RuntimeOperation::Platform,
        cause,
    ))
}
fn invalid(reason: &'static str) -> RootStoreError {
    runtime_error(io::Error::new(io::ErrorKind::InvalidData, reason))
}
/// Observe `.swarm` then the process temporary directory using statfs Bsize and
/// Bavail, preserving inode-unlimited semantics and checked byte arithmetic.
/// Requires an explicitly saved complete storage policy; historical absence
/// refuses before probes. Probe failures yield blocked/unknown plus typed causes.
/// Git uses saved command policy; absent compiler metadata stays unknown, while
/// malformed embedded identity refuses explicitly rather than borrowing HEAD.
/// Valid identity comes only from compiler metadata. No application state or SQL
/// is written by this observer; Git uses the existing command adapter contract.
pub(crate) fn observe(
    root: &Path,
    settings: &EngineSettingsDocument,
    observed_at: &str,
) -> Result<RuntimeObservation, RootStoreError> {
    settings.validate().map_err(RootStoreError::Settings)?;
    let policy = settings
        .values
        .runtime_storage_policy()
        .map_err(RootStoreError::Settings)?;
    let mut causes = vec![];
    let (version, diagnostic) = version(root, settings, &mut causes)?;
    let mut runtime = RuntimeHealth {
        state: "ready".into(),
        launch_allowed: true,
        observed_at: observed_at.into(),
        message: String::new(),
        next: String::new(),
        volumes: Some(vec![]),
        version,
        diagnostic,
    };
    for (kind, path) in [
        ("store", root.join(".swarm")),
        ("temporary", std::env::temp_dir()),
    ] {
        let volume = volume(kind, &path, policy, probe(&path), &mut causes);
        if volume.state == "blocked" || volume.state == "unknown" {
            runtime.state = "blocked".into();
            runtime.launch_allowed = false;
        } else if volume.state == "warning" && runtime.state == "ready" {
            runtime.state = "warning".into();
        }
        runtime
            .volumes
            .as_mut()
            .expect("initialized volume collection")
            .push(volume);
    }
    let (message,next) = match runtime.state.as_str() {
        "blocked" => ("Stockage indisponible ou presque plein : nouveaux départs suspendus.", "Libérez de l’espace sur le volume signalé ou rétablissez son accès, puis vérifiez à nouveau. Les autres conditions de lancement restent applicables."),
        "warning" => ("Le stockage approche de la saturation.", "Libérez de l’espace avant la prochaine exécution longue. Les seuils de stockage enregistrés restent applicables."),
        _ => ("Le stockage est disponible.", "Les autres conditions de lancement restent contrôlées par le moteur."),
    };
    runtime.message = message.into();
    runtime.next = next.into();
    Ok(RuntimeObservation { runtime, causes })
}
/// Measure only; multiplication overflow or a negative block size is unavailable,
/// never truncated. Files==0 is the protocol sentinel for no inode limit.
fn probe(path: &Path) -> Result<(u64, u64), RootStoreError> {
    let stat = rustix::fs::statfs(path)
        .map_err(|e| runtime_error(io::Error::from_raw_os_error(e.raw_os_error())))?;
    let block = u64::try_from(stat.f_bsize).map_err(runtime_error)?;
    let bytes = stat
        .f_bavail
        .checked_mul(block)
        .ok_or_else(|| invalid("runtime-storage-byte-overflow"))?;
    Ok((
        bytes,
        if stat.f_files == 0 {
            u64::MAX
        } else {
            stat.f_ffree
        },
    ))
}
/// Classify one measurement using only saved operator thresholds. Unknown has
/// zero counters solely for legacy DTO compatibility and retains its cause.
fn volume(
    kind: &str,
    path: &Path,
    policy: RuntimeStoragePolicy,
    measured: Result<(u64, u64), RootStoreError>,
    causes: &mut Vec<RootStoreError>,
) -> StorageVolume {
    let (available, inodes, state) = match measured {
        Err(error) => {
            causes.push(error);
            (0, 0, "unknown")
        }
        Ok((bytes, inodes)) => (
            bytes,
            inodes,
            if bytes < policy.stop_bytes || inodes < policy.stop_inodes {
                "blocked"
            } else if bytes < policy.warning_bytes {
                "warning"
            } else {
                "ready"
            },
        ),
    };
    StorageVolume {
        kind: kind.into(),
        path: path.to_string_lossy().into_owned(),
        available,
        inodes,
        state: state.into(),
    }
}
/// Observe source Git with the authoritative timeout. Unqualified compiled Rust
/// identity never borrows source HEAD; incomplete Git outputs remain nullable.
fn version(
    root: &Path,
    settings: &EngineSettingsDocument,
    causes: &mut Vec<RootStoreError>,
) -> Result<(ServerVersion, VersionDiagnostic), RootStoreError> {
    let mut commit = None;
    let mut modified = None;
    match crate::observe_git_metadata(root, settings) {
        Ok(git) => {
            let head_known = !git.state.head.is_empty()
                && !git
                    .diagnostics
                    .iter()
                    .any(|d| d.field == crate::GitMetadataField::Head);
            let changes_known = !git
                .diagnostics
                .iter()
                .any(|d| d.field == crate::GitMetadataField::Changes);
            commit = head_known.then(|| Box::new(git.state.head));
            modified =
                (head_known && changes_known).then(|| Box::new(!git.state.changes.is_empty()));
            for issue in git.issues {
                causes.push(RootStoreError::Git(issue.cause));
            }
        }
        Err(error) => causes.push(RootStoreError::GitMetadata(error)),
    }
    let head_known = commit.is_some();
    let history = match history(include_bytes!("../../../../version_history.json")) {
        Ok(h) => h,
        Err(error) => {
            causes.push(error);
            VersionHistory {
                schema: 1,
                available: false,
                releases: Some(vec![]),
                local_changes: None,
            }
        }
    };
    let binary=crate::binary_version().map_err(runtime_error)?;
    let source = SourceVersion {
        available: head_known,
        commit: commit.clone(),
        modified: modified.clone(),
        compared: false,
        matches_binary: None,
    };
    let installed_cli = VersionObservation { available:false,version:None,commit:None,modified:None,provenance:"browser".into(),hint:"Le navigateur ne peut pas inspecter le CLI local. Lancez swarm version --server URL_DE_SESSION.".into() };
    let active_server = VersionObservation {
        available: true,
        version: Some(Box::new(binary.version.clone())),
        commit: binary.commit.clone(),
        modified: binary.modified.clone(),
        provenance: binary.provenance.clone(),
        hint: String::new(),
    };
    let candidate = VersionObservation {
        available: head_known,
        version: None,
        commit,
        modified,
        provenance: "git".into(),
        hint: if head_known {
            String::new()
        } else {
            "Vérifiez --root et que le candidat est une copie Git lisible.".into()
        },
    };
    let diagnostic = crate::compare_version_observations(installed_cli,active_server,candidate);
    Ok((
        ServerVersion {
            binary,
            sources: source,
            history,
        },
        diagnostic,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: i64,
    releases: Vec<Release>,
    local_changes: Option<Local>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Summary {
    fr: String,
    en: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    sha: String,
    url: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Release {
    version: String,
    date: String,
    summary: Summary,
    commits: Option<Vec<Commit>>,
    release_url: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Local {
    summary: Summary,
    items: Vec<Summary>,
}
impl Summary {
    /// Reject empty, oversized or control-containing FR/EN history summaries.
    /// Validation is pure; malformed embedded data retains an InvalidData cause
    /// through the runtime wrapper and is never advertised as available history.
    fn validate(&self) -> Result<(), RootStoreError> {
        // Go history schema1 format bound, not an execution policy.
        if [&self.fr, &self.en]
            .iter()
            .any(|s| s.is_empty() || s.len() > 8192 || s.chars().any(char::is_control))
        {
            return Err(invalid("version-history-summary"));
        }
        Ok(())
    }
    /// Transfer a previously validated bilingual summary into the legacy DTO.
    /// This pure field projection neither revalidates input nor performs IO.
    fn project(self) -> LocalizedSummary {
        LocalizedSummary {
            fr: self.fr,
            en: self.en,
        }
    }
}
/// Validate the complete embedded schema1 manifest, matching retained Go format
/// limits (1MiB,20 releases,50 commits,8KiB summaries), never truncating. Invalid
/// content is unavailable with its typed cause retained by the runtime producer.
fn history(raw: &[u8]) -> Result<VersionHistory, RootStoreError> {
    if raw.len() > 1048576 {
        return Err(invalid("version-history-format-size"));
    }
    let manifest: Manifest = serde_json::from_slice(raw).map_err(runtime_error)?;
    if manifest.schema_version != 1 || manifest.releases.len() > 20 {
        return Err(invalid("version-history-envelope"));
    }
    let mut seen = BTreeSet::new();
    let mut releases = vec![];
    for release in manifest.releases {
        if !release_version(&release.version) || !seen.insert(release.version.clone()) {
            return Err(invalid("version-history-release-version"));
        }
        let p: Vec<_> = release.date.split('-').collect();
        if p.len() != 3
            || p[0].len() != 4
            || p[1].len() != 2
            || p[2].len() != 2
            || p.iter()
                .any(|part| !part.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(invalid("version-history-release-date"));
        }
        let date = p[0]
            .parse::<i32>()
            .ok()
            .zip(p[1].parse::<u8>().ok())
            .zip(p[2].parse::<u8>().ok());
        let valid = date.and_then(|((y, m), d)| {
            time::Month::try_from(m)
                .ok()
                .and_then(|m| time::Date::from_calendar_date(y, m, d).ok())
        });
        if valid.is_none() {
            return Err(invalid("version-history-release-date"));
        }
        release.summary.validate()?;
        let prefix = "https://github.com/";
        let suffix = format!("/releases/tag/{}", release.version);
        let repo = release
            .release_url
            .strip_prefix(prefix)
            .and_then(|s| s.strip_suffix(&suffix))
            .ok_or_else(|| invalid("version-history-release-url"))?;
        if repo.split('/').count() != 2
            || repo.split('/').any(|s| s.is_empty())
            || repo.chars().any(|c| c.is_control() || "?#@\\".contains(c))
        {
            return Err(invalid("version-history-release-url"));
        }
        let commits = release
            .commits
            .map(|commits| -> Result<Vec<_>, RootStoreError> {
                if commits.len() > 50 {
                    return Err(invalid("version-history-commit-count"));
                }
                commits
                    .into_iter()
                    .map(|c| {
                        if !commit_sha(&c.sha)
                            || c.url != format!("https://github.com/{repo}/commit/{}", c.sha)
                        {
                            return Err(invalid("version-history-commit"));
                        }
                        Ok(VersionCommit {
                            sha: c.sha,
                            url: c.url,
                        })
                    })
                    .collect()
            })
            .transpose()?;
        releases.push(VersionRelease {
            version: release.version,
            date: release.date,
            summary: release.summary.project(),
            commits,
            release_url: release.release_url,
        });
    }
    let local_changes = manifest
        .local_changes
        .map(|local| -> Result<_, RootStoreError> {
            local.summary.validate()?;
            if local.items.is_empty() {
                return Err(invalid("version-history-local-items"));
            }
            for item in &local.items {
                item.validate()?;
            }
            Ok(Box::new(LocalVersionChanges {
                summary: local.summary.project(),
                items: Some(local.items.into_iter().map(Summary::project).collect()),
            }))
        })
        .transpose()?;
    Ok(VersionHistory {
        schema: 1,
        available: true,
        releases: Some(releases),
        local_changes,
    })
}
/// Recognize the history format's exact lowercase SHA-1 spelling without IO.
/// This syntax check does not establish a real commit or executable provenance.
fn commit_sha(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Exercise saved threshold boundaries, disabled thresholds and unknown probes; pure fixture measurements never establish real disk health.
    #[test]
    fn storage_boundaries_unknown_and_disabled_thresholds() {
        let policy = RuntimeStoragePolicy {
            stop_bytes: 256,
            warning_bytes: 1024,
            stop_inodes: 256,
        };
        let mut causes = vec![];
        for (bytes, inodes, expected) in [
            (1024, 256, "ready"),
            (1023, 256, "warning"),
            (256, 256, "warning"),
            (255, 256, "blocked"),
            (1024, 255, "blocked"),
            (1024, u64::MAX, "ready"),
        ] {
            assert_eq!(
                volume(
                    "store",
                    Path::new("/fixture"),
                    policy,
                    Ok((bytes, inodes)),
                    &mut causes
                )
                .state,
                expected
            );
        }
        let off = RuntimeStoragePolicy {
            stop_bytes: 0,
            warning_bytes: 0,
            stop_inodes: 0,
        };
        assert_eq!(
            volume("store", Path::new("/fixture"), off, Ok((0, 0)), &mut causes).state,
            "ready"
        );
        assert_eq!(
            volume(
                "store",
                Path::new("/fixture"),
                off,
                Err(invalid("fixture-probe")),
                &mut causes
            )
            .state,
            "unknown"
        );
        assert_eq!(causes.len(), 1);
        assert!(std::error::Error::source(&causes[0]).is_some());
    }
    /// Build explicit storage test inputs independent of configurable installation
    /// thresholds. Other families retain a valid current installation fixture.
    fn saved_storage_values()->crate::EngineSettingsValues{
        let mut values=crate::EngineSettingsValues::installation_defaults().unwrap();
        values.runtime_storage_stop_bytes=Some(crate::SettingCount(256));
        values.runtime_storage_warning_bytes=Some(crate::SettingCount(1024));
        values.runtime_storage_stop_inodes=Some(crate::SettingCount(256));
        values.validate().unwrap();values
    }
    /// Reconstruct an explicitly historical version using the independently
    /// declared catalogue introductions, rather than clearing one new family
    /// while silently retaining later fields. This is test data, not migration
    /// adoption or a production fallback; decoding/validation failures must fail.
    fn historical_storage_values(version:i64)->crate::EngineSettingsValues{
        let mut raw=serde_json::to_value(saved_storage_values()).unwrap();
        let catalogue:serde_json::Value=serde_json::from_str(include_str!("../config/engine-settings-catalog.json")).unwrap();
        for field in catalogue["fields"].as_array().unwrap(){
            if field["introduced_schema_version"].as_i64().unwrap()>version{
                raw.as_object_mut().unwrap().remove(field["name"].as_str().unwrap());
            }
        }
        let values:crate::EngineSettingsValues=serde_json::from_value(raw).unwrap();values.validate().unwrap();values
    }
    /// Check absent/partial/malformed operator policy refusal and explicit zero semantics without adopting implicit installation defaults.
    #[test]
    fn saved_policy_absence_partial_invalid_and_explicit_zero() {
        let mut values = saved_storage_values();
        assert!(values.schema_version() >= 38); // Storage fields require at least protocol v38.
        assert_eq!(
            values.runtime_storage_policy().unwrap().stop_bytes,
            256
        );
        values.runtime_storage_warning_bytes = Some(crate::SettingCount(1));
        assert!(values.validate().is_err());
        values.runtime_storage_warning_bytes = None;
        assert!(values.validate().is_err());
        values.runtime_storage_stop_bytes = None;
        values.runtime_storage_stop_inodes = None;
        values.validate().unwrap();
        assert!(values.runtime_storage_policy().is_err());
        values.runtime_storage_stop_bytes = Some(crate::SettingCount(0));
        values.runtime_storage_warning_bytes = Some(crate::SettingCount(0));
        values.runtime_storage_stop_inodes = Some(crate::SettingCount(0));
        values.validate().unwrap();
        assert_eq!(values.runtime_storage_policy().unwrap().stop_inodes, 0);
    }
    /// Verify pure preview, explicit historical adoption, preserved history and decode reopening, then reject an invalid threshold relation.
    #[test]
    fn saved_storage_policy_preview_apply_reopen_and_refusal() {
        let old = historical_storage_values(37);
        let current=saved_storage_values();
        let doc = EngineSettingsDocument::initial(
            old,
            "fixture/uid:1".into(),
            "2026-10-23T00:00:00Z".into(),
        )
        .unwrap();
        assert_eq!(doc.schema_version, 37);
        assert!(doc.values.runtime_storage_policy().is_err());
        let mut change = crate::EngineSettingsChange {
            schema_version: current.schema_version(),
            event_id: "storage-policy-adoption".into(),
            expected_revision: doc.revision,
            values: current,
            reason: "Explicit installation adoption".into(),
            preview_token: String::new(),
        };
        let before = serde_json::to_vec(&doc).unwrap();
        change.preview_token = doc.preview(&change).unwrap().preview_token;
        assert_eq!(serde_json::to_vec(&doc).unwrap(), before);
        let (saved, replayed) = doc
            .apply(
                &change,
                "fixture/uid:1".into(),
                "2026-10-23T00:00:01Z".into(),
            )
            .unwrap();
        assert!(!replayed);
        let reopened =
            EngineSettingsDocument::decode(&serde_json::to_vec(&saved).unwrap()).unwrap();
        assert_eq!(
            reopened.values.runtime_storage_policy().unwrap().stop_bytes,
            256
        );
        assert_eq!(reopened.history.len(), 2);
        change.expected_revision = reopened.revision;
        change.event_id = "invalid-storage-policy".into();
        change.values.runtime_storage_warning_bytes = Some(crate::SettingCount(1));
        assert!(reopened.preview(&change).is_err());
        assert_eq!(reopened.history.len(), 2);
    }
    /// Observe a missing owned fixture and actual Git outcome; storage stays blocked.
    /// Binary identity must equal the immutable compiled declaration, whether
    /// unknown plain Cargo or explicit injected facts; no fixture is created.
    #[test]
    fn real_runtime_missing_store_is_blocked_with_actual_compiled_identity() {
        let root =
            std::env::temp_dir().join(format!("swarm-runtime-empty-root-{}", std::process::id()));
        assert!(!root.exists());
        let settings = EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture/uid:1".into(),
            "2026-10-23T00:00:00Z".into(),
        )
        .unwrap();
        let observation = observe(&root, &settings, "2026-10-23T00:00:00Z").unwrap();
        assert_eq!(observation.runtime.state, "blocked");
        assert!(!observation.runtime.launch_allowed);
        assert_eq!(observation.runtime.volumes.as_ref().unwrap().len(), 2);
        assert_eq!(
            observation.runtime.volumes.as_ref().unwrap()[0].state,
            "unknown"
        );
        assert!(observation.runtime.version.binary == crate::binary_version().unwrap());
        assert!(!observation.runtime.version.sources.compared);
        assert!(observation.runtime.version.sources.matches_binary.is_none());
        assert!(observation.runtime.version.history.available);
        assert!(!observation.causes.is_empty());
        assert!(!root.exists());
    }
    /// Validate the actual embedded history and controlled malformed schema/date/SemVer fixtures; no network or Git authority is inferred.
    #[test]
    fn embedded_history_and_invalid_variants() {
        assert!(
            history(include_bytes!("../../../../version_history.json"))
                .unwrap()
                .available
        );
        for raw in [br#"{"schema_version":1,"releases":null}"#.as_slice(),br#"{"schema_version":2,"releases":[]}"#,br#"{"schema_version":1,"releases":[],"extra":1}"#,br#"{"schema_version":1,"releases":[]} {}"#,br#"{"schema_version":1,"releases":[],"local_changes":{"summary":{"fr":"ok","en":"ok"},"items":[]}}"#] {assert!(history(raw).is_err());}
        let valid_release = br#"{"schema_version":1,"releases":[{"version":"v1.2.3","date":"2026-10-23","summary":{"fr":"ok","en":"ok"},"commits":[],"release_url":"https://github.com/owner/repo/releases/tag/v1.2.3"}]}"#;
        assert!(history(valid_release).is_ok());
        let invalid_year = String::from_utf8(valid_release.to_vec())
            .unwrap()
            .replace("2026-10-23", "+100-10-23");
        assert!(history(invalid_year.as_bytes()).is_err());
        assert!(release_version("v1.2.3-alpha.2+build.1"));
        for s in ["1.2.3", "v01.2.3", "v1.2.3-01", "v1.2.3+", "v1.2.3-"] {
            assert!(!release_version(s));
        }
    }
    /// Use the real temporary-volume probe as control, then preserve a genuinely missing path cause without writing or replacing storage.
    #[test]
    fn real_readonly_probe_preserves_missing_path_cause() {
        assert!(probe(&std::env::temp_dir()).is_ok());
        let missing =
            std::env::temp_dir().join(format!("swarm-missing-runtime-{}", std::process::id()));
        assert!(!missing.exists());
        let error = probe(&missing).unwrap_err();
        assert!(std::error::Error::source(&error).is_some());
        assert!(!missing.exists());
    }
}
