//! Native requested model routing. Local/configured announcements never prove
//! actual model identity, account availability, pricing or execution permission.
#![cfg(target_os = "linux")]
use crate::{
    agent_launch_error::finish,
    agent_launch_preflight::hash,
    model_wire::{Fields, Kind},
    AgentLaunchError, AgentLaunchPolicy, EngineSettingsValues, Launch, ModelRoute, Provider,
    ProviderModelSyntax,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    os::{fd::IntoRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
};

/// Compare every saved model-route field except explanatory reason text.
/// Missing versus present differs; this compares supplied metadata only, with no
/// IO, normalization, source authentication, availability or execution authority.
pub(crate) fn same_authority_route(old: Option<&crate::ModelRoute>, new: Option<&crate::ModelRoute>) -> bool {
    match (old, new) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            let mut expected = a.clone();
            expected.reason.clone_from(&b.reason);
            expected == *b
        }
        _ => false,
    }
}

/// Engine-constructed candidate, deliberately neither deserializable nor a
/// launch capability. Private fields keep callers on the checked constructor.
pub(crate) struct ModelRoutePreparation {
    original: Provider,
    provider: Provider,
    route: Option<ModelRoute>,
    scope: Option<String>,
}
impl ModelRoutePreparation {
    /// Borrow the effective argv/configuration; never execute it here.
    pub(crate) fn provider(&self) -> &Provider {
        &self.provider
    }
    /// Borrow the requested route, absent only for supported unrouted custom CLI.
    pub(crate) fn route(&self) -> Option<&ModelRoute> {
        self.route.as_ref()
    }
    /// Bind relevant rules/catalogue sources into the existing host receipt.
    /// Absence preserves the old custom/no-route scope representation.
    pub(crate) fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }
    /// Compare the unique model-source check with this fresh candidate. A missing,
    /// duplicate, changed or falsely verified receipt refuses. Unrouted historical
    /// custom receipts must contain no model check. This grants no permission.
    pub(crate) fn check_recorded_scope(
        &self,
        original: &crate::PreflightResult,
    ) -> Result<(), AgentLaunchError> {
        let mut checks = original
            .checks
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|c| c.name == "model_route_sources");
        let recorded = checks.next();
        if checks.next().is_some()
            || recorded.is_some_and(|c| c.status != "declared_unverified")
            || recorded.map(|c| c.detail.as_str()) != self.scope()
        {
            return Err(AgentLaunchError::Refused(
                "agent-execution-model-route-source-changed",
            ));
        }
        Ok(())
    }
    /// Reread effective-environment catalogue sources and apply the same saved
    /// decision. Refuse changed candidate bytes; no defaults, retry or child.
    pub(crate) fn recheck(
        &self,
        launch: &Launch,
        settings: &EngineSettingsValues,
        policy: &AgentLaunchPolicy,
    ) -> Result<(), AgentLaunchError> {
        let fresh = resolve(&self.original, launch, settings, policy)?;
        if fresh.provider != self.provider || fresh.route != self.route || fresh.scope != self.scope
        {
            return Err(AgentLaunchError::Refused(
                "agent-model-route-source-changed",
            ));
        }
        Ok(())
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct ModelCache {
    models: Option<Vec<CacheModel>>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct CacheModel {
    slug: String,
    visibility: String,
    supported_reasoning_levels: Option<Vec<CacheEffort>>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct CacheEffort {
    effort: String,
}
/// External Codex cache field shape only. Unknown metadata stays ignored through
/// the existing iterative Go projection; nil/string/type behavior stays explicit.
fn cache_shape() -> Fields {
    &[("models", Kind::Array(&Kind::Object(cache_model_shape)))]
}
/// Known model metadata, not a capability or authentication assertion.
fn cache_model_shape() -> Fields {
    &[
        ("slug", Kind::String),
        ("visibility", Kind::String),
        (
            "supported_reasoning_levels",
            Kind::Array(&Kind::Object(cache_effort_shape)),
        ),
    ]
}
/// Announced effort string; the selected value still needs argv validation.
fn cache_effort_shape() -> Fields {
    &[("effort", Kind::String)]
}

/// Close the actual owned cache FD; a close failure prevents successful use.
fn close(file: File) -> Result<(), AgentLaunchError> {
    nix::unistd::close(file.into_raw_fd())
        .map_err(|e| AgentLaunchError::caused("agent-model-catalogue-close", e))
}
/// Read one regular local metadata file under the same saved per-file provider
/// configuration cap. NONBLOCK prevents opening a FIFO from waiting for a writer;
/// no contents are read from nonregular objects. Missing is distinct from invalid,
/// permission/read/change/close errors. Buffer8192 is transfer representation,
/// never an operator ceiling. No file/configuration/provider is modified.
fn read_cache(path: &Path, cap: Option<usize>) -> Result<Option<Vec<u8>>, AgentLaunchError> {
    let fd = match rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(e) if e == rustix::io::Errno::NOENT => return Ok(None),
        Err(e) => return Err(AgentLaunchError::caused("agent-model-catalogue-open", e)),
    };
    let mut file: File = fd.into();
    let result = (|| {
        let before = file
            .metadata()
            .map_err(|e| AgentLaunchError::caused("agent-model-catalogue-metadata", e))?;
        if !before.is_file() {
            return Err(AgentLaunchError::Refused(
                "agent-model-catalogue-not-regular",
            ));
        }
        if cap.is_some_and(|cap| u128::from(before.len()) > cap as u128) {
            return Err(AgentLaunchError::Refused(
                "agent-model-catalogue-outside-saved-policy",
            ));
        }
        let mut raw = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|e| AgentLaunchError::caused("agent-model-catalogue-read", e))?;
            if count == 0 {
                break;
            }
            let next = raw
                .len()
                .checked_add(count)
                .ok_or(AgentLaunchError::Refused(
                    "agent-model-catalogue-size-overflow",
                ))?;
            if cap.is_some_and(|cap| next > cap) {
                return Err(AgentLaunchError::Refused(
                    "agent-model-catalogue-outside-saved-policy",
                ));
            }
            raw.extend_from_slice(&buffer[..count]);
        }
        let after = file
            .metadata()
            .map_err(|e| AgentLaunchError::caused("agent-model-catalogue-metadata", e))?;
        let current = std::fs::metadata(path)
            .map_err(|e| AgentLaunchError::caused("agent-model-catalogue-path", e))?;
        let signature =
            |m: &std::fs::Metadata| (m.dev(), m.ino(), m.len(), m.mtime(), m.mtime_nsec());
        if signature(&before) != signature(&after) || signature(&after) != signature(&current) {
            return Err(AgentLaunchError::Refused(
                "agent-model-catalogue-changed-during-read",
            ));
        }
        Ok(Some(raw))
    })();
    finish(result, close(file))
}

/// Resolve cache location from the child's explicit effective name allowlist,
/// not a hidden parent variable or system-home fallback. Empty/relative supplied
/// homes refuse to avoid observing a different cwd/account than the child.
/// No effective HOME is explicit absence; path values never leave this function.
fn cache_path(
    provider: &Provider,
    policy: &AgentLaunchPolicy,
) -> Result<Option<PathBuf>, AgentLaunchError> {
    let mut names = BTreeSet::new();
    for name in policy
        .environment
        .iter()
        .chain(provider.env_allow.as_deref().unwrap_or_default())
    {
        if !crate::agent_launch_policy::environment_name(name) {
            return Err(AgentLaunchError::Refused(
                "agent-provider-environment-name-invalid",
            ));
        }
        names.insert(name.as_str());
    }
    for name in ["CODEX_HOME", "HOME"] {
        if !names.contains(name) {
            continue;
        }
        if let Some(value) = std::env::var_os(name) {
            let root = PathBuf::from(value);
            if !root.is_absolute() {
                return Err(AgentLaunchError::Refused(
                    "agent-model-catalogue-home-absolute-required",
                ));
            }
            return Ok(Some(if name == "CODEX_HOME" {
                root.join("models_cache.json")
            } else {
                root.join(".codex/models_cache.json")
            }));
        }
    }
    Ok(None)
}
/// Recognize configured model spelling using the same dialect helper as the
/// compiler, preserving generic single-dash flags. Execution/resolution callers
/// separately compile argv and refuse unsafe forms. Inventory also displays this
/// raw declaration without compiling it; no argument validity, effective model
/// identity or availability is established.
fn configured_model(args: &[String], syntax: ProviderModelSyntax) -> Option<&str> {
    for (index, arg) in args.iter().enumerate() {
        if matches!(arg.as_str(), "--model" | "-m") {
            return args.get(index + 1).map(String::as_str);
        }
        if let Some(value) = crate::provider_model_arguments::attached_model_argument(syntax, arg) {
            return Some(value);
        }
    }
    None
}
/// Only syntax selection from basename, never executable/agent authentication.
/// Select the established CLI argument dialect by basename only; this does not
/// authenticate an executable, provider account, capability or model.
pub(crate) fn adapter(provider: &Provider) -> &'static str {
    match Path::new(&provider.command)
        .file_name()
        .and_then(|s| s.to_str())
    {
        Some("codex") => "codex",
        Some("claude") => "claude",
        Some("skynet_harness") => "skynet",
        _ => "custom",
    }
}
/// Retained provenance facts only. Paths, full cache, credentials and unselected
/// names are not serialized into the model receipt.
#[derive(Serialize)]
struct ModelSources<'a> {
    schema_version: i64,
    requested_level: &'a str,
    automatic: bool,
    adapter: &'a str,
    cache_state: &'a str,
    cache_sha256: Option<String>,
    configured_model: Option<&'a str>,
    selected_announcement: &'a str,
    policy_hash: &'a str,
    routing_rules_sha256: String,
}
/// Build one requested route from an explicit saved provider policy and current
/// v19 restrictions. Only the selected entry needs a local/configured announcement;
/// other tiers are not forced available. No model/effort names, escalation or
/// pricing defaults are inferred. Unsupported probes/API and known adapters
/// lacking a saved policy refuse. Custom/no-route remains unchanged.
/// Cache contents are Go-compatible declarations, never account/model proof;
/// missing may use an already configured model without invented effort, while
/// invalid/read/close errors refuse. All IO is read-only and no child runs.
pub(crate) fn resolve(
    provider: &Provider,
    launch: &Launch,
    settings: &EngineSettingsValues,
    policy: &AgentLaunchPolicy,
) -> Result<ModelRoutePreparation, AgentLaunchError> {
    if !policy.allow_unverified {
        return Err(AgentLaunchError::Refused(
            "agent-provider-verified-context-required",
        ));
    }
    if !provider.api_connection_id.is_empty()
        || provider.preflight_required
        || provider
            .preflight_args
            .as_ref()
            .is_some_and(|v| !v.is_empty())
    {
        return Err(AgentLaunchError::Refused(
            "agent-provider-adapter-or-model-route-unported",
        ));
    }
    resolve_configuration_metadata(provider, launch, settings, policy)
}
/// Validate inherited policy-v1 structural vocabulary only. No model names,
/// catalogue availability, defaults adoption or execution permission is inferred.
/// Shared by runtime selection and portable policy export/import; caller still
/// checks actual selected choices and current operator restrictions.
pub(crate) fn policy_levels(
    model_policy: &crate::ModelPolicy,
) -> Result<&std::collections::BTreeMap<String, crate::ModelChoice>, AgentLaunchError> {
    let levels = model_policy
        .levels
        .as_ref()
        .ok_or(AgentLaunchError::Refused(
            "agent-model-policy-levels-required",
        ))?;
    if model_policy.version != 1
        || !matches!(model_policy.billing.as_str(), "external" | "on_premise")
        || levels.len() != 3
        || ["simple", "standard", "exigeant"]
            .iter()
            .any(|level| !levels.contains_key(*level))
        || !crate::model_routing_policy::model_level(&model_policy.work_level)
        || !crate::model_routing_policy::model_level(&model_policy.page_level)
    {
        return Err(AgentLaunchError::Refused("agent-model-policy-invalid"));
    }
    Ok(levels)
}

/// Observe configured/local model metadata for configuration validation only.
/// This deliberately does not require execution's unverified-context/probe grant:
/// storing a policy is not launching a process. API metadata remains unsupported;
/// all shape/rules/argv/catalogue/environment/cap/source checks stay shared. The
/// returned private candidate grants no admission, identity or future lease;
/// execution always goes through resolve and its unchanged guards/rechecks.
pub(crate) fn resolve_configuration_metadata(
    provider: &Provider,
    launch: &Launch,
    settings: &EngineSettingsValues,
    policy: &AgentLaunchPolicy,
) -> Result<ModelRoutePreparation, AgentLaunchError> {
    resolve_configuration_metadata_for_purpose(provider, launch, settings, policy, "work")
}

/// One locally declared model option for operator inventory. This data proves
/// neither account access nor effective effort/identity; selection still uses
/// the common resolver and current saved restrictions before admission.
#[derive(Serialize)]
pub struct ProviderModelOption {
    pub id: String,
    pub efforts: Vec<String>,
    pub source: &'static str,
}

/// Read actual compatible visible Codex cache entries and the configured argv
/// model, with no built-in vendor model list or inferred default. Cache reads
/// share the runtime's effective environment, saved per-file cap and close checks.
/// Equal IDs merge declared efforts as runtime selection uses any matching row;
/// output is sorted. Missing cache may expose an already configured model, but
/// malformed/type/IO/close failures never become a successful empty catalogue.
/// No provider call, write, availability probe or execution grant occurs.
pub(crate) fn model_options(
    provider: &Provider,
    policy: &AgentLaunchPolicy,
) -> Result<Vec<ProviderModelOption>, AgentLaunchError> {
    let adapter = adapter(provider);
    let mut options = std::collections::BTreeMap::<String, ProviderModelOption>::new();
    if adapter == "codex" {
        if let Some(path) = cache_path(provider, policy)? {
            if let Some(raw) = read_cache(&path, policy.provider_bytes)? {
                let cache: ModelCache = crate::work::decode_mode(&raw, cache_shape(), false)
                    .map_err(|e| AgentLaunchError::caused("agent-model-catalogue-invalid", e))?;
                for model in cache.models.unwrap_or_default() {
                    if model.visibility != "list" || !crate::model_routing_policy::model_policy_v1_name(&model.slug) { continue; }
                    let entry = options.entry(model.slug.clone()).or_insert_with(|| ProviderModelOption {
                        id: model.slug, efforts: Vec::new(), source: "local_cache_unverified",
                    });
                    entry.efforts.extend(model.supported_reasoning_levels.unwrap_or_default().into_iter().map(|effort| effort.effort));
                    entry.efforts.sort(); entry.efforts.dedup();
                }
            }
        }
    }
    let syntax = if adapter == "codex" { ProviderModelSyntax::Codex } else { ProviderModelSyntax::ModelAndEffort };
    if let Some(model) = configured_model(provider.args.as_deref().unwrap_or_default(), syntax).filter(|v| !v.is_empty()) {
        options.entry(model.into()).or_insert_with(|| ProviderModelOption {
            id: model.into(), efforts: Vec::new(), source: "configured_argv_unverified",
        });
    }
    Ok(options.into_values().collect())
}

/// Inherited provider-resolution purpose vocabulary, never a permission or a
/// model/policy default. This pure check is shared by public reads and resolver.
pub(crate) fn valid_purpose(purpose: &str) -> bool {
    matches!(purpose, "work" | "page" | "brainstorm" | "preparation" | "planning")
}

/// Resolve read-only local metadata for an explicit inherited purpose. Automatic
/// page selection uses saved page_level; other purposes use work_level. Retain
/// automatic=true for saved restriction checks, rather than converting a default
/// into an explicit request. All existing catalogue/argv/source guards remain;
/// neither this candidate nor a successful read grants execution or availability.
/// Invalid purpose/policy/selection, corrupt sources and IO/close errors refuse.
pub(crate) fn resolve_configuration_metadata_for_purpose(
    provider: &Provider,
    launch: &Launch,
    settings: &EngineSettingsValues,
    policy: &AgentLaunchPolicy,
    purpose: &str,
) -> Result<ModelRoutePreparation, AgentLaunchError> {
    if !valid_purpose(purpose) {
        return Err(AgentLaunchError::Refused("provider-model-purpose-invalid"));
    }
    if !provider.api_connection_id.is_empty() {
        return Err(AgentLaunchError::Refused("provider-policy-api-unported"));
    }
    let adapter = adapter(provider);
    let Some(model_policy) = provider.model_policy.as_deref() else {
        if adapter != "custom" {
            return Err(AgentLaunchError::Refused(
                "agent-model-saved-policy-required",
            ));
        }
        if !matches!(launch.level.as_str(), "" | "auto") || !launch.model_policy_hash.is_empty() {
            return Err(AgentLaunchError::Refused(
                "agent-provider-model-route-unported",
            ));
        }
        return Ok(ModelRoutePreparation {
            original: provider.clone(),
            provider: provider.clone(),
            route: None,
            scope: None,
        });
    };
    let levels = policy_levels(model_policy)?;
    let automatic = matches!(launch.level.as_str(), "" | "auto");
    let level = if automatic {
        if purpose == "page" { model_policy.page_level.as_str() }
        else { model_policy.work_level.as_str() }
    } else {
        launch.level.as_str()
    };
    let choice = levels
        .get(level)
        .ok_or(AgentLaunchError::Refused("agent-model-level-unknown"))?;
    if !crate::model_routing_policy::model_policy_v1_name(&choice.model) {
        return Err(AgentLaunchError::Refused(
            "agent-model-policy-name-v1-invalid",
        ));
    }
    let routing = settings
        .agent_model_routing_policy()
        .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
    routing
        .check_selection(level, &choice.model, automatic)
        .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
    let policy_hash = hash(
        &crate::encode_model(model_policy)
            .map_err(|e| AgentLaunchError::caused("agent-model-policy-encoding", e))?,
    );
    if !launch.model_policy_hash.is_empty() && launch.model_policy_hash != policy_hash {
        return Err(AgentLaunchError::Refused("agent-model-policy-hash-changed"));
    }
    let args = provider.args.as_deref().unwrap_or_default();
    let syntax = if adapter == "codex" {
        ProviderModelSyntax::Codex
    } else {
        ProviderModelSyntax::ModelAndEffort
    };
    let effective_args = crate::compile_provider_model_arguments(syntax, args, choice)?;
    let configured = configured_model(args, syntax);
    let mut cache_state = "not_applicable";
    let mut cache_sha256 = None;
    let mut announced = false;
    if adapter == "codex" {
        cache_state = "no_effective_home";
        if let Some(path) = cache_path(provider, policy)? {
            cache_state = "absent";
            if let Some(raw) = read_cache(&path, policy.provider_bytes)? {
                cache_state = "observed_unverified";
                cache_sha256 = Some(hash(&raw));
                let cache: ModelCache = crate::work::decode_mode(&raw, cache_shape(), false)
                    .map_err(|e| AgentLaunchError::caused("agent-model-catalogue-invalid", e))?;
                announced = cache
                    .models
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .any(|model| {
                        model.visibility == "list"
                            && model.slug == choice.model
                            && (choice.effort.is_empty()
                                || model
                                    .supported_reasoning_levels
                                    .as_deref()
                                    .unwrap_or_default()
                                    .iter()
                                    .any(|effort| effort.effort == choice.effort))
                    });
            }
        }
    }
    let selected_announcement = if announced {
        "local_cache_unverified"
    } else if configured == Some(choice.model.as_str()) && choice.effort.is_empty() {
        "configured_argv_unverified"
    } else {
        return Err(AgentLaunchError::Refused(
            "agent-model-choice-not-announced",
        ));
    };
    let rules=crate::encode_model(&serde_json::json!({"automatic":settings.agent_model_automatic_levels,"restricted":settings.agent_model_restricted_levels,"rules":settings.agent_model_restriction_rules})).map_err(|e|AgentLaunchError::caused("agent-model-rules-encoding",e))?;
    let scope = hash(
        &crate::encode_model(&ModelSources {
            schema_version: 1,
            requested_level: &launch.level,
            automatic,
            adapter,
            cache_state,
            cache_sha256,
            configured_model: configured,
            selected_announcement,
            policy_hash: &policy_hash,
            routing_rules_sha256: hash(&rules),
        })
        .map_err(|e| AgentLaunchError::caused("agent-model-scope-encoding", e))?,
    );
    let route=ModelRoute{level:level.into(),model:choice.model.clone(),effort:choice.effort.clone(),billing:model_policy.billing.clone(),policy_hash,reason:format!("Politique fournisseur enregistrée · {purpose} · sans montée en gamme automatique ; annonce non authentifiée")};
    let mut effective = provider.clone();
    effective.args = Some(effective_args);
    Ok(ModelRoutePreparation {
        original: provider.clone(),
        provider: effective,
        route: Some(route),
        scope: Some(scope),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A real checked custom-policy candidate, with no host/cache IO or child.
    /// Literal model/levels are fixture data; scope is computed by production code.
    fn candidate(routed: bool) -> ModelRoutePreparation {
        let settings = crate::EngineSettingsValues::installation_defaults().unwrap();
        let policy = settings.agent_launch_policy().unwrap();
        let choice = crate::ModelChoice {
            model: "fixture-standard".into(),
            effort: String::new(),
        };
        let provider = Provider {
            command: "/fixture/custom".into(),
            args: Some(vec!["--model".into(), "fixture-standard".into()]),
            model_policy: routed.then(|| {
                Box::new(crate::ModelPolicy {
                    version: 1,
                    billing: "external".into(),
                    work_level: "standard".into(),
                    page_level: "simple".into(),
                    levels: Some(
                        ["simple", "standard", "exigeant"]
                            .into_iter()
                            .map(|level| (level.into(), choice.clone()))
                            .collect(),
                    ),
                })
            }),
            ..Default::default()
        };
        resolve(&provider, &Launch::default(), &settings, &policy).unwrap()
    }
    /// Missing/duplicate/changed/falsely verified receipts cannot turn local
    /// model metadata into authorization; each fault has the valid same witness.
    #[test]
    fn model_source_receipt_refuses_missing_duplicate_changed_and_false_verification() {
        let candidate = candidate(true);
        let check = crate::PreflightCheck {
            name: "model_route_sources".into(),
            status: "declared_unverified".into(),
            detail: candidate.scope().unwrap().into(),
        };
        let valid = crate::PreflightResult {
            checks: Some(vec![check.clone()]),
            ..Default::default()
        };
        for checks in [
            None,
            Some(vec![]),
            Some(vec![check.clone(), check.clone()]),
            Some(vec![crate::PreflightCheck {
                detail: "different".into(),
                ..check.clone()
            }]),
            Some(vec![crate::PreflightCheck {
                status: "verified".into(),
                ..check.clone()
            }]),
        ] {
            candidate.check_recorded_scope(&valid).unwrap();
            let changed = crate::PreflightResult {
                checks,
                ..Default::default()
            };
            assert_eq!(
                candidate.check_recorded_scope(&changed).unwrap_err().code(),
                "agent-execution-model-route-source-changed"
            );
            candidate.check_recorded_scope(&valid).unwrap();
        }
    }
    /// Old unrouted custom receipts need no invented model source; adding an
    /// apparent model receipt refuses rather than silently upgrading provenance.
    #[test]
    fn unrouted_custom_receipt_has_no_invented_model_source_or_fallback() {
        let candidate = candidate(false);
        assert!(candidate.route().is_none() && candidate.scope().is_none());
        let valid = crate::PreflightResult {
            checks: Some(vec![crate::PreflightCheck {
                name: "host_observations".into(),
                status: "ready".into(),
                detail: "fixture".into(),
            }]),
            ..Default::default()
        };
        candidate.check_recorded_scope(&valid).unwrap();
        let mut changed = valid.clone();
        changed
            .checks
            .as_mut()
            .unwrap()
            .push(crate::PreflightCheck {
                name: "model_route_sources".into(),
                status: "declared_unverified".into(),
                detail: "invented".into(),
            });
        assert_eq!(
            candidate.check_recorded_scope(&changed).unwrap_err().code(),
            "agent-execution-model-route-source-changed"
        );
        candidate.check_recorded_scope(&valid).unwrap();
    }
    /// Read-only configuration metadata never grants execution when the saved
    /// provider-context policy or required probe refuses actual admission.
    #[test]
    fn configuration_model_metadata_is_not_execution_permission() {
        let candidate = candidate(true);
        let mut provider = candidate.original;
        let settings = EngineSettingsValues::installation_defaults().unwrap();
        let mut policy = settings.agent_launch_policy().unwrap();
        let launch = Launch::default();
        policy.allow_unverified = false;
        assert!(
            resolve_configuration_metadata(&provider, &launch, &settings, &policy)
                .unwrap()
                .route()
                .is_some()
        );
        assert_eq!(
            resolve(&provider, &launch, &settings, &policy)
                .err()
                .unwrap()
                .code(),
            "agent-provider-verified-context-required"
        );
        policy.allow_unverified = true;
        provider.preflight_required = true;
        provider.preflight_args = Some(vec!["fixture-probe".into()]);
        assert!(
            resolve_configuration_metadata(&provider, &launch, &settings, &policy)
                .unwrap()
                .route()
                .is_some()
        );
        assert_eq!(
            resolve(&provider, &launch, &settings, &policy)
                .err()
                .unwrap()
                .code(),
            "agent-provider-adapter-or-model-route-unported"
        );
    }
}
