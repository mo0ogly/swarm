//! Policy-only adaptation of the existing checked provider-file publisher.
use super::{finished, RootStore, RootStoreError as E};
use crate::{ProviderPolicyChange, ProviderPolicyExport, ProviderPolicyMutation};
impl RootStore {
    /// Export only explicitly saved model policies in the portable version1
    /// shape. Read-only, bounded by current file policy; no API secret read or
    /// implicit named model. Known providers lacking saved policy and an empty
    /// export refuse, exposing migration work rather than silent policy loss.
    /// Structural/metadata/read/close failures retain safe causes. This proves
    /// neither catalogue availability nor operator/model identity or admission.
    pub fn export_provider_policies(&self) -> Result<ProviderPolicyExport, E> {
        let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
        let result = (|| {
            let source = self.provider_configuration()?;
            let mut policies = std::collections::BTreeMap::new();
            for (name, p) in source.configuration.providers.unwrap_or_default() {
                let known = crate::provider_model_route::adapter(&p) != "custom";
                if let Some(policy) = p.model_policy {
                    crate::provider_model_route::policy_levels(&policy)
                        .map_err(|e| E::Launch(Box::new(e)))?;
                    policies.insert(name, *policy);
                } else if known {
                    return Err(E::Refused(
                        "provider-policy-implicit-default-migration-required",
                    ));
                }
            }
            if policies.is_empty() {
                return Err(E::Refused("provider-policy-export-empty"));
            }
            Ok(ProviderPolicyExport {
                version: 1,
                policies,
            })
        })();
        finished(result, reservation.close().map_err(E::Storage))
    }
    /// Preview policy changes without creating provider locks/snapshots or SQL
    /// mutations. All tiers/defaults are checked against current saved rules and
    /// unverified catalogue data, not execution permission. The public token
    /// comes from the common Root/source/settings binding. Errors retain causes.
    pub fn preview_provider_policies(
        &self,
        r: &ProviderPolicyChange,
    ) -> Result<ProviderPolicyMutation, E> {
        self.change_provider_policies(r, false)
    }
    /// Apply only current source/token on a writable Root. Explicit preview:true
    /// cannot write. Exact originals, shared lock/CAS/snapshot/fsync, uncertainty
    /// and recovery are delegated to the existing publisher. No command/env/key,
    /// Agent/Work/reservation or audit identity is changed/claimed. Source and
    /// catalogue failures refuse before publication; all late causes survive.
    pub fn apply_provider_policies(
        &self,
        r: &ProviderPolicyChange,
    ) -> Result<ProviderPolicyMutation, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        if r.preview {
            return Err(E::Refused("provider-policy-preview-not-application"));
        }
        self.change_provider_policies(r, true)
    }
    /// Merge only requested policies under one settings reservation. Nested
    /// shared readers remain checked/closed; the provider publisher rereads CAS.
    /// Catalogue metadata grants no launch capability and no hidden count32 or
    /// byte65536 ceiling is added: current per-file/transport policies govern.
    fn change_provider_policies(
        &self,
        r: &ProviderPolicyChange,
        apply: bool,
    ) -> Result<ProviderPolicyMutation, E> {
        let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
        let mut published = false;
        let result = (|| {
            let source = self.provider_configuration()?;
            if source.digest != r.expected_digest {
                return Err(E::Refused("revision_conflict"));
            }
            let mut configuration = source.configuration;
            let entries = r.entries().map_err(E::Providers)?;
            let settings = self.read_settings()?;
            let policy = settings.values.agent_launch_policy().map_err(E::Settings)?;
            let routing = settings
                .values
                .agent_model_routing_policy()
                .map_err(E::Settings)?;
            for (name, model_policy) in &entries {
                let provider = configuration
                    .providers
                    .as_mut()
                    .and_then(|p| p.get_mut(name))
                    .ok_or(E::Refused("provider-policy-provider-missing"))?;
                if !provider.api_connection_id.is_empty() {
                    return Err(E::Refused("provider-policy-api-unported"));
                }
                let levels = crate::provider_model_route::policy_levels(model_policy)
                    .map_err(|e| E::Launch(Box::new(e)))?;
                provider.model_policy = Some(Box::new(model_policy.clone()));
                for level in ["simple", "standard", "exigeant"] {
                    let launch = crate::Launch {
                        provider: name.clone(),
                        level: level.into(),
                        ..Default::default()
                    };
                    crate::provider_model_route::resolve_configuration_metadata(
                        provider,
                        &launch,
                        &settings.values,
                        &policy,
                    )
                    .map_err(|e| E::Launch(Box::new(e)))?;
                }
                for level in [&model_policy.work_level, &model_policy.page_level] {
                    let choice = levels
                        .get(level)
                        .ok_or(E::Refused("provider-policy-default-invalid"))?;
                    routing
                        .check_selection(level, &choice.model, true)
                        .map_err(|e| {
                            E::Launch(Box::new(crate::AgentLaunchError::caused(e.reason(), e)))
                        })?;
                }
            }
            let change = crate::ProviderConfigurationChange {
                expected_digest: r.expected_digest.clone(),
                proposal:
                    crate::provider_configuration::ProviderConfigurationProposal::Configuration(
                        configuration,
                    ),
                preview_token: r.preview_token.clone(),
            };
            let result = if apply {
                let outcome = self.apply_provider_configuration(&change);
                published = match &outcome {
                    Ok(view) => view.changed,
                    Err(E::Published { .. }) => true,
                    _ => false,
                };
                outcome?
            } else {
                self.preview_provider_configuration(&change)?
            };
            Ok(ProviderPolicyMutation::observed(result, entries))
        })();
        policy_finished(result, reservation.close().map_err(E::Storage), published)
    }
}
/// Join the outer settings-release cause with the primary result. A known
/// changed/uncertain publication stays top-level preserved state, including a
/// compound release failure. Existing Published errors are not redundantly
/// wrapped. Preview/no-change/before-publication errors claim no new bytes.
/// This only classifies observed outcomes; it performs no IO, rollback or retry.
fn policy_finished<T>(
    result: Result<T, E>,
    cleanup: Result<(), E>,
    published: bool,
) -> Result<T, E> {
    match finished(result, cleanup) {
        Err(error @ E::Published { .. }) => Err(error),
        Err(primary) if published => Err(E::Published {
            primary: Box::new(primary),
        }),
        result => result,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// A known prior publication plus outer-release failure must remain explicit
    /// preserved state. This models result aggregation, not an actual OS failure.
    #[test]
    fn policy_outer_release_retains_known_published_state() {
        let error = policy_finished(
            Ok::<(), E>(()),
            Err(E::Refused("fixture-outer-release")),
            true,
        )
        .err()
        .unwrap();
        assert!(
            matches!(error, E::Published { .. }),
            "known publication became ordinary cleanup error"
        );
        let error = policy_finished(
            Err::<(), E>(E::Published {
                primary: Box::new(E::Refused("fixture-prior-publication")),
            }),
            Err(E::Refused("fixture-outer-release")),
            true,
        )
        .err()
        .unwrap();
        match error {
            E::Published { primary } => match *primary {
                E::Cleanup {
                    primary: Some(prior),
                    cleanup,
                } => {
                    assert!(matches!(*prior, E::Published { .. }));
                    assert!(matches!(*cleanup, E::Refused("fixture-outer-release")));
                }
                _ => panic!("both primary and cleanup must remain"),
            },
            _ => panic!("compound known publication lost"),
        }
        assert!(policy_finished(Ok::<(), E>(()), Ok(()), true).is_ok());
        assert!(matches!(
            policy_finished(
                Err::<(), E>(E::Refused("fixture-before-publication")),
                Err(E::Refused("fixture-outer-release")),
                false
            ),
            Err(E::Cleanup { .. })
        ));
        assert!(matches!(
            policy_finished(
                Ok::<(), E>(()),
                Err(E::Refused("fixture-readonly-release")),
                false
            ),
            Err(E::Cleanup { primary: None, .. })
        ));
    }
}
