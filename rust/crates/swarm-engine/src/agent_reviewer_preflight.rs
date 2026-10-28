//! Current reviewer configuration observations, never model/account availability proof.
use crate::local_storage::LocalDirectory;
use crate::{AgentLaunchError, AgentLaunchPolicy, Launch, PlanningState};
use std::path::Path;
/// Match Go reviewerAvailable using actual Root catalogue/cooldown/executable
/// metadata and configured call capacity. No planning means no reviewer check;
/// a supplied hierarchy never receives a missing-reviewer or digest fallback.
/// Caller owns the observed Work revision and Root/settings transaction. Original
/// capacity/catalogue/connection/IO causes survive; no child or model is invoked.
/// Requested tool-free argv and an expired cooldown establish neither actual
/// health, account capacity, verified external isolation nor execution authority.
pub(crate) fn check(
    root: &Path,
    directory: &LocalDirectory,
    planning: Option<&PlanningState>,
    policy: &AgentLaunchPolicy,
) -> Result<(), AgentLaunchError> {
    let Some(planning) = planning else {
        return Ok(());
    };
    crate::validate_mission_reviewer_capacity(planning)
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    configuration(root, directory, planning, policy)
}
/// Observe an already counted reviewer call without charging capacity twice.
/// Caller must first verify the exact normalized review job, current Work and
/// intact financial reservation in the same settings/SQL snapshot. This permits
/// the last reserved call only: positive consumption may equal, never exceed,
/// the saved max_calls. Missing authorization/failure/counter still refuses.
/// No new call, refund, launch, model authentication or grant is created.
pub(crate) fn check_reserved(
    root: &Path,
    directory: &LocalDirectory,
    planning: &PlanningState,
    policy: &AgentLaunchPolicy,
) -> Result<(), AgentLaunchError> {
    let reviewer = planning
        .reviewer
        .as_deref()
        .filter(|reviewer| !reviewer.authorized.is_empty() && reviewer.max_calls > 0)
        .ok_or(AgentLaunchError::Refused("reviewer_required"))?;
    if reviewer.calls < 1 {
        return Err(AgentLaunchError::caused(
            "reviewer_unavailable",
            AgentLaunchError::Refused("reviewer-reserved-call-count-invalid"),
        ));
    }
    if reviewer.calls > reviewer.max_calls || !reviewer.failure.is_empty() {
        let error = crate::MissionOrganizationError::ReviewerUnavailable {
            calls: reviewer.calls,
            limit: reviewer.max_calls,
            failure: reviewer.failure.clone(),
        };
        return Err(AgentLaunchError::caused(error.code(), error));
    }
    configuration(root, directory, planning, policy)
}
/// Reuse exact catalogue/digest/cooldown/tool-free configuration observations.
/// Capacity/claim funding belongs to the caller's new-call or reserved-call guard;
/// these read operations grant no execution. Original IO/codec causes survive.
fn configuration(
    root: &Path,
    directory: &LocalDirectory,
    planning: &PlanningState,
    policy: &AgentLaunchPolicy,
) -> Result<(), AgentLaunchError> {
    let reviewer = planning
        .reviewer
        .as_deref()
        .ok_or(AgentLaunchError::Refused("reviewer_required"))?;
    let refuse = |reason| {
        AgentLaunchError::caused("reviewer_unavailable", AgentLaunchError::Refused(reason))
    };
    if !swarm_identity_contract::name(reviewer.provider.as_bytes()) {
        return Err(refuse("reviewer-provider-name-invalid"));
    }
    if !crate::provider_configuration::configuration_digest_valid(&reviewer.provider_digest) {
        return Err(refuse("reviewer-provider-digest-required"));
    }
    crate::agent_launch::cooldown(root, &reviewer.provider)?;
    let request = Launch {
        provider: reviewer.provider.clone(),
        provider_digest: reviewer.provider_digest.clone(),
        ..Default::default()
    };
    let (provider, _) = crate::agent_launch::provider(directory, &request, policy)
        .map_err(|e| AgentLaunchError::caused("reviewer_unavailable", e))?;
    crate::prepare_assistant_provider(&provider)
        .map_err(|e| AgentLaunchError::caused("reviewer_unavailable", e))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    /// Owned real Root files and a nonexecuted inert CLI establish each positive
    /// precondition. No fixture edits touch a live mission, quota or real account.
    #[test]
    fn reviewer_runtime_catalogue_digest_cooldown_and_executable_refuse_then_recover() {
        let path =
            std::env::temp_dir().join(crate::generate_identifier("reviewer-runtime-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let root = crate::RootStore::initialize(&path, &mut crate::SystemEnvironment).unwrap();
        let command = path.join("claude");
        fs::write(&command, b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&command, fs::Permissions::from_mode(0o700)).unwrap();
        let provider = crate::Provider {
            command: command.to_str().unwrap().into(),
            args: Some(vec!["--dangerously-skip-permissions".into()]),
            ..Default::default()
        };
        let digest = crate::provider_configuration::configuration_digest(
            &crate::encode_model(&provider).unwrap(),
        );
        let providers = crate::Providers {
            schema_version: 1,
            providers: Some(std::collections::BTreeMap::from([(
                "reviewer".into(),
                provider,
            )])),
        };
        root.initialize_provider_configuration(&providers).unwrap();
        let directory = LocalDirectory::open(&path.join(".swarm")).unwrap();
        let policy = root
            .settings()
            .unwrap()
            .values
            .agent_launch_policy()
            .unwrap();
        let p = crate::PlanningState {
            reviewer: Some(Box::new(crate::ReviewerConfig {
                provider: "reviewer".into(),
                provider_digest: digest,
                authorized: "declared".into(),
                max_calls: 10,
                ..Default::default()
            })),
            ..Default::default()
        };
        check(&path, &directory, None, &policy).unwrap();
        check(&path, &directory, Some(&p), &policy).unwrap();
        // Metadata witness only: real ownership/funding is covered by the public
        // fixture SDK. Last counted call fits; a new call remains exhausted.
        let mut reserved = p.clone();
        reserved.reviewer.as_mut().unwrap().calls = reserved.reviewer.as_ref().unwrap().max_calls;
        check_reserved(&path, &directory, &reserved, &policy).unwrap();
        assert!(check(&path, &directory, Some(&reserved), &policy).is_err());
        check_reserved(&path, &directory, &reserved, &policy).unwrap();
        reserved.reviewer.as_mut().unwrap().calls += 1;
        assert!(check_reserved(&path, &directory, &reserved, &policy).is_err());
        reserved.reviewer.as_mut().unwrap().calls -= 1;
        check_reserved(&path, &directory, &reserved, &policy).unwrap();
        reserved.reviewer.as_mut().unwrap().calls = 0;
        assert!(check_reserved(&path, &directory, &reserved, &policy).is_err());
        reserved.reviewer.as_mut().unwrap().calls = reserved.reviewer.as_ref().unwrap().max_calls;
        check_reserved(&path, &directory, &reserved, &policy).unwrap();
        reserved.reviewer.as_mut().unwrap().failure = "PRIVATE fixture refusal".into();
        let error = check_reserved(&path, &directory, &reserved, &policy).unwrap_err();
        assert!(!error.to_string().contains("PRIVATE"));
        reserved.reviewer.as_mut().unwrap().failure.clear();
        check_reserved(&path, &directory, &reserved, &policy).unwrap();
        let mut changed = p.clone();
        changed.reviewer.as_mut().unwrap().provider_digest = "a".repeat(64);
        assert_eq!(
            check(&path, &directory, Some(&changed), &policy)
                .unwrap_err()
                .code(),
            "reviewer_unavailable"
        );
        check(&path, &directory, Some(&p), &policy).unwrap();
        fs::set_permissions(&command, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            check(&path, &directory, Some(&p), &policy)
                .unwrap_err()
                .code(),
            "reviewer_unavailable"
        );
        fs::set_permissions(&command, fs::Permissions::from_mode(0o700)).unwrap();
        check(&path, &directory, Some(&p), &policy).unwrap();
        let cooldown = path.join(".swarm/provider-cooldowns");
        fs::create_dir(&cooldown).unwrap();
        fs::set_permissions(&cooldown, fs::Permissions::from_mode(0o700)).unwrap();
        let file = cooldown.join("reviewer.json");
        let value = crate::ProviderCooldown {
            provider: "reviewer".into(),
            observed_at: "2026-10-20T00:00:00Z".into(),
            source: "fixture".into(),
            signal: "backoff".into(),
            ..Default::default()
        };
        fs::write(&file, crate::encode_model(&value).unwrap()).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            check(&path, &directory, Some(&p), &policy)
                .unwrap_err()
                .code(),
            "provider_cooldown"
        );
        fs::remove_file(file).unwrap();
        check(&path, &directory, Some(&p), &policy).unwrap();
        let mut invalid = p.clone();
        invalid.reviewer.as_mut().unwrap().calls = -1;
        assert_eq!(
            check(&path, &directory, Some(&invalid), &policy)
                .unwrap_err()
                .code(),
            "reviewer_unavailable"
        );
        check(&path, &directory, Some(&p), &policy).unwrap();
        directory.close().unwrap();
        root.close().unwrap();
        fs::remove_dir_all(path).unwrap();
    }
}
