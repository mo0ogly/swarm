//! Provider-file bootstrap under the parent store's checked directory/settings.
use super::{absent, finished, RootStore, RootStoreError};
use crate::{
    local_storage::{AtomicStorageFault, PublicationState},
    ProviderConfiguration, Providers,
};
impl RootStore {
    /// Read the exact CLI provider file under its saved per-file cap and a shared
    /// settings reservation. Missing/unsafe/nonregular/type/schema/read/close
    /// failures refuse; no empty fallback, API-secret read or discovery happens.
    /// Root must already be initialized with an explicit launch-policy group.
    /// This is retained configuration, not runtime availability or permission.
    pub fn provider_configuration(&self) -> Result<ProviderConfiguration, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let policy = settings
                .values
                .agent_launch_policy()
                .map_err(RootStoreError::Settings)?;
            let raw = self
                .directory
                .read_capped("providers.json", policy.provider_bytes)
                .map_err(RootStoreError::Storage)?;
            let config =
                crate::decode_provider_configuration(&raw).map_err(RootStoreError::Providers)?;
            Ok(ProviderConfiguration::observed(config, &raw, false))
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Create the CLI provider file exactly once using supplied typed data.
    /// Validates outer schema and current saved per-file cap under an exclusive
    /// settings reservation. Atomic NOREPLACE publication never overwrites any
    /// existing/racing entry, including an unsafe/symlink target. No default,
    /// discovery, API merge, budget/Work mutation or execution is performed.
    /// Original causes and cleanup survive. Visible rename with uncertain dir
    /// durability or later close failure returns preserved-state error: reread,
    /// never assume rollback. No successful initialization grants admission.
    pub fn initialize_provider_configuration(
        &self,
        config: &Providers,
    ) -> Result<ProviderConfiguration, RootStoreError> {
        self.initialize_provider_configuration_impl(config, AtomicStorageFault::None)
    }
    /// Test-only injection into the real create-only publisher. Absent from the
    /// normal image; fixture input is not a serialized policy or execution grant.
    /// Never call this against a live/shared root. Same preconditions/causes and
    /// preserved-state semantics as initialize_provider_configuration apply.
    #[cfg(feature = "test-fixtures")]
    pub fn fixture_initialize_provider_configuration(
        &self,
        config: &Providers,
        fault: AtomicStorageFault,
    ) -> Result<ProviderConfiguration, RootStoreError> {
        self.initialize_provider_configuration_impl(config, fault)
    }
    /// Shared no-replace path; the fault is a compile-gated verifier, not policy.
    /// Cap applies to actual canonical stored bytes. Configuration-only validation
    /// deliberately retains nested declarations for later authoritative admission.
    fn initialize_provider_configuration_impl(
        &self,
        config: &Providers,
        fault: AtomicStorageFault,
    ) -> Result<ProviderConfiguration, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        crate::provider_configuration::validate_provider_configuration(config)
            .map_err(RootStoreError::Providers)?;
        let reservation = self
            .directory
            .reserve(true, false)
            .map_err(RootStoreError::Storage)?;
        let mut visible = false;
        let result = (|| {
            let settings = self.read_settings()?;
            let policy = settings
                .values
                .agent_launch_policy()
                .map_err(RootStoreError::Settings)?;
            let raw = crate::encode_model(config).map_err(|e| {
                RootStoreError::Providers(crate::ProviderConfigurationError::Format(e))
            })?;
            if policy.provider_bytes.is_some_and(|cap| raw.len() > cap) {
                return Err(RootStoreError::Providers(
                    crate::ProviderConfigurationError::Refused(
                        "provider-configuration-outside-saved-policy",
                    ),
                ));
            }
            // Fast refusal is not the race protection; NOREPLACE below is.
            match self
                .directory
                .read_capped("providers.json", policy.provider_bytes)
            {
                Ok(_) => {
                    return Err(RootStoreError::Refused(
                        "provider-configuration-already-exists",
                    ))
                }
                Err(e) if absent(&e) => {}
                Err(e) => return Err(RootStoreError::Storage(e)),
            }
            if let Err(e) = self
                .directory
                .publish_new_with_fault("providers.json", &raw, fault)
            {
                visible = e.publication == PublicationState::RenamedDurabilityUnknown;
                return Err(RootStoreError::Storage(e));
            }
            visible = true;
            Ok(ProviderConfiguration::observed(config.clone(), &raw, true))
        })();
        match finished(result, reservation.close().map_err(RootStoreError::Storage)) {
            Err(primary) if visible => Err(RootStoreError::Published {
                primary: Box::new(primary),
            }),
            result => result,
        }
    }
}

/// Source bytes and preview computed under one checked settings snapshot.
/// This engine-only value is not deserializable and grants no execution.
struct ConfigurationProposal {
    before: Vec<u8>,
    after: Vec<u8>,
    result: crate::ProviderConfigurationMutation,
}
impl RootStore {
    /// Observe only current-file or selected-snapshot metadata, even when its
    /// outer format is invalid. Invalid content is explicitly marked, never an
    /// empty/ready provider set. No API-secret read, repair or mutation occurs;
    /// missing/unsafe/read/policy/close failures remain errors. Snapshot refs must
    /// follow the reserved grammar; observed/claimed digests stay distinct.
    pub fn inspect_provider_configuration(
        &self,
        reference: Option<&str>,
    ) -> Result<crate::ProviderConfigurationInspection, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let policy = settings
                .values
                .agent_launch_policy()
                .map_err(RootStoreError::Settings)?;
            let expected = if let Some(r) = reference {
                Some(
                    crate::provider_configuration::configuration_backup_digest(r).ok_or(
                        RootStoreError::Refused("provider-configuration-reference-invalid"),
                    )?,
                )
            } else {
                None
            };
            let raw = self
                .directory
                .read_capped(reference.unwrap_or("providers.json"), policy.provider_bytes)
                .map_err(RootStoreError::Storage)?;
            let digest = crate::provider_configuration::configuration_digest(&raw);
            let parsed = crate::decode_provider_configuration(&raw);
            Ok(crate::ProviderConfigurationInspection {
                schema_version: 1,
                matches_reference: expected.map(|v| v == digest),
                expected_backup_digest: expected.map(str::to_owned),
                reference: reference.map(str::to_owned),
                digest,
                format_valid: parsed.is_ok(),
                format_error_code: parsed.err().map(|e| e.code()),
                scope: "source_metadata_only_no_admission",
            })
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Enumerate reserved snapshot references only. Directory/read/close failures
    /// refuse without a partial successful list. Names are neither verified
    /// content nor authenticated/ordered audit history. Restore checks actual
    /// descriptor ownership/type, bytes, digest and format again; no file changes.
    pub fn provider_configuration_backups(
        &self,
    ) -> Result<crate::ProviderConfigurationBackups, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let mut references: Vec<String> = self
                .directory
                .names()
                .map_err(RootStoreError::Storage)?
                .into_iter()
                .filter_map(|n| String::from_utf8(n).ok())
                .filter(|n| crate::provider_configuration::configuration_backup_digest(n).is_some())
                .collect();
            references.sort();
            references.dedup();
            Ok(crate::ProviderConfigurationBackups {
                schema_version: 1,
                references,
                scope: "snapshot_references_only_unverified",
            })
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Preview one source-bound update/restoration without creating any file or
    /// provider lock. Token binds before/after bytes, proposal, observed Root
    /// descriptor identity, settings revision and actual value digest; it is not
    /// a secret, authenticated identity, adoption or launch permission.
    /// Malformed proposals, stale sources, invalid/unsafe snapshots, policy/codec/
    /// read/close failures refuse. Nested declarations remain unverified data.
    pub fn preview_provider_configuration(
        &self,
        change: &crate::ProviderConfigurationChange,
    ) -> Result<crate::ProviderConfigurationMutation, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            Ok(self
                .provider_configuration_proposal(change, &settings)?
                .result)
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Apply only a current preview after acquiring Go's providers.lock and the
    /// checked settings reservation. CAS/token are verified before writes; exact
    /// prior bytes are durably snapshotted under a unique no-replace reference.
    /// Source and snapshot are reread before replacement. No agent, reservation,
    /// model default, API secret or chronological audit ledger is changed/claimed.
    /// Refusals preserve originals; visible replacement/late cleanup failure
    /// returns preserved state, including uncertain directory durability. No-op
    /// is explicit. Retry with an old digest refuses; reread/repreview instead.
    pub fn apply_provider_configuration(
        &self,
        change: &crate::ProviderConfigurationChange,
    ) -> Result<crate::ProviderConfigurationMutation, RootStoreError> {
        self.apply_provider_configuration_impl(
            change,
            AtomicStorageFault::None,
            AtomicStorageFault::None,
        )
    }
    /// Test-only injection into backup/replacement stages of the real path.
    /// Not compiled into the normal binary or accepted by JSON. Never use against
    /// a live/shared root; same preconditions, causes and preservation contract.
    #[cfg(feature = "test-fixtures")]
    pub fn fixture_apply_provider_configuration(
        &self,
        change: &crate::ProviderConfigurationChange,
        backup_fault: AtomicStorageFault,
        replace_fault: AtomicStorageFault,
    ) -> Result<crate::ProviderConfigurationMutation, RootStoreError> {
        self.apply_provider_configuration_impl(change, backup_fault, replace_fault)
    }
    /// Build a complete preview under the caller's settings reservation. A corrupt
    /// current file can be replaced only by an explicit matching-digest proposal;
    /// its exact bytes remain available for the snapshot. Restoration validates
    /// the selected backup's content hash and outer format before proposing it.
    fn provider_configuration_proposal(
        &self,
        change: &crate::ProviderConfigurationChange,
        settings: &crate::EngineSettingsDocument,
    ) -> Result<ConfigurationProposal, RootStoreError> {
        use crate::provider_configuration::{
            configuration_digest as hash, ProviderConfigurationProposal as P,
        };
        let policy = settings
            .values
            .agent_launch_policy()
            .map_err(RootStoreError::Settings)?;
        let before = self
            .directory
            .read_capped("providers.json", policy.provider_bytes)
            .map_err(RootStoreError::Storage)?;
        let before_digest = hash(&before);
        if before_digest != change.expected_digest {
            return Err(RootStoreError::Refused("revision_conflict"));
        }
        let (after, configuration, restore_reference) = match &change.proposal {
            P::Configuration(config) => {
                crate::provider_configuration::validate_provider_configuration(config)
                    .map_err(RootStoreError::Providers)?;
                let raw = crate::encode_model(config).map_err(|e| {
                    RootStoreError::Providers(crate::ProviderConfigurationError::Format(e))
                })?;
                (raw, config.clone(), None)
            }
            P::Restore(reference) => {
                let expected =
                    crate::provider_configuration::configuration_backup_digest(reference).ok_or(
                        RootStoreError::Refused("provider-configuration-reference-invalid"),
                    )?;
                let raw = self
                    .directory
                    .read_capped(reference, policy.provider_bytes)
                    .map_err(RootStoreError::Storage)?;
                if hash(&raw) != expected {
                    return Err(RootStoreError::Refused(
                        "provider-configuration-backup-digest-mismatch",
                    ));
                }
                let config = crate::decode_provider_configuration(&raw)
                    .map_err(RootStoreError::Providers)?;
                (raw, config, Some(reference.clone()))
            }
        };
        if policy.provider_bytes.is_some_and(|cap| after.len() > cap) {
            return Err(RootStoreError::Refused(
                "provider-configuration-outside-saved-policy",
            ));
        }
        let after_digest = hash(&after);
        let policy_bytes = crate::encode_model(&settings.values)
            .map_err(|e| RootStoreError::Providers(crate::ProviderConfigurationError::Format(e)))?;
        let root_storage_identity = self.directory.identity().map_err(RootStoreError::Storage)?;
        let binding=crate::encode_model(&serde_json::json!({"schema_version":1,"before":before_digest,"after":after_digest,"settings_revision":settings.revision,"settings_digest":hash(&policy_bytes),"root_storage_identity":root_storage_identity,"restore_reference":restore_reference})).map_err(|e|RootStoreError::Providers(crate::ProviderConfigurationError::Format(e)))?;
        let token = hash(&binding);
        Ok(ConfigurationProposal {
            before,
            after,
            result: crate::ProviderConfigurationMutation {
                schema_version: 1,
                preview: true,
                applied: false,
                changed: before_digest != after_digest,
                before_digest,
                after_digest,
                preview_token: token,
                backup_reference: None,
                restore_reference,
                configuration,
                provider_execution_started: false,
                scope: "cli_configuration_content_only_unverified",
            },
        })
    }
    /// Shared fixture/normal implementation; fault enums are verification devices,
    /// never policies. Each acquired lock is explicitly closed with joined causes.
    /// A durable snapshot may remain after a refused replacement; it never grants
    /// permission or proves that the current file changed. No hidden retry occurs.
    fn apply_provider_configuration_impl(
        &self,
        change: &crate::ProviderConfigurationChange,
        backup_fault: AtomicStorageFault,
        replace_fault: AtomicStorageFault,
    ) -> Result<crate::ProviderConfigurationMutation, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        let settings_lock = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let mut visible = false;
        let result = (|| {
            // Reject known-stale/invalid decisions before even creating the
            // provider coordination lock; repeat under that lock for races.
            let settings = self.read_settings()?;
            let first = self.provider_configuration_proposal(change, &settings)?;
            if change.preview_token.is_empty() || change.preview_token != first.result.preview_token
            {
                return Err(RootStoreError::Refused(
                    "provider-configuration-preview-required",
                ));
            }
            let provider_lock = self
                .directory
                .reserve_provider_configuration()
                .map_err(RootStoreError::Storage)?;
            let result = (|| {
                let settings = self.read_settings()?;
                let policy = settings
                    .values
                    .agent_launch_policy()
                    .map_err(RootStoreError::Settings)?;
                let mut proposal = self.provider_configuration_proposal(change, &settings)?;
                if change.preview_token.is_empty()
                    || change.preview_token != proposal.result.preview_token
                {
                    return Err(RootStoreError::Refused(
                        "provider-configuration-preview-required",
                    ));
                }
                if proposal.result.changed {
                    let id =
                        crate::generate_identifier("snapshot-").map_err(RootStoreError::Runtime)?;
                    let reference = format!(
                        "providers-before-{}-{id}.json",
                        proposal.result.before_digest
                    );
                    if crate::provider_configuration::configuration_backup_digest(&reference)
                        .is_none()
                    {
                        return Err(RootStoreError::Refused(
                            "provider-configuration-snapshot-identity-invalid",
                        ));
                    }
                    self.directory
                        .publish_new_with_fault(&reference, &proposal.before, backup_fault)
                        .map_err(RootStoreError::Storage)?;
                    let retained = self
                        .directory
                        .read_capped(&reference, policy.provider_bytes)
                        .map_err(RootStoreError::Storage)?;
                    if retained != proposal.before {
                        return Err(RootStoreError::Refused(
                            "provider-configuration-snapshot-changed",
                        ));
                    }
                    let current = self
                        .directory
                        .read_capped("providers.json", policy.provider_bytes)
                        .map_err(RootStoreError::Storage)?;
                    if current != proposal.before {
                        return Err(RootStoreError::Refused("revision_conflict"));
                    }
                    // A selected snapshot is mutable local data, not authority.
                    if let Some(reference) = &proposal.result.restore_reference {
                        if self
                            .directory
                            .read_capped(reference, policy.provider_bytes)
                            .map_err(RootStoreError::Storage)?
                            != proposal.after
                        {
                            return Err(RootStoreError::Refused(
                                "provider-configuration-restore-source-changed",
                            ));
                        }
                    }
                    if let Err(e) = self.directory.publish_with_fault(
                        "providers.json",
                        &proposal.after,
                        replace_fault,
                    ) {
                        visible = e.publication == PublicationState::RenamedDurabilityUnknown;
                        return Err(RootStoreError::Storage(e));
                    }
                    visible = true;
                    proposal.result.backup_reference = Some(reference);
                } else {
                    // Explicitly re-confirm durability after an earlier visible
                    // replacement error; a readonly read alone cannot prove it.
                    visible = true;
                    self.directory
                        .synchronize_existing("providers.json")
                        .map_err(RootStoreError::Storage)?;
                }
                proposal.result.preview = false;
                proposal.result.applied = true;
                Ok(proposal.result)
            })();
            finished(
                result,
                provider_lock.close().map_err(RootStoreError::Storage),
            )
        })();
        let result = finished(
            result,
            settings_lock.close().map_err(RootStoreError::Storage),
        );
        match result {
            Err(primary) if visible => Err(RootStoreError::Published {
                primary: Box::new(primary),
            }),
            result => result,
        }
    }
}
