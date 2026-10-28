//! Development entry point. Only migrated commands are executable; never calls Go.
#![forbid(unsafe_code)]
#[cfg(target_os = "linux")]
mod identity_control;
#[cfg(target_os = "linux")]
mod personal_secret_frame;
#[cfg(target_os = "linux")]
mod identity_personal;
#[cfg(target_os = "linux")]
mod identity_service;
#[cfg(target_os = "linux")]
mod identity_service_call;
#[cfg(target_os = "linux")]
mod trust_enrollment;
#[cfg(target_os = "linux")]
mod identity_linux_prerequisites;
#[cfg(target_os = "linux")]
mod identity_provisioning;
#[cfg(target_os = "linux")]
mod identity_operator;
#[cfg(target_os = "linux")]
mod identity_checkpoint;
#[cfg(target_os = "linux")]
mod identity_initial;

use serde::Serialize;
use std::{
    fmt,
    io::{self, Write},
};
use swarm_enrollment_bootstrap as bootstrap;
const USAGE:&str="swarm-rust init --root ROOT | work list --json --root ROOT | work show ID | resume ID | planning show|enable|claim|lease-renew|decide|judge|renew|pause|resume|reject-instructions ID | mission show|order|queue|priority|start|resume|pause|unpause|stop ID --input FILE | mission watch|status|changes|spending ID | mission status-native ID --json | costs show ID | budget show|preview|apply ID [--input FILE] | task add|update|submit|validation-preview|validation-candidate ID --input FILE | task-model show|preview|apply ID | providers config show|init|inspect|backups|preview|apply --root ROOT [--input FILE] | providers policies export|preview|apply --root ROOT [--input FILE] | pricing list|save|estimate --input FILE | engine-settings show|preview|apply|model-check --root ROOT [--input FILE] | admin-budget show|preview|apply --root ROOT [--input FILE] | identity initial-create|initial-reconcile REQUEST_SHA256 OPERATOR_DIRECTORY PRIVATE_ANCHOR --input PRIVATE_REQUEST | identity checkpoint-check REQUEST_SHA256 --input PRIVATE_ANCHOR | identity operator-check SHA256 --input PRIVATE_OPERATOR_DIRECTORY | identity service-export|service-export-reconcile SHA256 PRIVATE_OUTPUT --input PRIVATE_OPERATOR_DIRECTORY | identity provisioning-check SHA256 --input PRIVATE_REQUEST | identity bootstrap-check|control-check SHA256 --input FILE | identity personal-init|personal-enroll|personal-recover SHA256 SEED_PATH --input BOOTSTRAP | identity service-install|service-resume|service-reconcile SHA256 --input PRIVATE_ARTIFACT | identity service-call SHA256 SEED_PATH --input BOOTSTRAP | trust enrollment-call OPERATION ABSOLUTE_DECLARATION --input REQUESTFILE | trust enrollment-serve OPERATION ABSOLUTE_DECLARATION [LOOPBACK_ADDRESS] --input REQUESTFILE";
const HELP_FR:&str="swarm-rust — développement : migration partielle\nCommandes disponibles :\n  swarm-rust lifecycle list --root ROOT --json\n  swarm-rust planning history WORK --root ROOT --json\n  swarm-rust visit show|mark WORK --root ROOT [--input INTENT.json] --json\n  swarm-rust runtime-health --root ROOT --json\n  swarm-rust logs query WORK AGENT --root ROOT [--input QUERY.json] --json\n  swarm-rust run-limits logs show|history|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust activity show WORK --root ROOT [--input QUERY.json] --json\n  swarm-rust run-limits activity show|history|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust exchange list WORK --root ROOT --json\n  swarm-rust run-limits performance show|history|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust run-limits web-requests show|history|preview|apply [--input FICHIER] [--json]\n  swarm-rust run-limits show|history|preview|apply SCOPE MISSION KEY --root ROOT [--input FILE] --json\n  swarm-rust run-limits effective MISSION ROLE TASK --root ROOT --json\n  swarm-rust run-limits rollback-preview|rollback SCOPE MISSION KEY REV --root ROOT --input FILE --json\n  swarm-rust trust show|config-preview|config-apply|mandate-create|request|approve|commit|reconcile|export|anchor|verify --root RACINE [--input FICHIER|-]\n  swarm-rust version [--json] [--root ROOT]\n  swarm-rust --version [--json]\n  swarm-rust init --root ROOT [--json]\n  swarm-rust web [ADDRESS] --root ROOT [--json]\n  swarm-rust prepare send|stop|use-proposal|create|save|method|adopt-brief|validate-plan|answer-questions|create-missions|revise-missions|configure-team|release-plan|budget|source-add|source-remove [ID] --root ROOT --input FILE [--json]\n  swarm-rust prepare list|methods|capabilities --root ROOT [--json]\n  swarm-rust prepare show|budget|release-review|dialogue ID --root ROOT [--json]\n  swarm-rust prepare files ID [SCOPE [QUERY]] --root ROOT [--json]\n  swarm-rust prepare templates | template ID | template-check --input JSON\n  swarm-rust project-profile show|check|list --root ROOT [--json]\n  swarm-rust project-profile preview|apply|select [PROFILE] --root ROOT --input JSON [--json]\n  swarm-rust prepare source ID PATH START END --root ROOT [--json]\n  swarm-rust prepare history ID besoin|brief|plan [BEFORE] --root ROOT [--json]\n  swarm-rust work list --root ROOT [--json]\n  swarm-rust work show ID --root ROOT [--json]\n  swarm-rust resume ID --root ROOT [--json]\n  swarm-rust planning show ID --root ROOT\n  swarm-rust costs show ID --root ROOT [--json]\n  swarm-rust budget show|preview|apply ID --root ROOT [--input FILE] [--json]\n  swarm-rust pricing list|save|estimate --root ROOT [--input FILE] [--json]\n  swarm-rust providers config show --root ROOT [--json]\n  swarm-rust providers config init --root ROOT --input FILE [--json]\n  swarm-rust providers config inspect [SNAPSHOT] --root ROOT [--json]\n  swarm-rust run-limits provider-wait show|history|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust providers connections show|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust providers policies export|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust providers config backups --root ROOT [--json]\n  swarm-rust providers config preview|apply --root ROOT --input FILE [--json]\n  swarm-rust task review-reconcile|review-retire-stale REVIEW_ID --root ROOT [--json]\n  swarm-rust task review|review-preview|review-inputs|review-check|baseline-preview|publication-preview|publish|validation-preview|validation-candidate|validate|validation-reconcile|validation-retire ID --root ROOT --input FILE [--json]\n  swarm-rust agent run ID --root ROOT [--json]\n  swarm-rust agent show|stop|reconcile ID --root ROOT [--json]\n  swarm-rust agent logs ID [AFTER] --root ROOT [--json] [--output FILE]\n  swarm-rust agent preview|prepare WORK --root ROOT --input FILE [--json]\n  swarm-rust agent guard ID --root ROOT --input FILE [--json]\n  swarm-rust agent limits WORK ROLE TASK --root ROOT --input FILE [--json]\n  swarm-rust planning enable|claim|lease-renew|decide|judge|renew|pause|resume|reject-instructions|call-preview|run ID --root ROOT --input FILE\n  swarm-rust planning step WORK --root ROOT [--json]\n  swarm-rust planning reconcile CLAIM --root ROOT [--json]\n  swarm-rust work create|update [ID] --root ROOT --input FILE [--json]\n  swarm-rust task add|update|submit ID --root ROOT --input FILE [--json]\n  swarm-rust quotas show|preview|apply ID --root ROOT [--input FILE] [--json]\n  swarm-rust mission show|order|queue|priority|start|resume|pause|unpause|stop ID --root ROOT [--input FILE] [--json]\n  swarm-rust mission watch ID --root ROOT [--json]\n  swarm-rust mission changes|spending|seen ID --root ROOT [--json]\n  swarm-rust mission status ID --root ROOT [--json]\n  swarm-rust mission status-native ID --root ROOT --json\n  swarm-rust task-model show|preview|apply ID --root ROOT [--input FILE] --json\n  swarm-rust ooda|checkpoint ID --root ROOT --input FILE [--json]\n  swarm-rust gate ID --root ROOT --input FILE [--task TASK] [--json]\n  swarm-rust project-profile show|check --root ROOT [--json]\n  swarm-rust project-context ROLE --root ROOT [--json]\n  swarm-rust agent-workflow ROLE --root ROOT [--json]\n  swarm-rust planning-output check --root ROOT --input FILE [--json]\n  swarm-rust planning-report --root ROOT --input FILE [--json]\n  swarm-rust evaluate --root ROOT --input FILE [--phase entry|validation|delivery|audit] [--json]\n  swarm-rust admin-budget show|preview|apply --root ROOT [--input FILE] [--json]\n  swarm-rust engine-settings show|describe|quality-profile --root ROOT [--json]\n  swarm-rust engine-settings preview|apply|model-check --root ROOT --input FILE [--json]\n  swarm-rust identity linux-prerequisites --json [--lang fr|en]\n    Read-only current CLI namespace metadata; no launch or protection verified.\n  swarm-rust identity service-call SHA256 PRIVATE_SEED --input BOOTSTRAP [--json]\n    Appel opérateur local : paquet binaire sur stdin, aucun listener ou session.\n  swarm-rust identity service-install|service-resume|service-reconcile SHA256 --input ARTEFACT_PRIVE_ABSOLU [--json]\n    Installation initiale, sans compte humain ni démarrage de service.\n  swarm-rust identity personal-init|personal-enroll|personal-recover EMPREINTE_SHA256 CLE_PRIVEE --input BOOTSTRAP [--json]\n    personal-enroll : frame binaire sur stdin ; aucun secret dans argv.\n  swarm-rust identity initial-create|initial-reconcile PIN_DEMANDE DOSSIER_OPERATEUR ANCRE_PRIVEE --input DEMANDE_PRIVEE [--json]\n    Matière dormante uniquement ; intention avant génération, aucun compte/Root/service/humain admis.\n  swarm-rust identity checkpoint-check PIN_DEMANDE --input ANCRE_PRIVEE_ABSOLUE [--json]\n    Lecture uniquement ; confiance du localisateur à établir par l’appelant, aucun paquet lu/créé.\n  swarm-rust identity operator-check PIN --input DOSSIER_OPERATEUR_PRIVE [--json]\n  swarm-rust identity service-export|service-export-reconcile PIN SORTIE_PRIVEE --input DOSSIER_OPERATEUR_PRIVE [--json]\n    Artefact de service uniquement ; aucune installation, nouvelle clé/preuve ou identité humaine vérifiée.\n  swarm-rust identity provisioning-check EMPREINTE_SHA256 --input DEMANDE_PRIVEE_ABSOLUE [--json]\n    Précontrôle uniquement ; aucun secret créé, compte vérifié ou service démarré.\n  swarm-rust identity bootstrap-check EMPREINTE_SHA256 --input FILE [--json]\n  swarm-rust identity control-check EMPREINTE_SHA256 --input FILE [--json] (précontrôle, sans démarrage)\nLangue : --lang fr|en. init initialise sans service ; web lance l’API native de développement.\nMigration partielle : seules les commandes Rust listées sont actuellement disponibles.\n";
const HELP_EN:&str="swarm-rust — development: partial migration\nAvailable commands:\n  swarm-rust lifecycle list --root ROOT --json\n  swarm-rust planning history WORK --root ROOT --json\n  swarm-rust visit show|mark WORK --root ROOT [--input INTENT.json] --json\n  swarm-rust runtime-health --root ROOT --json\n  swarm-rust logs query WORK AGENT --root ROOT [--input QUERY.json] --json\n  swarm-rust run-limits logs show|history|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust activity show WORK --root ROOT [--input QUERY.json] --json\n  swarm-rust run-limits activity show|history|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust exchange list WORK --root ROOT --json\n  swarm-rust run-limits performance show|history|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust run-limits web-requests show|history|preview|apply [--input FILE] [--json]\n  swarm-rust run-limits show|history|preview|apply SCOPE MISSION KEY --root ROOT [--input FILE] --json\n  swarm-rust run-limits effective MISSION ROLE TASK --root ROOT --json\n  swarm-rust run-limits rollback-preview|rollback SCOPE MISSION KEY REV --root ROOT --input FILE --json\n  swarm-rust run-limits provider-wait show|history|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust trust show|config-preview|config-apply|mandate-create|request|approve|commit|reconcile|export|anchor|verify --root ROOT [--input FILE|-]\n  swarm-rust version [--json] [--root ROOT]\n  swarm-rust --version [--json]\n  swarm-rust init --root ROOT [--json]\n  swarm-rust web [ADDRESS] --root ROOT [--json]\n  swarm-rust prepare send|stop|use-proposal|create|save|method|adopt-brief|validate-plan|answer-questions|create-missions|revise-missions|configure-team|release-plan|budget|source-add|source-remove [ID] --root ROOT --input FILE [--json]\n  swarm-rust prepare list|methods|capabilities --root ROOT [--json]\n  swarm-rust prepare show|budget|release-review|dialogue ID --root ROOT [--json]\n  swarm-rust prepare files ID [SCOPE [QUERY]] --root ROOT [--json]\n  swarm-rust prepare templates | template ID | template-check --input JSON\n  swarm-rust project-profile show|check|list --root ROOT [--json]\n  swarm-rust project-profile preview|apply|select [PROFILE] --root ROOT --input JSON [--json]\n  swarm-rust prepare source ID PATH START END --root ROOT [--json]\n  swarm-rust prepare history ID besoin|brief|plan [BEFORE] --root ROOT [--json]\n  swarm-rust work list --root ROOT [--json]\n  swarm-rust work show ID --root ROOT [--json]\n  swarm-rust resume ID --root ROOT [--json]\n  swarm-rust planning show ID --root ROOT\n  swarm-rust costs show ID --root ROOT [--json]\n  swarm-rust budget show|preview|apply ID --root ROOT [--input FILE] [--json]\n  swarm-rust pricing list|save|estimate --root ROOT [--input FILE] [--json]\n  swarm-rust providers config show --root ROOT [--json]\n  swarm-rust providers config init --root ROOT --input FILE [--json]\n  swarm-rust providers config inspect [SNAPSHOT] --root ROOT [--json]\n  swarm-rust providers policies export|preview|apply --root ROOT [--input FILE] --json\n  swarm-rust providers config backups --root ROOT [--json]\n  swarm-rust providers config preview|apply --root ROOT --input FILE [--json]\n  swarm-rust task review-reconcile|review-retire-stale REVIEW_ID --root ROOT [--json]\n  swarm-rust task review|review-preview|review-inputs|review-check|baseline-preview|publication-preview|publish|validation-preview|validation-candidate|validate|validation-reconcile|validation-retire ID --root ROOT --input FILE [--json]\n  swarm-rust agent run ID --root ROOT [--json]\n  swarm-rust agent show|stop|reconcile ID --root ROOT [--json]\n  swarm-rust agent logs ID [AFTER] --root ROOT [--json] [--output FILE]\n  swarm-rust agent preview|prepare WORK --root ROOT --input FILE [--json]\n  swarm-rust agent guard ID --root ROOT --input FILE [--json]\n  swarm-rust agent limits WORK ROLE TASK --root ROOT --input FILE [--json]\n  swarm-rust planning enable|claim|lease-renew|decide|judge|renew|pause|resume|reject-instructions|call-preview|run ID --root ROOT --input FILE\n  swarm-rust planning step WORK --root ROOT [--json]\n  swarm-rust planning reconcile CLAIM --root ROOT [--json]\n  swarm-rust work create|update [ID] --root ROOT --input FILE [--json]\n  swarm-rust task add|update|submit ID --root ROOT --input FILE [--json]\n  swarm-rust quotas show|preview|apply ID --root ROOT [--input FILE] [--json]\n  swarm-rust mission show|order|queue|priority|start|resume|pause|unpause|stop ID --root ROOT [--input FILE] [--json]\n  swarm-rust mission watch ID --root ROOT [--json]\n  swarm-rust mission changes|spending|seen ID --root ROOT [--json]\n  swarm-rust mission status ID --root ROOT [--json]\n  swarm-rust mission status-native ID --root ROOT --json\n  swarm-rust task-model show|preview|apply ID --root ROOT [--input FILE] --json\n  swarm-rust ooda|checkpoint ID --root ROOT --input FILE [--json]\n  swarm-rust gate ID --root ROOT --input FILE [--task TASK] [--json]\n  swarm-rust project-profile show|check --root ROOT [--json]\n  swarm-rust project-context ROLE --root ROOT [--json]\n  swarm-rust agent-workflow ROLE --root ROOT [--json]\n  swarm-rust planning-output check --root ROOT --input FILE [--json]\n  swarm-rust planning-report --root ROOT --input FILE [--json]\n  swarm-rust evaluate --root ROOT --input FILE [--phase entry|validation|delivery|audit] [--json]\n  swarm-rust admin-budget show|preview|apply --root ROOT [--input FILE] [--json]\n  swarm-rust engine-settings show|describe|quality-profile --root ROOT [--json]\n  swarm-rust engine-settings preview|apply|model-check --root ROOT --input FILE [--json]\n  swarm-rust identity service-call SHA256 PRIVATE_SEED --input BOOTSTRAP [--json]\n    Local operator call: binary stdin packet, no listener or session.\n  swarm-rust identity service-install|service-resume|service-reconcile SHA256 --input ABSOLUTE_PRIVATE_ARTIFACT [--json]\n    Initial installation only; no human account or service started.\n  swarm-rust identity personal-init|personal-enroll|personal-recover SHA256_DIGEST PRIVATE_KEY --input BOOTSTRAP [--json]\n    personal-enroll: binary frame on stdin; no secrets in argv.\n  swarm-rust identity initial-create|initial-reconcile REQUEST_PIN OPERATOR_DIRECTORY PRIVATE_ANCHOR --input PRIVATE_REQUEST [--json]\n    Dormant material only; intent before generation, no account/Root/service/human admitted.\n  swarm-rust identity checkpoint-check REQUEST_PIN --input ABSOLUTE_PRIVATE_ANCHOR [--json]\n    Read only; caller must establish locator trust, no package read/creation.\n  swarm-rust identity operator-check PIN --input PRIVATE_OPERATOR_DIRECTORY [--json]\n  swarm-rust identity service-export|service-export-reconcile PIN PRIVATE_OUTPUT --input PRIVATE_OPERATOR_DIRECTORY [--json]\n    Service artifact only; no installation, new keys/proof or verified human identity.\n  swarm-rust identity provisioning-check SHA256_DIGEST --input ABSOLUTE_PRIVATE_REQUEST [--json]\n    Preflight only; no material generated, account verified or service started.\n  swarm-rust identity bootstrap-check SHA256_DIGEST --input FILE [--json]\n  swarm-rust identity control-check SHA256_DIGEST --input FILE [--json] (preflight, no service start)\nLanguage: --lang fr|en. init starts no service; web starts the native development API.\nPartial migration: only the listed Rust commands are currently available.\n";
pub enum CliError {
    Trust(Box<swarm_engine::TrustError>),
    TrustPublication { primary:Box<Self>, publication: &'static str },
    AgentExecutionPreserved {
        primary: Box<Self>,
        english: bool,
    },
    AgentLaunchPreserved {
        primary: Box<CliError>,
        english: bool,
    },
    AgentCancellationPreserved {
        primary: Box<CliError>,
        english: bool,
    },
    AgentStopPreserved {
        primary: Box<CliError>,
        english: bool,
    },
    Usage,
    MissingValue {
        option: String,
        english: bool,
    },
    Language(String),
    Unsupported,
    #[cfg(target_os = "linux")]
    NativeWeb {
        cause: Box<swarm_web::NativeServerError>,
        english: bool,
    },
    /// Session observation from the private worker bootstrap, never a grant or
    /// proof that an Agent/provider started. The original runtime error remains
    /// typed, including known lifecycle state, close and output failures.
    #[cfg(target_os = "linux")]
    MissionWorkerEntry {
        primary: Box<CliError>,
        session_id: Option<i32>,
        english: bool,
    },
    PlanningUnsupported {
        english: bool,
    },
    /// A task-control command recorded Work before a later refusal/failure.
    /// This metadata preserves readback context, never success or retry authority.
    TaskValidationStatePreserved {
        primary: Box<CliError>, work: String, revision: i64, english: bool,
    },
    PlanningStatePreserved {
        primary: Box<CliError>,
        work: String,
        revision: i64,
        english: bool,
    },
    ConfigurationStatePreserved {
        primary: Box<CliError>,
        english: bool,
    },
    ArgumentEncoding,
    Bootstrap(bootstrap::Error),
    /// Pinned current-source preflight failure; no project/grant/config mutation.
    #[cfg(target_os = "linux")]
    ControlCheck { cause: Box<swarm_enrollment_authority::startup::Error>, english: bool },
    /// Personal lifecycle failure retains known publication and independent close/output causes.
    #[cfg(target_os = "linux")]
    PersonalLifecycle { cause: Box<identity_personal::Error>, english: bool },
    /// Installer failures retain input validation, root publication and output causes.
    #[cfg(target_os = "linux")]
    ServiceInstallation { cause: Box<identity_service::Error>, english: bool },
    /// Operator call retains operation, independent close and output causes.
    #[cfg(target_os = "linux")]
    ServiceCall { cause: Box<identity_service_call::Error>, english: bool },
    /// Pinned request preflight retains observed read/validation and cleanup/output causes.
    #[cfg(target_os = "linux")]
    ProvisioningCheck { cause: Box<identity_provisioning::Error>, english: bool },
    /// Private operator package/handoff errors retain source and target/output phases.
    #[cfg(target_os = "linux")]
    OperatorPackage { cause: Box<identity_operator::Error>, english: bool },
    /// Protected-locator anchor reading retains actual FS/decode/close/output facts.
    #[cfg(target_os = "linux")]
    CheckpointCheck { cause: Box<identity_checkpoint::Error>, english: bool },
    /// Dormant initial material retains durable intent/anchor/package and output facts.
    #[cfg(target_os = "linux")]
    InitialCreation { cause: Box<identity_initial::Error>, english: bool },
    Root(Box<swarm_engine::RootStoreError>),
    Evaluation(Box<swarm_engine::GateEvaluationError>),
    SettingsInput(serde_json::Error),
    /// Offline PAEA data failure, with no opened Root or effect authority.
    Protection(swarm_protection_contract::Error),
    Input(io::Error),
    CommandInput(swarm_engine::CommandInputError),
    /// Typed auxiliary receipt kept beside an actual later close/output error.
    /// No new write, business acceptance, peer receipt or retry grant follows.
    AuxiliaryState { primary:Box<CliError>, effects:Vec<swarm_engine::AuxiliaryEffect>, english:bool },
    RootCleanup {
        primary: Box<CliError>,
        cleanup: Box<swarm_engine::RootStoreError>,
    },
    View(swarm_engine::WorkViewError),
    GateInput(swarm_engine::GateRequestError),
    GateTaskMismatch {
        english: bool,
    },
    StatePreserved {
        primary: Box<CliError>,
        work: String,
        revision: i64,
        english: bool,
    },
    /// An export handle may have been opened before publication failed. The
    /// primary write/sync and independent close errors remain typed. `opened`
    /// confirms only a descriptor, not complete bytes or rollback. No retry grant.
    OutputFile {
        primary: Option<Box<CliError>>,
        close: Option<Box<CliError>>,
        opened: bool,
        english: bool,
    },
    Output(io::Error),
}
impl CliError {
    /// Collect typed causes through CLI and native engine wrapper matrices,
    /// preserving secondary cleanup and custom IO errors. Wide
    /// error identity prevents source cycles without conflating transparent wrappers.
    /// This observation never infers success or retries an operation.
    fn typed_causes(&self) -> Vec<&(dyn std::error::Error + 'static)> {
        use std::error::Error;
        let mut pending: Vec<&(dyn Error + 'static)> = vec![self];
        let mut seen: Vec<*const dyn Error> = vec![];
        let mut causes = vec![];
        while let Some(error) = pending.pop() {
            let pointer = error as *const dyn Error;
            if seen.iter().any(|p| std::ptr::eq(*p, pointer)) {
                continue;
            }
            seen.push(pointer);
            causes.push(error);
            if let Some(error)=error.downcast_ref::<Box<Self>>(){pending.push(error.as_ref());}
            #[cfg(target_os = "linux")]
            pending.extend(native_web::branches(error));
            // source() alone loses secondary Root cleanup causes. Reuse the
            // engine's established typed wrapper matrix for every CLI consumer,
            // including known-effect discovery through late web failures.
            #[cfg(target_os = "linux")]
            pending.extend(swarm_engine::native_error_links(error).children.into_iter().map(|(_, cause)| cause));
            if let Some(Self::RootCleanup { cleanup, .. }) = error.downcast_ref::<Self>() {
                pending.push(cleanup.as_ref());
            }
            if let Some(Self::OutputFile { primary, close, .. }) = error.downcast_ref::<Self>() {
                if let Some(cause) = primary { pending.push(cause.as_ref()); }
                if let Some(cause) = close { pending.push(cause.as_ref()); }
            }
            #[cfg(target_os = "linux")]
            pending.extend(identity_control::error_links(error).1);
            #[cfg(target_os = "linux")]
            pending.extend(identity_personal::error_links(error).1);
            #[cfg(target_os = "linux")]
            pending.extend(identity_service::error_links(error).1);
            #[cfg(target_os = "linux")]
            pending.extend(identity_service_call::error_links(error).1);
            #[cfg(target_os = "linux")]
            pending.extend(identity_provisioning::error_links(error).1);
            #[cfg(target_os = "linux")]
            pending.extend(identity_operator::error_links(error).1);
            #[cfg(target_os = "linux")]
            pending.extend(identity_checkpoint::error_links(error).1);
            #[cfg(target_os = "linux")]
            pending.extend(identity_initial::error_links(error).1);
            // IO source() skips its custom error itself. Retain that typed node,
            // as the established safe diagnostic traversal already does.
            if let Some(io) = error.downcast_ref::<io::Error>() {
                if let Some(inner) = io.get_ref() {
                    pending.push(inner);
                    continue;
                }
            }
            if let Some(source) = error.source() {
                pending.push(source);
            }
        }
        causes
    }
    /// Recover retained trust outcome through all supported error wrappers.
    fn trust_publication(&self)->Option<&'static str>{self.typed_causes().into_iter().find_map(|e|{if let Some(Self::TrustPublication{publication,..})=e.downcast_ref::<Self>(){Some(*publication)}else{e.downcast_ref::<swarm_engine::RootStoreError>().and_then(|e|e.trust_publication())}})}
    /// Find known preparation receipts through CLI/Root/IO cleanup wrappers.
    /// No reread or retry occurs; historical revision is not current authority.
    fn recorded_preparation(&self)->Option<(&str,i64)>{
        for error in self.typed_causes(){
            if let Some(root)=error.downcast_ref::<swarm_engine::RootStoreError>(){if let Some(receipt)=root.recorded_preparation(){return Some(receipt);}}
            #[cfg(target_os="linux")]
            if let Some(web)=error.downcast_ref::<swarm_web::HttpServiceError>(){if let Some(receipt)=web.recorded_preparation(){return Some(receipt);}}
        }None
    }
    /// Collect actual auxiliary witnesses across CLI/Root/IO and pending Web
    /// outcomes. Exact duplicates only; an absent witness never proves rollback.
    /// These data do not grant acceptance, execution or unchanged retries.
    fn auxiliary_effects(&self)->Vec<swarm_engine::AuxiliaryEffect>{
        let mut result=Vec::new();
        for error in self.typed_causes(){
            if let Some(Self::AuxiliaryState{effects,..})=error.downcast_ref::<Self>(){swarm_engine::extend_auxiliary_effects(&mut result,effects.clone());}
            swarm_engine::extend_auxiliary_effects(&mut result,swarm_engine::auxiliary_effects(error));
            #[cfg(target_os="linux")]
            if let Some(error)=error.downcast_ref::<swarm_web::NativeServerError>(){swarm_engine::extend_auxiliary_effects(&mut result,error.auxiliary_effects());}
            #[cfg(target_os="linux")]
            if let Some(error)=error.downcast_ref::<swarm_web::HttpServiceError>(){swarm_engine::extend_auxiliary_effects(&mut result,error.auxiliary_effects());}
            #[cfg(target_os="linux")]
            if let Some(error)=error.downcast_ref::<swarm_web::NativeObservationFailure>(){swarm_engine::extend_auxiliary_effects(&mut result,error.auxiliary_effects());}
        }result
    }
    /// Recover known Work metadata from explicit CLI state-preservation wrappers
    /// and typed Root/native outcomes, including nested cleanup failures. Never
    /// parse human messages or infer revision from an output error. None means
    /// unknown, not rollback; the original typed failure remains authoritative.
    fn recorded_work(&self) -> Option<(String, i64)> {
        self.typed_causes().into_iter().find_map(|error| {
            if let Some(Self::TaskValidationStatePreserved { work, revision, .. } | Self::PlanningStatePreserved { work, revision, .. } | Self::StatePreserved { work, revision, .. }) = error.downcast_ref::<Self>() {
                return Some((work.clone(), *revision));
            }
            if let Some(root) = error.downcast_ref::<swarm_engine::RootStoreError>() {
                return root
                    .recorded_work()
                    .map(|(id, revision)| (id.to_owned(), revision));
            }
            #[cfg(target_os = "linux")]
            if let Some(web) = error.downcast_ref::<swarm_web::NativeServerError>() {
                return web
                    .recorded_work()
                    .map(|(id, revision)| (id.to_owned(), revision));
            }
            None
        })
    }
    /// Inspect typed saved/visible configuration state, not an error string.
    /// A false observation is unknown/unrecorded, never proof of rollback.
    fn configuration_may_be_recorded(&self) -> bool {
        self.typed_causes().into_iter().any(|error| {
            if matches!(
                error.downcast_ref::<Self>(),
                Some(Self::ConfigurationStatePreserved { .. })
            ) || error
                .downcast_ref::<swarm_engine::RootStoreError>()
                .is_some_and(swarm_engine::RootStoreError::configuration_may_be_recorded)
            {
                return true;
            }
            #[cfg(target_os = "linux")]
            if let Some(web) = error.downcast_ref::<swarm_web::NativeServerError>() {
                return web.configuration_may_be_recorded();
            }
            false
        })
    }
    /// Return the stable safe failure family, retaining the primary family after
    /// an already-recorded lifecycle/output failure. This pure classification
    /// grants no retry or permission and never includes arbitrary cause text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::OutputFile { primary, close, .. } => primary.as_ref().or(close.as_ref()).map_or("command_failed", |e| e.code()),
            Self::Output(e) if e.raw_os_error() == Some(28) => "storage_unavailable",
            Self::Trust(e)=>e.code(),
            Self::TrustPublication{primary,..}=>primary.code(),
            Self::Root(e) => e.code(),
            #[cfg(target_os = "linux")]
            Self::NativeWeb { cause, .. } => cause.code(),
            #[cfg(target_os = "linux")]
            Self::MissionWorkerEntry { primary, session_id, .. } => if session_id.is_some() { primary.code() } else { "mission-worker-session-unavailable" },
            Self::ConfigurationStatePreserved { primary, .. } => primary.code(),
            Self::AgentStopPreserved { primary, .. } => primary.code(),
            Self::AgentCancellationPreserved { primary, .. } => primary.code(),
            Self::AgentExecutionPreserved { primary, .. } => primary.code(),
            Self::AgentLaunchPreserved { primary, .. } => primary.code(),
            Self::Unsupported | Self::PlanningUnsupported { .. } => "command_not_ported",
            Self::PlanningStatePreserved { primary, .. } => primary.code(),
            Self::TaskValidationStatePreserved { primary, .. } => primary.code(),
            Self::View(e) => e.code(),
            Self::GateInput(e) => e.code(),
            Self::GateTaskMismatch { .. } => "gate-task-mismatch",
            Self::StatePreserved { primary, .. } | Self::AuxiliaryState {primary,..} => primary.code(),
            Self::Evaluation(e) => e.code(),
            #[cfg(target_os = "linux")]
            Self::ControlCheck { cause, .. } => cause.code(),
            #[cfg(target_os = "linux")]
            Self::PersonalLifecycle { cause, .. } => cause.code(),
            #[cfg(target_os = "linux")]
            Self::ServiceInstallation { cause, .. } => cause.code(),
            #[cfg(target_os = "linux")]
            Self::ServiceCall { cause, .. } => cause.code(),
            #[cfg(target_os = "linux")]
            Self::ProvisioningCheck { cause, .. } => cause.code(),
            #[cfg(target_os = "linux")]
            Self::OperatorPackage { cause, .. } => cause.code(),
            #[cfg(target_os = "linux")]
            Self::CheckpointCheck { cause, .. } => cause.code(),
            #[cfg(target_os = "linux")]
            Self::InitialCreation { cause, .. } => cause.code(),
            Self::SettingsInput(_) => "settings-request-invalid",
            Self::Protection(e) => e.code(),
            Self::Input(_) => "input-unavailable",
            Self::CommandInput(e) => e.code(),
            Self::RootCleanup { .. } => "root-cleanup",
            _ => "command_failed",
        }
    }
    /// Render the supported human diagnostic, including language and recorded
    /// state where known. Domain errors retain their safe Display contract;
    /// original typed causes remain in the error chain, not interpolated payloads.
    /// This observation neither retries work nor establishes task acceptance.
    fn message(&self) -> String {
        match self {
            #[cfg(target_os = "linux")]
            Self::MissionWorkerEntry { primary, session_id: Some(sid), english } => if *english { format!("Independent session {sid} established; agent runtime failed: {primary}. Session creation does not prove worker entry; inspect the original state before recovery.") } else { format!("Session indépendante {sid} établie ; échec du runtime agent : {primary}. La création de session ne prouve pas l’entrée du worker ; relire l’état original avant reprise.") },
            #[cfg(target_os = "linux")]
            Self::MissionWorkerEntry { primary, session_id: None, english } => if *english { format!("Independent worker session not established; agent runtime was not called: {primary}.") } else { format!("Session indépendante du worker non établie ; runtime agent non appelé : {primary}.") },
            Self::AgentExecutionPreserved {primary,english} => if *english {format!("Process settled; output/close failed: {primary}. Re-read the agent before replay; the task is not accepted.")}else{format!("Processus clôturé ; échec de sortie/fermeture : {primary}. Relire l’agent avant rejeu ; tâche non acceptée.")},
            Self::AgentLaunchPreserved {primary,english} => if *english {format!("Launch intention is recorded; output/close failed: {primary}. Re-read state before replay; no process has been started.")}else{format!("Intention de lancement enregistrée ; échec de sortie/fermeture : {primary}. Relire avant rejeu ; aucun processus démarré.")},
            Self::AgentCancellationPreserved {primary,english} => if *english {format!("Cancellation is recorded; output/close failed: {primary}. Re-read state; task acceptance is not established.")}else{format!("Annulation enregistrée ; échec de sortie/fermeture : {primary}. Relire l’état ; la tâche n’est pas validée par l’annulation.")},
            Self::AgentStopPreserved {primary,english} => if *english {format!("Stop intention is recorded; output/close failed: {primary}. Re-read state; termination is not confirmed.")}else{format!("Demande d’arrêt enregistrée ; échec de sortie/fermeture : {primary}. Relire l’état ; fin du processus non confirmée.")},
            Self::ConfigurationStatePreserved {primary,english} => if *english {format!("Configuration is recorded; output/close failed: {primary}. Re-read before retrying.")}else{format!("Configuration enregistrée ; échec de sortie/fermeture : {primary}. Relire avant de reprendre.")},
            Self::OutputFile { opened, english, .. } => match (*opened, *english) {
                (true,true) => "Output handle opened; publication or close failed. The file may be partial. Inspect it before retrying.",
                (false,true) => "Output handle not obtained. File outcome is unconfirmed; inspect the destination before retrying.",
                (true,false) => "Descripteur de sortie ouvert ; publication ou fermeture en échec. Le fichier peut être partiel. Vérifier avant reprise.",
                (false,false) => "Descripteur de sortie non obtenu. État du fichier non confirmé ; vérifier la destination avant reprise.",
            }.into(),
            Self::Usage => USAGE.into(),
            Self::ArgumentEncoding => "arguments must be valid UTF-8".into(),
            Self::MissingValue { option, english } => format!("{} {option}", if *english { "missing value:" } else { "valeur manquante :" }),
            Self::Language(v) => v.clone(),
            Self::Unsupported => "Commande non encore portée dans ce binaire Rust de développement.".into(),
            Self::PlanningUnsupported { english } => if *english { "This planning action is not yet ported.".into() } else { "Cette action de planification n’est pas encore portée.".into() },
            Self::TaskValidationStatePreserved { primary, work, revision, english } => if *english {
                format!("Task control recorded Work {work} revision {revision}; command failed: {primary}. Read stored state and original receipts before recovery; do not rerun the control blindly.")
            } else {format!("Contrôle de tâche : travail {work} révision {revision} enregistré ; échec de commande : {primary}. Relire l’état et les reçus avant reprise ; ne pas relancer aveuglément le contrôle.")},
            Self::PlanningStatePreserved { primary, work, revision, english } => if *english {
                format!("Work {work} revision {revision} is recorded; output/close failed: {primary}. Inspect stored state before retrying the control.")
            } else { format!("Travail {work} révision {revision} enregistré ; échec de sortie/fermeture : {primary}. Vérifier l’état enregistré avant de relancer le contrôle.") },
            #[cfg(target_os = "linux")]
            Self::InitialCreation { cause, english } => if *english {format!("Initial material command failed ({}); {}. Preserve intent/anchor/package and use initial-reconcile; never regenerate implicitly.",cause.code(),cause.observation())}else{format!("Échec de commande de matière initiale ({}) ; {}. Conserver intention/ancre/paquet et utiliser initial-reconcile ; jamais de régénération implicite.",cause.code(),cause.observation())},
            #[cfg(target_os = "linux")]
            Self::CheckpointCheck { cause, english } => if *english {format!("Private anchor check failed ({}); {}. Inspect protected locator and retained read phases.",cause.code(),cause.observation())}else{format!("Échec du contrôle d’ancre privée ({}) ; {}. Vérifier le localisateur protégé et les phases de lecture conservées.",cause.code(),cause.observation())},
            #[cfg(target_os = "linux")]
            Self::OperatorPackage { cause, english } => if *english {
                format!("Operator package command failed ({}); {}. Inspect retained phases and reconcile from the independent package pin.",cause.code(),cause.observation())
            } else {format!("Échec de commande du paquet opérateur ({}) ; {}. Consulter les phases conservées et réconcilier depuis le pin indépendant du paquet.",cause.code(),cause.observation())},
            #[cfg(target_os = "linux")]
            Self::ProvisioningCheck { cause, english } => if *english {
                format!("Provisioning request check failed ({}); {}. No material generated or service started.",cause.code(),cause.observation())
            } else {format!("Échec du précontrôle de préparation ({}); {}. Aucun secret créé ni service démarré.",cause.code(),cause.observation())},
            #[cfg(target_os = "linux")]
            Self::ServiceCall { cause, english } => if *english {
                format!("Personal service operation/output/close failed ({}); {}. Inspect known state before any retry.",cause.code(),cause.observation())
            } else {format!("Échec d’opération/sortie/fermeture du noyau personnel ({}) ; {}. Consulter l’état connu avant toute reprise.",cause.code(),cause.observation())},
            #[cfg(target_os = "linux")]
            Self::ServiceInstallation { cause, english } => if *english {
                format!("Service installation command failed ({}); {}. Inspect retained phases before explicit recovery.",cause.code(),cause.observation())
            } else {format!("Échec de commande d’installation du service ({}) ; {}. Consulter les phases conservées avant reprise explicite.",cause.code(),cause.observation())},
            #[cfg(target_os = "linux")]
            Self::PersonalLifecycle { cause, english } => if *english {
                format!("Personal lifecycle command failed ({}); {}. Inspect the retained checkpoint before recovery.",cause.code(),cause.observation())
            } else {format!("Échec de commande du lifecycle personnel ({}) ; {}. Relire le checkpoint conservé avant reprise.",cause.code(),cause.observation())},
            #[cfg(target_os = "linux")]
            Self::ControlCheck { cause, english } => if *english {
                format!("Authority startup check refused ({}); no request authorized or service started.", cause.code())
            } else { format!("Contrôle de démarrage de l’autorité refusé ({}) ; aucune requête autorisée ni service démarré.", cause.code()) },
            Self::Bootstrap(e) => match e.code() {
                "document" => "enrollment bootstrap: invalid canonical declaration".into(),
                "pin" => "enrollment bootstrap: independently approved digest required".into(),
                "path" => "enrollment bootstrap: protected service-owned storage required".into(),
                _ => e.to_string(),
            },
            Self::Trust(e)=>e.to_string(),
            Self::TrustPublication{primary,publication}=>format!("trust publication {publication}; inspect recorded state: {primary}"),
            Self::Root(e) => e.to_string(),
            #[cfg(target_os="linux")]
            Self::NativeWeb{cause,english}=>if *english{format!("Native HTTP API stopped or refused ({}); inspect saved state before replay.",cause.code())}else{format!("API HTTP native arrêtée ou refusée ({}) ; vérifier l’état sauvegardé avant rejeu.",cause.code())},
            Self::View(e) => e.to_string(),
            Self::GateInput(e) => e.to_string(),
            Self::GateTaskMismatch{english} => if *english{"gate: --task must match the input task_id"}else{"gate : --task doit correspondre au task_id du document"}.into(),
            Self::StatePreserved{primary,work,revision,english} => if *english {
                format!("Work {work} revision {revision} is recorded; view/output failed: {primary}. Inspect the stored state before retrying the mutation.")
            } else {format!("Travail {work} révision {revision} enregistré ; échec de fiche/sortie : {primary}. Vérifier l’état enregistré avant de relancer la mutation.")},
            Self::Evaluation(e) => e.to_string(),
            Self::SettingsInput(_) => "engine settings: invalid request document".into(),
            Self::Protection(e) => e.to_string(),
            Self::Input(e) => format!("input failed ({:?}, errno {:?})",e.kind(),e.raw_os_error()),
            Self::CommandInput(e)=>e.to_string(),
            Self::AuxiliaryState{primary,effects,english}=>{
                let uncertain=effects.iter().any(|effect|match effect {swarm_engine::AuxiliaryEffect::SessionVisit{state,..}=>*state==swarm_engine::AuxiliaryEffectState::Uncertain});
                let notice=match (*english,uncertain){(true,true)=>"A consultation may already be recorded. Read its state before retrying.",(true,false)=>"Consultation recorded. Read its state before retrying.",(false,true)=>"Une consultation peut déjà être enregistrée. Relisez son état avant de reprendre.",(false,false)=>"Consultation enregistrée. Relisez son état avant de reprendre."};format!("{notice} {}",primary.message())
            },
            Self::RootCleanup{..}=>"root cleanup failed; inspect the current state before recovery".into(),
            Self::Output(e) if e.raw_os_error() == Some(28) => "Écriture impossible : stockage plein. Libérez de l’espace puis vérifiez l’état de la mission avant de reprendre ; aucun résultat n’est validé par cette erreur.".into(),
            Self::Output(e) => format!("output failed ({:?}, errno {:?})", e.kind(), e.raw_os_error()),
        }
    }
}
impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
impl std::error::Error for CliError {
    /// Retain the original primary error, including a private bootstrap's known
    /// session observation. Independent cleanup branches remain traversable by
    /// typed_causes/safe_causes; source alone does not enumerate them all.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConfigurationStatePreserved { primary, .. } => Some(primary.as_ref()),
            Self::AgentStopPreserved { primary, .. } => Some(primary.as_ref()),
            Self::AgentCancellationPreserved { primary, .. } => Some(primary.as_ref()),
            Self::AgentExecutionPreserved { primary, .. } => Some(primary.as_ref()),
            Self::AgentLaunchPreserved { primary, .. } => Some(primary.as_ref()),
            Self::Bootstrap(e) => Some(e),
            #[cfg(target_os = "linux")]
            Self::ControlCheck { cause, .. } => Some(cause.as_ref()),
            #[cfg(target_os = "linux")]
            Self::PersonalLifecycle { cause, .. } => Some(cause.as_ref()),
            #[cfg(target_os = "linux")]
            Self::ServiceInstallation { cause, .. } => Some(cause.as_ref()),
            #[cfg(target_os = "linux")]
            Self::ServiceCall { cause, .. } => Some(cause.as_ref()),
            #[cfg(target_os = "linux")]
            Self::ProvisioningCheck { cause, .. } => Some(cause.as_ref()),
            #[cfg(target_os = "linux")]
            Self::OperatorPackage { cause, .. } => Some(cause.as_ref()),
            #[cfg(target_os = "linux")]
            Self::CheckpointCheck { cause, .. } => Some(cause.as_ref()),
            #[cfg(target_os = "linux")]
            Self::InitialCreation { cause, .. } => Some(cause.as_ref()),
            Self::Trust(e)=>Some(e.as_ref()),
            Self::TrustPublication{primary,..}=>Some(primary.as_ref()),
            Self::Root(e) => Some(e.as_ref()),
            #[cfg(target_os = "linux")]
            Self::NativeWeb { cause, .. } => Some(cause.as_ref()),
            #[cfg(target_os = "linux")]
            Self::MissionWorkerEntry { primary, .. } => Some(primary.as_ref()),
            Self::View(e) => Some(e),
            Self::GateInput(e) => Some(e),
            Self::StatePreserved { primary, .. } | Self::AuxiliaryState {primary,..} => Some(primary.as_ref()),
            Self::PlanningStatePreserved { primary, .. } => Some(primary.as_ref()),
            Self::TaskValidationStatePreserved { primary, .. } => Some(primary.as_ref()),
            Self::Evaluation(e) => Some(e.as_ref()),
            Self::SettingsInput(e) => Some(e),
            Self::Protection(e) => Some(e),
            Self::Input(e) => Some(e),
            Self::CommandInput(e) => Some(e),
            Self::RootCleanup { primary, .. } => Some(primary.as_ref()),
            Self::OutputFile { primary, close, .. } => primary.as_ref().or(close.as_ref()).map(|e| e.as_ref() as &(dyn std::error::Error + 'static)),
            Self::Output(e) => Some(e),
            _ => None,
        }
    }
}
/// Detailed causes are for authorized diagnostics, not direct public rendering.
pub struct RunOutcome {
    pub code: u8,
    pub error: Option<CliError>,
    pub diagnostic_error: Option<io::Error>,
}
#[derive(Serialize)]
struct BootstrapCheck {
    schema_version: i64,
    bootstrap_digest: String,
    bootstrap: bootstrap::Document,
    storage_verified: bool,
    human_authentication_verified: bool,
    confinement_verified: bool,
    service_started: bool,
}
#[derive(Serialize)]
struct Failure<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    causes: Vec<String>,
}
/// Complete and flush one prepared result. A transport failure retains its IO
/// cause and may follow partial bytes; callers must keep committed state explicit.
fn write_output(out: &mut dyn Write, bytes: &[u8]) -> Result<(), CliError> {
    out.write_all(bytes)
        .and_then(|()| out.flush())
        .map_err(CliError::Output)
}
/// Encode before output, preserving opaque JSON lexemes and Go public escaping.
/// Serialization/syntax failures retain their causes without a partial result.
fn json_bytes(value: &impl Serialize) -> io::Result<Vec<u8>> {
    let raw = swarm_engine::encode_public_model(value).map_err(io::Error::other)?;
    swarm_engine::pretty_public_json(&raw).map_err(io::Error::other)
}
struct Arguments {
    pos: Vec<String>,
    input: String,
    root: String,
    output: String,
    task: String,
    phase: String,
    server: bool,
    json: bool,
    help: bool,
    language: String,
}
/// Parse language and command arguments without IO. Executable identity routes
/// accept each input/json/lang option at most once and reject project/unknown
/// flags even when their values equal defaults. Other command grammars retain
/// their existing consumers; public help remains a non-executing observation.
fn parse(args: &[String], environment_language: &str) -> Result<Arguments, CliError> {
    let mut language = if environment_language.is_empty() {
        "fr".into()
    } else {
        environment_language.to_owned()
    };
    let mut filtered = Vec::new();
    let mut identity_flags = Vec::new();
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == "--lang" {
            identity_flags.push("--lang".to_owned());
            language = iter
                .next()
                .ok_or_else(|| CliError::Language("--lang requires fr or en".into()))?
                .clone();
        } else if let Some(v) = a.strip_prefix("--lang=") {
            identity_flags.push("--lang".to_owned());
            language = v.into();
        } else {
            filtered.push(a.clone());
        }
    }
    if language != "fr" && language != "en" {
        return Err(CliError::Language(format!(
            "unsupported language {language:?}; use fr or en"
        )));
    }
    let mut result = Arguments {
        pos: Vec::new(),
        input: String::new(),
        root: ".".into(),
        output: String::new(),
        task: String::new(),
        phase: "delivery".into(),
        server: false,
        json: false,
        help: false,
        language,
    };
    let mut iter = filtered.into_iter();
    while let Some(a) = iter.next() {
        // Record option names at their producer, never values or secret input.
        if a.starts_with('-') { identity_flags.push(a.clone()); }
        match a.as_str() {
            "--json" => result.json = true,
            "--version" => result.pos.push("version".into()),
            "--help" | "-h" => {
                result.help = true;
                return Ok(result);
            }
            "--root" | "--input" | "--output" | "--task" | "--phase" | "--server" => {
                let value = iter.next().ok_or_else(|| CliError::MissingValue {
                    option: a.clone(),
                    english: result.language == "en",
                })?;
                match a.as_str() {
                    "--root" => result.root = value,
                    "--input" => result.input = value,
                    "--output" => result.output = value,
                    "--task" => result.task = value,
                    "--phase" => result.phase = value,
                    "--server" => result.server = true,
                    _ => unreachable!(),
                }
            }
            _ => result.pos.push(a),
        }
    }
    if result.pos.first().is_some_and(|p|p=="identity") && result.pos.get(1).is_some_and(|p|p=="linux-prerequisites") && identity_flags.iter().any(|f|f=="--input") {
        return Err(CliError::Usage);
    }
    if result.pos.first().is_some_and(|p|p=="identity") && !closed_identity_options(&identity_flags) {
        return Err(CliError::Usage);
    }
    Ok(result)
}
/// Canonical identity option names, not execution policy. Preserve explicit
/// presence so default-valued forbidden flags cannot be silently ignored; reject
/// ambiguous duplicates. No value, path, datastore or identity is inspected.
fn closed_identity_options(flags:&[String])->bool {
    let mut seen=std::collections::HashSet::new();
    flags.iter().all(|f| matches!(f.as_str(),"--input"|"--json"|"--lang") && seen.insert(f.as_str()))
}
mod version;
mod trust;
mod protection;
mod agent;
mod agent_logs;
#[cfg(target_os = "linux")]
mod native_web;
#[cfg(target_os = "linux")]
mod provider_configuration;
#[cfg(target_os = "linux")]
mod provider_policy;
#[cfg(target_os="linux")]
mod provider_connections;
#[cfg(target_os="linux")]
mod provider_wait;
mod log_query;
#[cfg(target_os = "linux")]
mod runtime_health;
#[cfg(target_os="linux")]
mod session_visit;
#[cfg(target_os="linux")]
mod planning_history;
#[cfg(target_os="linux")]
mod lifecycle_inventory;
mod log_read_policy;
mod activity;
mod activity_policy;
mod exchange;
mod graph_performance;
mod web_request_policy;
mod run_limits_admin;
#[cfg(target_os = "linux")]
mod admin_budget;
#[cfg(target_os = "linux")]
mod task_validation;
mod task_review;
#[cfg(target_os = "linux")]
mod task_model;
#[cfg(target_os = "linux")]
mod quotas;
#[cfg(target_os = "linux")]
mod mission;
#[cfg(target_os = "linux")]
mod mission_watch;
#[cfg(target_os = "linux")]
mod mission_status;
#[cfg(target_os = "linux")]
mod mission_worker_entry;
#[cfg(target_os = "linux")]
mod costs;
/// Dispatch validated native operations. Context/workflow observations share
/// strict read-only arguments and checked serialization/close/write/flush; paired
/// primary/release failures keep both causes. Planning-report decodes bounded
/// untrusted event data, then reads under its saved cap without acknowledgement.
/// Guidance and report reads create no agent or
/// identity and grant no authority. Other handlers retain their own transaction
/// and persisted policy contracts; unknown operations explicitly refuse.
/// Explicit Linux agent run owns foreground execution; the private helper
/// still requires its live gate capability. Help/read handlers do not launch.
/// The separate private mission-worker entry only establishes a process session
/// before reusing that Agent route; recognition of its name grants no permission.
fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    // Recognize this private route before help so malformed bootstrap options
    // cannot bypass its pure validation. Recognition itself grants no runtime right.
    #[cfg(target_os = "linux")]
    if a.pos.first().is_some_and(|op| op == mission_worker_entry::OPERATION) {
        return mission_worker_entry::execute(a, out);
    }
    if a.help || a.pos.is_empty() {
        write_output(
            out,
            if a.language == "en" {
                HELP_EN.as_bytes()
            } else {
                HELP_FR.as_bytes()
            },
        )?;
        write_output(out, if a.language == "en" {
            b"  swarm-rust protection canonicalize --input FILE|- --json (offline data only; no authority)\n  swarm-rust protection inspect --input FILE|- --json (exact action content; no approval or effect)\n  swarm-rust protection check-approval ACTION APPROVAL --json (supplied binding correspondence; no approval or effect)\n  swarm-rust protection compare BEFORE AFTER --json (two files; no authority)\n  swarm-rust protection resolve-payment REGISTRY REQUEST --json (offline debt resolution; no authority)\n  swarm-rust protection check-receipt ACTION RECEIPT SINK PREVIOUS_STATE --json (two files; no authenticity or effect established)\n  swarm-rust protection receipt-report ACTION RECEIPT SINK PREVIOUS_STATE --json (schema v2; exact input hashes; no authenticity or effect established)\n"
        } else {
            b"  swarm-rust protection canonicalize --input FICHIER|- --json (donnees hors ligne ; aucun droit)\n  swarm-rust protection inspect --input FICHIER|- --json (contenu exact ; aucun accord ni effet)\n  swarm-rust protection check-approval ACTION ACCORD --json (correspondance fournie ; aucun accord ni effet)\n  swarm-rust protection compare AVANT APRES --json (deux fichiers ; aucun droit)\n  swarm-rust protection resolve-payment REGISTRE DEMANDE --json (resolution de dette hors ligne ; aucun droit)\n  swarm-rust protection check-receipt ACTION RECU SERVICE ETAT_PRECEDENT --json (deux fichiers ; authenticite et effet non etablis)\n  swarm-rust protection receipt-report ACTION RECU SERVICE ETAT_PRECEDENT --json (schema v2 ; empreintes des entrees ; authenticite et effet non etablis)\n"
        })?;
        return Ok(0);
    }
    if a.pos[0] == "version" { return version::execute(a,out); }
    #[cfg(target_os = "linux")]
    if a.pos[0] == "prepare" { return preparation::execute(a,out); }
    #[cfg(target_os="linux")]
    if a.pos[0] == "_prepare_turn" {return preparation_worker::execute(a);}
    #[cfg(target_os="linux")]
    if a.pos[0] == "_prepare_cli_exec" {return preparation_worker::execute_cli_gate(a);}
    if a.pos[0]=="project-profile"{return project_profile::execute(a,out);}
    #[cfg(target_os = "linux")]
    if a.pos[0] == "web" {
        return native_web::execute(a, out);
    }
    #[cfg(target_os = "linux")]
    if matches!(
        a.pos[0].as_str(),
        "_agent_gate" | "_planning_gate" | "_validation_gate" | "_reviewer_gate"
    ) {
        if a.pos.len() != 2
            || !a.input.is_empty()
            || !a.output.is_empty()
            || a.server
            || !a.task.is_empty()
            || a.phase != "delivery"
        {
            return Err(CliError::Usage);
        }
        let result = if a.pos[0] == "_reviewer_gate" {
            swarm_engine::execute_review_gate(std::path::Path::new(&a.root), &a.pos[1])
        } else if a.pos[0] == "_validation_gate" {
            swarm_engine::execute_validation_gate(std::path::Path::new(&a.root), &a.pos[1])
        } else if a.pos[0] == "_planning_gate" {
            swarm_engine::execute_planning_gate(std::path::Path::new(&a.root), &a.pos[1])
        } else {
            swarm_engine::execute_agent_gate(std::path::Path::new(&a.root), &a.pos[1])
        };
        return match result {
            Ok(never) => match never {},
            Err(error) => Err(CliError::Root(Box::new(error))),
        };
    }
    #[cfg(target_os = "linux")]
    if a.pos[0] == "admin-budget" {return admin_budget::execute(a,out);}
    if a.pos[0] == "init" || a.pos[0] == "engine-settings" {
        return execute_root(a, out).map(|()| 0);
    }
    #[cfg(target_os = "linux")]
    if a.pos[0] == "quotas" { return quotas::execute(a,out); }
    #[cfg(target_os = "linux")]
    if a.pos[0] == "mission" { return mission::execute(a,out); }
    #[cfg(target_os = "linux")]
    if a.pos[0] == "costs" { return costs::execute(a,out); }
    #[cfg(target_os = "linux")]
    if a.pos[0] == "task-model" {
        return task_model::execute(a, out);
    }
    #[cfg(target_os = "linux")]
    if a.pos[0]=="visit" {return session_visit::execute(a,out);}
    #[cfg(target_os="linux")]
    if a.pos[0]=="runtime-health" {return runtime_health::execute(a,out);}
    #[cfg(target_os = "linux")]
    if a.pos[0] == "logs" {return log_query::execute(a,out);}
    #[cfg(target_os="linux")]
    if a.pos[0] == "activity" {return activity::execute(a,out);}
    #[cfg(target_os="linux")]
    if a.pos[0] == "exchange" {return exchange::execute(a,out);}
    #[cfg(target_os = "linux")]
    if a.pos[0] == "run-limits" {return if a.pos.get(1).is_some_and(|p|p=="provider-wait"){provider_wait::execute(a,out)}else if a.pos.get(1).is_some_and(|p|p=="web-requests"){web_request_policy::execute(a,out)}else if a.pos.get(1).is_some_and(|p|p=="performance"){graph_performance::execute(a,out)}else if a.pos.get(1).is_some_and(|p|p=="activity"){activity_policy::execute(a,out)}else if a.pos.get(1).is_some_and(|p|p=="logs"){log_read_policy::execute(a,out)}else{run_limits_admin::execute(a,out)};}
    #[cfg(target_os="linux")]
    if a.pos[0] == "providers" {
        return if a.pos.get(1).is_some_and(|s| s == "policies") {
            provider_policy::execute(a, out)
        } else if a.pos.get(1).is_some_and(|s|s=="connections") {
            provider_connections::execute(a,out)
        } else {
            provider_configuration::execute(a, out)
        };
    }
    if a.pos[0] == "pricing" {
        return execute_pricing(a, out);
    }
    if a.pos[0] == "agent" {
        return agent::execute(a, out);
    }
    if a.pos[0] == "planning-output" {
        return execute_planning_output(a, out);
    }
    if a.pos[0] == "budget" && a.pos.get(1).is_some_and(|v| matches!(v.as_str(), "show" | "preview")) { return budget_history::execute(a,out); }
    #[cfg(target_os="linux")]
    if a.pos[0]=="planning" && a.pos.get(1).is_some_and(|v|v=="history"){return planning_history::execute(a,out);}
    #[cfg(target_os="linux")]
    if a.pos[0]=="lifecycle" && a.pos.get(1).is_some_and(|v|v=="list"){return lifecycle_inventory::execute(a,out);}
    if matches!(a.pos[0].as_str(), "planning" | "budget") {
        return execute_control(a, out);
    }
    #[cfg(target_os = "linux")]
    if a.pos[0]=="task" && a.pos.get(1).is_some_and(|v|matches!(v.as_str(),"review"|"review-reconcile"|"review-retire-stale")) {return task_review::execute(a,out);}
    if a.pos[0] == "task" && a.pos.get(1).is_some_and(|v| matches!(v.as_str(), "review-preview" | "review-inputs" | "review-check" | "baseline-preview" | "publication-preview" | "publish" | "validation-preview" | "validation-candidate" | "validate" | "validation-reconcile" | "validation-retire")) {
        return task_validation::execute(a, out);
    }
    if matches!(
        a.pos[0].as_str(),
        "work" | "task" | "resume" | "ooda" | "checkpoint" | "gate"
    ) {
        return execute_work(a, out);
    }
    if matches!(
        a.pos[0].as_str(),
        "project-context" | "agent-workflow" | "planning-report"
    ) {
        let report = a.pos[0] == "planning-report";
        if a.pos.len() != if report { 1 } else { 2 }
            || (report && a.input.is_empty())
            || (!report && !a.input.is_empty())
            || a.server
            || !a.output.is_empty()
            || !a.task.is_empty()
            || a.phase != "delivery"
        {
            return Err(CliError::Usage);
        }
        let report_input = if report {
            Some(
                if a.input == "-" {
                    swarm_engine::read_command_document(&mut io::stdin().lock())
                } else {
                    swarm_engine::read_command_file(std::path::Path::new(&a.input))
                }
                .map_err(CliError::CommandInput)?,
            )
        } else {
            None
        };
        let store = swarm_engine::RootStore::open(std::path::Path::new(&a.root), false)
            .map_err(|e| CliError::Root(Box::new(e)))?;
        let result = if let Some(raw) = report_input {
            swarm_engine::decode_planning_report_event(&raw)
                .map_err(|e| {
                    CliError::Root(Box::new(swarm_engine::RootStoreError::PlanningReport(e)))
                })
                .and_then(|event| {
                    store
                        .planning_report(&event)
                        .map_err(|e| CliError::Root(Box::new(e)))
                })
                .and_then(|observed| json_bytes(&observed).map_err(CliError::Output))
        } else if a.pos[0] == "agent-workflow" {
            store
                .agent_workflow(&a.pos[1])
                .map_err(|e| CliError::Root(Box::new(e)))
                .and_then(|observed| json_bytes(&observed).map_err(CliError::Output))
        } else {
            store
                .project_context(&a.pos[1])
                .map_err(|e| CliError::Root(Box::new(e)))
                .and_then(|observed| json_bytes(&observed).map_err(CliError::Output))
        };
        let closed = store.close();
        let observed = match (result, closed) {
            (Ok(v), Ok(())) => v,
            (Err(e), Ok(())) => return Err(e),
            (Ok(_), Err(e)) => return Err(CliError::Root(Box::new(e))),
            (Err(primary), Err(cleanup)) => {
                return Err(CliError::RootCleanup {
                    primary: Box::new(primary),
                    cleanup: Box::new(cleanup),
                })
            }
        };
        write_output(out, &observed)?;
        return Ok(0);
    }
    #[cfg(target_os = "linux")]
    if a.pos[0] == "trust" && a.pos.get(1).is_some_and(|s| matches!(s.as_str(),"enrollment-call"|"enrollment-serve")) { return trust_enrollment::execute(a,out); }
    if a.pos[0] == "trust" { return trust::execute(a,out); }
    if a.pos[0] == "protection" { return protection::execute(a,out); }
    if a.pos[0] == "evaluate" {
        return execute_evaluation(a, out);
    }
    if a.pos[0] != "identity" {
        return Err(CliError::Unsupported);
    }
    #[cfg(target_os = "linux")]
    if matches!(a.pos.get(1).map(String::as_str),Some("initial-create"|"initial-reconcile")){return identity_initial::execute(a,out);}
    #[cfg(target_os = "linux")]
    if a.pos.get(1).map(String::as_str)==Some("checkpoint-check"){return identity_checkpoint::execute(a,out);}
    #[cfg(target_os = "linux")]
    if matches!(a.pos.get(1).map(String::as_str),Some("operator-check"|"service-export"|"service-export-reconcile")) {
        return identity_operator::execute(a,out);
    }
    #[cfg(target_os = "linux")]
    if a.pos.get(1).map(String::as_str)==Some("provisioning-check") {
        return identity_provisioning::execute(a,out);
    }
    #[cfg(target_os = "linux")]
    if a.pos.get(1).map(String::as_str)==Some("linux-prerequisites") {
        return identity_linux_prerequisites::execute(a,out);
    }
    #[cfg(target_os = "linux")]
    if a.pos.get(1).map(String::as_str)==Some("service-call") {
        return identity_service_call::execute(a,out);
    }
    #[cfg(target_os = "linux")]
    if matches!(a.pos.get(1).map(String::as_str),Some("service-install"|"service-resume"|"service-reconcile")) {
        return identity_service::execute(a,out);
    }
    #[cfg(target_os = "linux")]
    if matches!(a.pos.get(1).map(String::as_str),Some("personal-init"|"personal-enroll"|"personal-recover")) {
        return identity_personal::execute(a,out);
    }
    #[cfg(target_os = "linux")]
    if a.pos.get(1).map(String::as_str) == Some("control-check") {
        return identity_control::execute(a, out);
    }
    if a.pos.len() != 3
        || a.pos[1] != "bootstrap-check"
        || a.input.is_empty()
        || a.input == "-"
        || a.server
        || a.root != "."
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let document = bootstrap::load(&a.input, &a.pos[2]).map_err(CliError::Bootstrap)?;
    let result = BootstrapCheck {
        schema_version: 1,
        bootstrap_digest: a.pos[2].clone(),
        bootstrap: document,
        storage_verified: true,
        human_authentication_verified: false,
        confinement_verified: false,
        service_started: false,
    };
    let bytes = json_bytes(&result).map_err(CliError::Output)?;
    write_output(out, &bytes).map(|()| 0)
}
/// Execute project pricing list/save/estimate through the owned RootStore.
/// Strict bounded intents and representation validation precede state changes.
/// Serialize and close before checked write/flush; later failures distinguish
/// preserved configuration. A simulation changes no budget or reported usage;
/// this local CLI provides no HTTP/Admin authentication or provider execution.
fn execute_pricing(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 2
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let operation = a.pos[1].as_str();
    if !matches!(operation, "list" | "save" | "estimate") {
        return Err(CliError::Unsupported);
    }
    let raw = if operation == "list" {
        if !a.input.is_empty() {
            return Err(CliError::Usage);
        }
        None
    } else {
        if a.input.is_empty() {
            return Err(CliError::Usage);
        }
        Some(
            if a.input == "-" {
                swarm_engine::read_command_document(&mut io::stdin().lock())
            } else {
                swarm_engine::read_command_file(std::path::Path::new(&a.input))
            }
            .map_err(CliError::CommandInput)?,
        )
    };
    let pricing_error = |e| CliError::Root(Box::new(swarm_engine::RootStoreError::Pricing(e)));
    let change = if operation == "save" {
        Some(
            swarm_engine::decode_rate_change(raw.as_deref().ok_or(CliError::Usage)?)
                .map_err(pricing_error)?,
        )
    } else {
        None
    };
    let quote = if operation == "estimate" {
        Some(
            swarm_engine::decode_rate_quote(raw.as_deref().ok_or(CliError::Usage)?)
                .map_err(pricing_error)?,
        )
    } else {
        None
    };
    if let Some(change) = change.as_ref() {
        swarm_engine::validate_rate_change(change).map_err(pricing_error)?;
    }
    let root_error = |e| CliError::Root(Box::new(e));
    let store = swarm_engine::RootStore::open(std::path::Path::new(&a.root), operation == "save")
        .map_err(root_error)?;
    let mut saved = false;
    let result = (|| match operation {
        "list" => {
            json_bytes(&store.rate_catalogue().map_err(root_error)?).map_err(CliError::Output)
        }
        "save" => {
            let rate = store
                .save_rate(
                    change.as_ref().ok_or(CliError::Usage)?,
                    &mut swarm_engine::SystemEnvironment,
                )
                .map_err(root_error)?;
            saved = true;
            json_bytes(&rate).map_err(CliError::Output)
        }
        "estimate" => {
            let q = store
                .rate_catalogue()
                .map_err(root_error)?
                .quote(quote.as_ref().ok_or(CliError::Usage)?)
                .map_err(pricing_error)?;
            json_bytes(&q).map_err(CliError::Output)
        }
        _ => Err(CliError::Unsupported),
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(e)) => Err(root_error(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if saved {
            CliError::ConfigurationStatePreserved {
                primary: Box::new(primary),
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
/// Check retained JSONL through the actual saved-policy consumer. Regular file/
/// stdin framing uses the existing command transport limit; no provider starts.
/// Encode and close Root before output, preserving primary and cleanup failures.
fn execute_planning_output(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos != ["planning-output", "check"]
        || a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let raw = if a.input == "-" {
        swarm_engine::read_command_document(&mut io::stdin().lock())
    } else {
        swarm_engine::read_command_file(std::path::Path::new(&a.input))
    }
    .map_err(CliError::CommandInput)?;
    let root_error = |e| CliError::Root(Box::new(e));
    let store =
        swarm_engine::RootStore::open(std::path::Path::new(&a.root), false).map_err(root_error)?;
    let result = store
        .check_planning_output(&raw)
        .map_err(root_error)
        .and_then(|v| json_bytes(&v).map_err(CliError::Output));
    match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root_error(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
/// Consume bounded native preview input after control-argument validation.
/// Encode/close before output and preserve primary plus close failures. Preview
/// reads current Root data; run uses the registered foreground private gate.
/// Step claims FIFO with saved policy and runs once; idle is not completion.
/// Reconcile uses receipts without inference; late errors retain known Work.
/// These routes grant no authenticated identity or business acceptance.
#[cfg(target_os = "linux")]
fn execute_planning_call_preview(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let action = a.pos[1].as_str();
    let raw = if action == "reconcile" || action == "step" {
        if !a.input.is_empty() {
            return Err(CliError::Usage);
        }
        Vec::new()
    } else {
        if a.input.is_empty() {
            return Err(CliError::Usage);
        }
        if a.input == "-" {
            swarm_engine::read_command_document(&mut io::stdin().lock())
        } else {
            swarm_engine::read_command_file(std::path::Path::new(&a.input))
        }
        .map_err(CliError::CommandInput)?
    };
    let root_error = |e| CliError::Root(Box::new(e));
    let mut store =
        swarm_engine::RootStore::open(std::path::Path::new(&a.root), action != "call-preview")
            .map_err(root_error)?;
    let mut committed = None;
    let result = (|| {
        if action == "call-preview" {
            return store
                .preview_planning_call(&a.pos[2], &raw)
                .map_err(root_error)
                .and_then(|v| json_bytes(&v).map_err(CliError::Output));
        }
        if action == "step" {
            let value = store.step_planning(&a.pos[2]).map_err(root_error)?;
            if let Some(run) = &value.run {
                committed = Some((run.work.id.clone(), run.work.revision));
            }
            return json_bytes(&value).map_err(CliError::Output);
        }
        let value = if action == "run" {
            store.run_planning(&a.pos[2], &raw)
        } else {
            store.reconcile_planning(&a.pos[2])
        }
        .map_err(root_error)?;
        committed = Some((value.work.id.clone(), value.work.revision));
        json_bytes(&value).map_err(CliError::Output)
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root_error(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if let Some((work, revision)) = committed.or_else(|| primary.recorded_work()) {
            CliError::PlanningStatePreserved {
                primary: Box::new(primary),
                work,
                revision,
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
/// Native preview is explicitly unavailable on unported platforms; no empty pass.
#[cfg(not(target_os = "linux"))]
fn execute_planning_call_preview(_: &Arguments, _: &mut dyn Write) -> Result<u8, CliError> {
    Err(CliError::Unsupported)
}
/// Execute supported planning controls, including unused-instruction rejection,
/// or budget apply as Work JSON. Operator rejection retains event/revision replay
/// and active-task guards; response/close/flush failure preserves known publication.
/// Enable records provider/reviewer metadata without a process. Repository flows
/// remain unported; native verdict/retry/protocol decisions use Root proof. Bounded input goes to
/// RootStore's atomic policy/event/revision/replay contract; submitted actor/time
/// never establish authority. Encode and close before checked output/flush.
/// Committed state survives output/close failure. Run/step use their own process
/// controller; control data implies no budget reset or authenticated actor.
/// Direct judge/renew advance root cycle state without a model, claim or input
/// acknowledgement; lease-renew only extends an existing leased activation.
fn execute_control(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let action = a.pos[1].as_str();
    if a.pos[0] == "planning" && matches!(action, "call-preview" | "run" | "reconcile" | "step") {
        return execute_planning_call_preview(a, out);
    }
    let kind = if a.pos[0] == "budget" {
        if action != "apply" {
            return Err(CliError::Unsupported);
        }
        Some("budget.configure")
    } else {
        match action {
            "show" => None,
            "enable" => Some("planning.enable"),
            "claim" => Some("planning.claim"),
            "lease-renew" => Some("planning.lease-renew"),
            "decide" => Some("planning.decide"),
            "judge" => Some("planning.judge"),
            "renew" => Some("planning.renew"),
            "pause" => Some("planning.pause"),
            "resume" => Some("planning.resume"),
            "reject-instructions" => Some("planning.reject-instructions"),
            _ => {
                return Err(CliError::PlanningUnsupported {
                    english: a.language == "en",
                })
            }
        }
    };
    let input = if kind.is_some() {
        if a.input.is_empty() {
            return Err(CliError::Usage);
        }
        Some(
            if a.input == "-" {
                swarm_engine::read_command_document(&mut io::stdin().lock())
            } else {
                swarm_engine::read_command_file(std::path::Path::new(&a.input))
            }
            .map_err(CliError::CommandInput)?,
        )
    } else {
        None
    };
    let root_error = |e| CliError::Root(Box::new(e));
    let mut store = swarm_engine::RootStore::open(std::path::Path::new(&a.root), kind.is_some())
        .map_err(root_error)?;
    let mut committed = None;
    let result = (|| {
        let work = if let (Some(kind), Some(raw)) = (kind, input.as_deref()) {
            let outcome = store
                .execute_external(&a.pos[2], kind, raw, &mut swarm_engine::SystemEnvironment)
                .map_err(root_error)?;
            let work = outcome.result.work;
            committed = Some((work.id.clone(), work.revision));
            work
        } else {
            store.read_work(&a.pos[2]).map_err(root_error)?
        };
        json_bytes(&work).map_err(CliError::Output)
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cleanup)) => Err(root_error(cleanup)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if let Some((work, revision)) = committed.or_else(|| primary.recorded_work()) {
            CliError::PlanningStatePreserved {
                primary: Box::new(primary),
                work,
                revision,
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
/// Apply migrated Work commands or observe a recorded view through RootStore.
/// Native report submission verifies current completed agent/attempt and actual
/// report bytes; it creates no attempt, authenticates no author and accepts no task.
/// Route only migrated native Work operations. Validate syntax before opening a
/// root. Show stays read-only; resume publishes a view. Committed mutations are
/// distinguished from later rendering/close/output failure, retaining both causes.
/// Gate results return0 if allowed and1 if denied but recorded; errors return2
/// through the caller. None of these statuses authenticate or accept an agent.
fn execute_work(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.server
        || !a.output.is_empty()
        || a.phase != "delivery"
        || (!a.task.is_empty() && a.pos[0] != "gate")
    {
        return Err(CliError::Usage);
    }
    let (kind, id) = match a.pos[0].as_str() {
        "work" if a.pos.len() == 2 && a.pos[1] == "list" => {
            return execute_work_list(a, out).map(|()| 0)
        }
        "work" if a.pos.len() == 2 && a.pos[1] == "create" => ("work.create", ""),
        "work" if a.pos.len() == 3 => (
            match a.pos[1].as_str() {
                "show" => "work.show",
                "update" => "work.update",
                _ => return Err(CliError::Usage),
            },
            a.pos[2].as_str(),
        ),
        "task" if a.pos.len() == 3 => (
            match a.pos[1].as_str() {
                "add" => "task.add",
                "update" => "task.update",
                "submit" => "task.submit",
                _ => return Err(CliError::Usage),
            },
            a.pos[2].as_str(),
        ),
        "resume" if a.pos.len() == 1 => return execute_work_list(a, out).map(|()| 0),
        "resume" | "ooda" | "checkpoint" | "gate" if a.pos.len() == 2 => {
            (a.pos[0].as_str(), a.pos[1].as_str())
        }
        _ => return Err(CliError::Usage),
    };
    let mutate = !matches!(kind, "work.show" | "resume");
    if mutate == a.input.is_empty() {
        return Err(CliError::Usage);
    }
    let input = if mutate {
        Some(
            if a.input == "-" {
                swarm_engine::read_command_document(&mut io::stdin().lock())
            } else {
                swarm_engine::read_command_file(std::path::Path::new(&a.input))
            }
            .map_err(CliError::CommandInput)?,
        )
    } else {
        None
    };
    let gate_task = if kind == "gate" {
        let raw = input.as_deref().ok_or(CliError::Usage)?;
        let request = swarm_engine::decode_gate_request(raw).map_err(CliError::GateInput)?;
        if !a.task.is_empty() && a.task != request.task_id {
            return Err(CliError::GateTaskMismatch {
                english: !a.json && a.language == "en",
            });
        }
        Some(request.task_id)
    } else {
        None
    };
    let language = if !a.json && a.language == "en" {
        swarm_engine::WorkViewLanguage::English
    } else {
        swarm_engine::WorkViewLanguage::French
    };
    let root_error = |e| CliError::Root(Box::new(e));
    let mut store =
        swarm_engine::RootStore::open(std::path::Path::new(&a.root), kind != "work.show")
            .map_err(root_error)?;
    let mut committed = None;
    let mut exit_status = 0;
    let result = (|| {
        let (selected, revision) = if let Some(raw) = input {
            let mutation = store
                .execute_external(id, kind, &raw, &mut swarm_engine::SystemEnvironment)
                .map_err(root_error)?;
            let work = mutation.result.work;
            committed = Some((work.id.clone(), work.revision));
            if let Some(task) = &gate_task {
                let gate = work
                    .tasks
                    .iter()
                    .flatten()
                    .find(|t| &t.id == task)
                    .and_then(|t| t.gate.as_deref())
                    .ok_or_else(|| {
                        root_error(swarm_engine::RootStoreError::Refused("gate-result-missing"))
                    })?;
                if !gate.evaluation.allowed {
                    exit_status = 1;
                }
            }

            (work.id, Some(work.revision))
        } else {
            (id.to_owned(), None)
        };
        let view = if kind == "work.show" {
            store.inspect_work_view(&selected, language)
        } else {
            store.render_work_view(&selected, revision, language)
        }
        .map_err(root_error)?;
        if a.json {
            swarm_engine::encode_work_view(&view).map_err(CliError::View)
        } else {
            Ok(format!("{}\n", view.resume_markdown).into_bytes())
        }
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(e)) => Err(root_error(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| exit_status).map_err(|primary| {
        if let Some((work, revision)) = committed.or_else(|| primary.recorded_work()) {
            CliError::StatePreserved {
                primary: Box::new(primary),
                work,
                revision,
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
/// Reading, serialization and checked close finish before any success output;
/// any failure preserves its cause, and an output error remains a failed command.
/// No initialization, mutation, acceptance, provider or Go process is performed.
fn execute_work_list(a: &Arguments, out: &mut dyn Write) -> Result<(), CliError> {
    let root_error = |e| CliError::Root(Box::new(e));
    if !(a.pos.len() == 2 && a.pos[0] == "work" && a.pos[1] == "list"
        || a.pos.len() == 1 && a.pos[0] == "resume")
        || !a.input.is_empty()
        || a.server
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let store =
        swarm_engine::RootStore::open(std::path::Path::new(&a.root), false).map_err(root_error)?;
    let result = if a.json {
        store
            .list_works()
            .map_err(root_error)
            .and_then(|works| json_bytes(&works).map_err(CliError::Output))
    } else {
        let language = if a.language == "en" {
            swarm_engine::WorkViewLanguage::English
        } else {
            swarm_engine::WorkViewLanguage::French
        };
        store
            .inspect_work_list_text(language)
            .map(|s| s.into_bytes())
            .map_err(root_error)
    };
    let bytes = match (result, store.close()) {
        (Ok(bytes), Ok(())) => bytes,
        (Err(primary), Ok(())) => return Err(primary),
        (Ok(_), Err(error)) => return Err(root_error(error)),
        (Err(primary), Err(cleanup)) => {
            return Err(CliError::RootCleanup {
                primary: Box::new(primary),
                cleanup: Box::new(cleanup),
            })
        }
    };
    write_output(out, &bytes)
}
fn evaluation_bytes(value: &impl Serialize) -> Result<Vec<u8>, CliError> {
    let text =
        serde_json::to_string_pretty(value).map_err(|e| CliError::Output(io::Error::other(e)))?;
    let mut bytes = text
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
        .into_bytes();
    bytes.push(b'\n');
    Ok(bytes)
}
fn execute_evaluation(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 1
        || a.input.is_empty()
        || a.server
        || !a.output.is_empty()
        || !a.task.is_empty()
    {
        return Err(CliError::Usage);
    }
    let raw = if a.input == "-" {
        swarm_engine::read_command_document(&mut io::stdin().lock())
    } else {
        swarm_engine::read_command_file(std::path::Path::new(&a.input))
    }
    .map_err(CliError::CommandInput)?;
    let method =
        swarm_engine::gate_document_method(&raw).map_err(|e| CliError::Evaluation(Box::new(e)))?;
    let root = std::path::Path::new(&a.root);
    let (allowed, bytes) = if method == "3" {
        let store =
            swarm_engine::RootStore::open(root, false).map_err(|e| CliError::Root(Box::new(e)))?;
        let result = (|| {
            store
                .settings()
                .map_err(|e| CliError::Root(Box::new(e)))?
                .quality_policy_snapshot()
                .map_err(|e| CliError::Root(Box::new(swarm_engine::RootStoreError::Settings(e))))
        })();
        let closed = store.close();
        let profile = match (result, closed) {
            (Ok(v), Ok(())) => v,
            (Err(primary), Ok(())) => return Err(primary),
            (Ok(_), Err(e)) => return Err(CliError::Root(Box::new(e))),
            (Err(primary), Err(cleanup)) => {
                return Err(CliError::RootCleanup {
                    primary: Box::new(primary),
                    cleanup: Box::new(cleanup),
                })
            }
        };
        let evaluated = swarm_engine::evaluate_gate_with_policy(&raw, root, &a.phase, &profile)
            .map_err(|e| CliError::Evaluation(Box::new(e)))?;
        (evaluated.evaluation.allowed, evaluation_bytes(&evaluated)?)
    } else {
        let evaluated = swarm_engine::evaluate_gate_method2(&raw, root, &a.phase)
            .map_err(|e| CliError::Evaluation(Box::new(e)))?;
        (evaluated.allowed, evaluation_bytes(&evaluated)?)
    };
    write_output(out, &bytes)?;
    Ok(if allowed { 0 } else { 1 })
}
/// Dispatch local settings operations with bounded strict input, checked close
/// and output/flush. Model-check is read-only and reports saved restrictions at
/// one revision; it grants no provider/identity/launch authority. Primary and
/// cleanup failures remain joined. Successful init/apply/replay retains known
/// configuration through encoding, Root close, write and flush failure. A refusal
/// before publication is never relabeled; unknown Root publication keeps its
/// original typed marker. No retry, rollback, history reset or authority is added.
fn execute_root(a: &Arguments, out: &mut dyn Write) -> Result<(), CliError> {
    use swarm_engine::{EngineSettingsChange, RootStore, SystemEnvironment};
    let root_error = |e| CliError::Root(Box::new(e));
    if a.server || !a.output.is_empty() || !a.task.is_empty() || a.phase != "delivery" {
        return Err(CliError::Usage);
    }
    let initialize = a.pos.len() == 1 && a.pos[0] == "init";
    let operation = if initialize {
        "init"
    } else if a.pos.len() == 2 && a.pos[0] == "engine-settings" {
        a.pos[1].as_str()
    } else {
        return Err(CliError::Usage);
    };
    if !matches!(
        operation,
        "init" | "show" | "preview" | "apply" | "describe" | "quality-profile" | "model-check"
    ) {
        return Err(CliError::Usage);
    }
    if operation == "describe" {
        if !a.input.is_empty() {
            return Err(CliError::Usage);
        }
        let catalog = swarm_engine::EngineSettingsCatalog::installation()
            .map_err(|e| root_error(swarm_engine::RootStoreError::Settings(e)))?;
        return write_output(out, &json_bytes(&catalog).map_err(CliError::Output)?);
    }
    let mut model_check = None;
    let change = if matches!(operation, "preview" | "apply" | "model-check") {
        if a.input.is_empty() {
            return Err(CliError::Usage);
        }
        let raw = if a.input == "-" {
            swarm_engine::read_command_document(&mut io::stdin().lock())
        } else {
            swarm_engine::read_command_file(std::path::Path::new(&a.input))
        }
        .map_err(CliError::CommandInput)?;
        if operation == "model-check" {
            model_check = Some(
                serde_json::from_slice::<swarm_engine::ModelPolicyCheckRequest>(&raw)
                    .map_err(CliError::SettingsInput)?,
            );
            None
        } else {
            Some(
                serde_json::from_slice::<EngineSettingsChange>(&raw)
                    .map_err(CliError::SettingsInput)?,
            )
        }
    } else {
        if !a.input.is_empty() {
            return Err(CliError::Usage);
        }
        None
    };
    let mut environment = SystemEnvironment;
    let store = if initialize {
        RootStore::initialize(std::path::Path::new(&a.root), &mut environment)
    } else {
        RootStore::open(std::path::Path::new(&a.root), operation == "apply")
    }
    .map_err(root_error)?;
    // A successful initialization already observed saved settings. Apply below
    // also covers a replay; neither observation implies a new publication.
    let mut configuration_recorded = initialize;
    let result = (|| match operation {
        "init" => Ok(
            serde_json::json!({"schema_version":1,"root":std::fs::canonicalize(&a.root).map_err(CliError::Input)?,"state":".swarm/state.db"}),
        ),
        "show" => serde_json::to_value(store.settings().map_err(root_error)?)
            .map_err(CliError::SettingsInput),
        "model-check" => serde_json::to_value(
            store
                .check_model_policy(model_check.as_ref().ok_or(CliError::Usage)?)
                .map_err(root_error)?,
        )
        .map_err(CliError::SettingsInput),
        "quality-profile" => serde_json::to_value(
            store
                .settings()
                .map_err(root_error)?
                .quality_policy_snapshot()
                .map_err(|e| root_error(swarm_engine::RootStoreError::Settings(e)))?,
        )
        .map_err(CliError::SettingsInput),
        "preview" => serde_json::to_value(
            store
                .preview_settings(change.as_ref().ok_or(CliError::Usage)?)
                .map_err(root_error)?,
        )
        .map_err(CliError::SettingsInput),
        "apply" => {
            let (document, replayed) = store
                .apply_settings(change.as_ref().ok_or(CliError::Usage)?, &mut environment)
                .map_err(root_error)?;
            configuration_recorded = true;
            Ok(serde_json::json!({"document":document,"replayed":replayed}))
        }
        _ => Err(CliError::Usage),
    })();
    let result = match (result, store.close()) {
        (Ok(value), Ok(())) => json_bytes(&value)
            .map_err(CliError::Output)
            .and_then(|bytes| write_output(out, &bytes)),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(error)) => Err(root_error(error)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map_err(|primary| {
        if configuration_recorded {
            CliError::ConfigurationStatePreserved {
                primary: Box::new(primary),
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}

pub fn run_detailed(
    args: &[String],
    environment_language: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> RunOutcome {
    let parsed = parse(args, environment_language);
    let json_mode = parsed.as_ref().is_ok_and(|a| a.json);
    let english = parsed.as_ref().is_ok_and(|a| a.language == "en");
    let result = match parsed.as_ref() {
        Ok(a) => execute(a, out),
        Err(_) => Ok(0),
    };
    let code = result.as_ref().copied().unwrap_or(0);
    let error = match (parsed.as_ref(), result) {
        (Err(_), _) => parsed.err(),
        (_, Err(e)) => Some(e),
        _ => None,
    };
    let Some(error) = error else {
        return RunOutcome {
            code,
            error: None,
            diagnostic_error: None,
        };
    };
    report(error, json_mode, english, err)
}
/// Project only classified codes, operation names and numeric OS causes. Raw
/// error Display/SQL/paths/context never enter this diagnostic. Compound cleanup
/// branches use the shared native classifier on Linux plus CLI/web wrappers
/// and the private watch wrapper's closed phase codes;
/// listed families remain scoped, not a proof about arbitrary Error types.
/// Typed-object dedup prevents
/// cycles without dropping a distinct source sharing a transparent wrapper's
/// data address. Diagnostic codes grant no retry, rollback or acceptance authority.
fn safe_causes(error: &(dyn std::error::Error + 'static)) -> Vec<String> {
    let mut pending = vec![error];
    let mut seen: Vec<*const dyn std::error::Error> = Vec::new();
    let mut out = Vec::new();
    while let Some(e) = pending.pop() {
        let pointer = e as *const dyn std::error::Error;
        // Distinct typed sources can occupy a transparent wrapper's address.
        // Include trait metadata while detecting repeated error objects.
        if seen.iter().any(|prior| std::ptr::eq(*prior, pointer)) {
            continue;
        }
        seen.push(pointer);
        #[cfg(target_os="linux")]
        if let Some(api)=e.downcast_ref::<swarm_api_client::Error>(){out.push(api.code().into());}
        #[cfg(target_os = "linux")]
        {
            pending.extend(native_web::branches(e));
            out.extend(mission_watch::safe_labels(e));
            if let Some(code) = native_web::family(e) {
                out.push(code.into());
            }
        }
        if let Some(cli_error @ CliError::RootCleanup { primary, cleanup }) =
            e.downcast_ref::<CliError>()
        {
            out.push(cli_error.code().into());
            pending.push(cleanup.as_ref());
            pending.push(primary.as_ref());
            continue;
        }
        if let Some(cli_error @ CliError::OutputFile { primary, close, .. }) = e.downcast_ref::<CliError>() {
            out.push(cli_error.code().into());
            if let Some(cause) = primary { pending.push(cause.as_ref()); }
            if let Some(cause) = close { pending.push(cause.as_ref()); }
            continue;
        }
        #[cfg(target_os = "linux")]
        {
            let (labels, children) = identity_control::error_links(e);
            out.extend(labels);
            pending.extend(children);
            let (labels, children) = identity_personal::error_links(e);
            out.extend(labels);
            pending.extend(children);
            let (labels, children) = identity_service::error_links(e);
            out.extend(labels);
            pending.extend(children);
            let (labels, children) = identity_service_call::error_links(e);
            out.extend(labels);
            pending.extend(children);
            let (labels, children) = identity_provisioning::error_links(e);
            out.extend(labels);
            pending.extend(children);
            let (labels, children) = identity_initial::error_links(e);
            out.extend(labels);
            pending.extend(children);
            let (labels, children) = identity_checkpoint::error_links(e);
            out.extend(labels);
            pending.extend(children);
            let (labels, children) = identity_operator::error_links(e);
            out.extend(labels);
            pending.extend(children);
            let links = swarm_engine::native_error_links(e);
            out.extend(links.labels);
            pending.extend(links.children.into_iter().rev().map(|(_, error)| error));
        }
        #[cfg(not(target_os = "linux"))]
        {
            if let Some(e) = e.downcast_ref::<swarm_engine::AgentGuardError>() {
                out.push(e.code().into());
            }
            if let Some(e) = e.downcast_ref::<swarm_engine::RootStoreError>() {
                out.push(e.code().into());
                if let swarm_engine::RootStoreError::Cleanup { primary, cleanup } = e {
                    pending.push(cleanup.as_ref());
                    if let Some(primary) = primary {
                        pending.push(primary.as_ref());
                    }
                    continue;
                }
            }
            if let Some(e) = e.downcast_ref::<swarm_engine::AgentLaunchError>() {
                out.push(e.code().into());
                if let swarm_engine::AgentLaunchError::Cleanup { primary, cleanup } = e {
                    pending.push(cleanup.as_ref());
                    pending.push(primary.as_ref());
                    continue;
                }
            }
            if let Some(e) = e.downcast_ref::<swarm_engine::LocalStorageError>() {
                out.push(format!("storage-operation:{:?}", e.operation));
            }
            if let Some(e) = e.downcast_ref::<swarm_engine::ValidationRunError>() {
                out.push(format!("process-operation:{}", e.operation()));
                for c in e.cleanup() {
                    out.push(format!("process-cleanup:{}", c.operation));
                }
            }
            if let Some(e) = e.downcast_ref::<io::Error>() {
                out.push(format!("io-kind:{:?}", e.kind()));
                if let Some(n) = e.raw_os_error() {
                    out.push(format!("os-errno:{n}"));
                }
                if let Some(inner) = e.get_ref() {
                    pending.push(inner);
                    continue;
                }
            }
            if let Some(source) = e.source() {
                pending.push(source);
            }
        }
    }
    out
}
/// Write a plain or JSON failure and explicitly flush the diagnostic stream.
/// Parser/language failures remain plain; JSON projects only classified causes.
/// Returns the protocol failure exit (preparation conflicts3, disabled4, unknown
/// method5; other ordinary failures2), with the original and any encode/write/flush
/// error, so a failed diagnostic cannot hide a recorded state or authorize retry.
/// Render a nonzero CLI failure with safe typed causes, known Work and typed
/// configuration/auxiliary-preservation metadata on every platform, independently of web
/// sessions. A true configuration_may_be_recorded requires reread before recovery;
/// its absence never proves rollback. Unknown effects are
/// omitted rather than asserted rolled back; output failures retain the original
/// diagnostic and secondary writer error in RunOutcome. This writes/flushes only
/// the supplied error sink, performs no Root IO/retry and grants no acceptance.
fn report(error: CliError, json_mode: bool, english: bool, err: &mut dyn Write) -> RunOutcome {
    // A Root apply may return Err after known publication, before the command's
    // local applied flag. Normalize that typed marker even through a second close.
    let error = if !matches!(&error, CliError::ConfigurationStatePreserved { .. })
        && error.configuration_may_be_recorded()
    {
        CliError::ConfigurationStatePreserved {
            primary: Box::new(error),
            english,
        }
    } else {
        error
    };
    // Retain an actual Root/transport receipt in both JSON and human diagnostics.
    // This adds no effect based on an error string or a missing witness.
    let effects=error.auxiliary_effects();
    let error=if !effects.is_empty()&&!matches!(&error,CliError::AuxiliaryState{..}) {CliError::AuxiliaryState{primary:Box::new(error),effects,english}}else{error};
    // Language parsing errors precede the command parser, matching Go Main. Missing
    // option values are also plain diagnostics, even when --json appeared earlier.
    let plain = matches!(
        error,
        CliError::Language(_) | CliError::MissingValue { .. } | CliError::ArgumentEncoding
    );
    let as_json = !plain && json_mode;
    let message = if english && matches!(&error, CliError::Unsupported) {
        "Command is not yet ported in this Rust development binary.".into()
    } else { error.message() };
    let bytes = if as_json {
        let mut document = serde_json::json!({"schema_version":1,"error":message,"failure":Failure{code:error.code(),message:&message,retryable:false,causes:safe_causes(&error)}});
        if let Some(publication)=error.trust_publication(){document["trust_publication"]=serde_json::json!(publication);document["trust_state_may_be_recorded"]=serde_json::json!(true);}
        let auxiliary=error.auxiliary_effects();
        if !auxiliary.is_empty(){document["auxiliary_effects"]=serde_json::json!(auxiliary);}
        if let Some((id, revision)) = error.recorded_work() {
            document["recorded_work"] = serde_json::json!({"id":id,"revision":revision});
        }
        if let Some((id,revision))=error.recorded_preparation(){document["recorded_preparation"]=serde_json::json!({"id":id,"revision":revision});}
        #[cfg(target_os = "linux")]
        native_web::details(&error, &mut document);
        // Typed configuration state applies to ordinary CLI commands as well as
        // web sessions. Missing metadata is unknown, never evidence of rollback.
        if error.configuration_may_be_recorded() {
            document["configuration_may_be_recorded"] = serde_json::json!(true);
        }
        json_bytes(&document)
    } else {
        Ok(if plain {
            format!("{message}\n")
        } else {
            format!(
                "{} {message}\n",
                if english { "Error:" } else { "Erreur :" }
            )
        }
        .into_bytes())
    };
    let diagnostic_error = match bytes {
        Ok(bytes) => err.write_all(&bytes).and_then(|()| err.flush()).err(),
        Err(e) => Some(e),
    };
    let code=if error.typed_causes().iter().any(|e|e.is::<swarm_engine::TrustError>()){match error.code(){"authorization_required"=>4,"revision_conflict"|"event_conflict"|"idempotency_conflict"=>3,_=>2}}else{match error.code(){"preparation-event-conflict"|"preparation-revision-conflict"|"preparation-work-revision-conflict"=>3,"preparation-write-disabled"=>4,"preparation-method-unavailable"=>5,_=>2}};
    RunOutcome {
        code,
        error: Some(error),
        diagnostic_error,
    }
}
pub fn run(
    args: &[String],
    environment_language: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    run_detailed(args, environment_language, out, err).code
}

impl fmt::Debug for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CliError")
            .field("code", &self.code())
            .field("message", &self.message())
            .finish()
    }
}
impl fmt::Debug for RunOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunOutcome")
            .field("code", &self.code)
            .field("error", &self.error)
            .field(
                "diagnostic_error",
                &self
                    .diagnostic_error
                    .as_ref()
                    .map(|e| (e.kind(), e.raw_os_error())),
            )
            .finish()
    }
}

/// Preserve a refusal and diagnostic-write failure for non-UTF8 OS arguments.
/// This development command accepts UTF-8 paths, as its bootstrap Text contract.
pub fn run_os_detailed(
    args: &[std::ffi::OsString],
    environment_language: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> RunOutcome {
    let converted = args
        .iter()
        .map(|a| a.to_str().map(String::from))
        .collect::<Option<Vec<_>>>();
    match converted {
        Some(args) => run_detailed(&args, environment_language, out, err),
        None => report(CliError::ArgumentEncoding, false, false, err),
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    /// Known typed commit wrappers must reach structured output independently
    /// of a web error. A bare output failure supplies no revision; cleanup nesting
    /// preserves known state/cause and never publishes the private IO message.
    #[test]
    fn recorded_cli_control_metadata_reaches_json_without_a_web_error() {
        let private=||CliError::Output(io::Error::new(io::ErrorKind::BrokenPipe,"PRIVATE_IO_CONTENT"));
        let mut raw=Vec::new();let result=report(private(),true,true,&mut raw);assert_ne!(result.code,0);
        let unknown:serde_json::Value=serde_json::from_slice(&raw).unwrap();assert!(unknown.get("recorded_work").is_none());
        for index in 0..3 {
            let known=match index {
                0=>CliError::PlanningStatePreserved{primary:Box::new(private()),work:"known-work".into(),revision:7,english:true},
                1=>CliError::StatePreserved{primary:Box::new(private()),work:"known-work".into(),revision:7,english:true},
                _=>CliError::TaskValidationStatePreserved{primary:Box::new(private()),work:"known-work".into(),revision:7,english:true},
            };
            assert_eq!(known.recorded_work(),Some(("known-work".into(),7)));
            let nested=CliError::RootCleanup{primary:Box::new(known),cleanup:Box::new(swarm_engine::RootStoreError::Refused("owned-close-refusal"))};
            let mut raw=Vec::new();let result=report(nested,true,true,&mut raw);assert_ne!(result.code,0);let value:serde_json::Value=serde_json::from_slice(&raw).unwrap();
            assert_eq!(value["recorded_work"],serde_json::json!({"id":"known-work","revision":7}));assert!(value["failure"]["causes"].as_array().unwrap().iter().any(|v|v=="io-kind:BrokenPipe"));assert!(!String::from_utf8(raw).unwrap().contains("PRIVATE_IO_CONTENT"));
        }
    }

    /// A typed unported command has the same safe code as unported planning,
    /// in either language, with no Root access or error-text classification.
    #[test]
    fn unsupported_commands_keep_typed_code_and_both_languages() {
        assert_eq!(CliError::Unsupported.code(), "command_not_ported");
        for language in ["fr", "en"] {
            let mut out=Vec::new();let mut err=Vec::new();
            let result=run_detailed(&["unknown-fixture-command".into(),"--json".into(),"--lang".into(),language.into()],language,&mut out,&mut err);
            assert_ne!(result.code,0);assert!(out.is_empty());
            let value:serde_json::Value=serde_json::from_slice(&err).unwrap();assert_eq!(value["failure"]["code"],"command_not_ported");
            assert!(value["error"].as_str().unwrap().contains(if language=="en" {"not yet ported"}else{"non encore portée"}));
        }
    }

    /// A known launch-class control precedes a typed guard cause nested in IO.
    /// Codes must remain visible without serializing private cause Display text.
    #[test]
    fn nested_guard_causes_are_classified_without_private_text() {
        let control = swarm_engine::AgentLaunchError::Refused("fixture-launch");
        assert!(safe_causes(&control).contains(&"fixture-launch".into()));
        let guard = swarm_engine::decode_agent_guard_trace(br#"{"schema_version":"PRIVATE"}"#)
            .err()
            .expect("invalid public trace must refuse");
        let expected = guard.code().to_owned();
        let guarded = io::Error::other(guard);
        let codes = safe_causes(&guarded);
        assert!(codes.contains(&expected));
        assert!(!codes.iter().any(|v| v.contains("PRIVATE")));
    }
    /// Both independently observable root refusals must survive the CLI's own
    /// compound cleanup wrapper, not only the lower-level RootStore wrapper.
    #[test]
    fn cli_compound_cleanup_retains_primary_and_secondary_causes() {
        let primary = CliError::Root(Box::new(swarm_engine::RootStoreError::Refused(
            "fixture-primary",
        )));
        let cleanup = swarm_engine::RootStoreError::Refused("fixture-cleanup");
        assert!(safe_causes(&primary).contains(&"fixture-primary".into()));
        assert!(safe_causes(&cleanup).contains(&"fixture-cleanup".into()));
        let compound = CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        };
        let codes = safe_causes(&compound);
        assert!(codes.contains(&"fixture-primary".into()));
        assert!(codes.contains(&"fixture-cleanup".into()));
    }
    /// Transparent wrappers and their distinct typed source share a data
    /// address. A real self-cycle remains bounded; type-distinct sources at that
    /// address must still be classified, without raw Display or Debug output.
    #[test]
    fn typed_same_address_source_is_not_mistaken_for_a_cycle() {
        #[derive(Debug)]
        struct Cycle;
        impl fmt::Display for Cycle {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("PRIVATE cycle")
            }
        }
        impl std::error::Error for Cycle {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(self)
            }
        }
        assert!(safe_causes(&Cycle).is_empty());
        #[repr(transparent)]
        #[derive(Debug)]
        struct Wrapped(io::Error);
        impl fmt::Display for Wrapped {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("PRIVATE wrapper")
            }
        }
        impl std::error::Error for Wrapped {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        let value = Wrapped(io::Error::other("PRIVATE payload"));
        assert!(safe_causes(&value.0).contains(&"io-kind:Other".into()));
        let codes = safe_causes(&value);
        assert!(codes.contains(&"io-kind:Other".into()));
        assert!(!codes.iter().any(|v| v.contains("PRIVATE")));
    }
}

#[cfg(test)]
mod recorded_work_tests {
    use super::*;
    /// Known CLI state must survive its own transparent and secondary wrappers,
    /// just as Root publication does. No actual OS close/output failure injected.
    #[test]
    fn cli_configuration_marker_survives_own_io_and_cleanup_wrappers() {
        let witness = CliError::Root(Box::new(swarm_engine::RootStoreError::Published {
            primary: Box::new(swarm_engine::RootStoreError::Refused("fixture-published")),
        }));
        assert!(witness.configuration_may_be_recorded());
        let marker = || CliError::ConfigurationStatePreserved {
            primary: Box::new(CliError::Output(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture output",
            ))),
            english: true,
        };
        let direct = marker();
        assert!(direct.configuration_may_be_recorded());
        let wrapped = CliError::Output(io::Error::other(marker()));
        assert!(wrapped.configuration_may_be_recorded());
        let compound = CliError::RootCleanup {
            primary: Box::new(CliError::Output(io::Error::other(marker()))),
            cleanup: Box::new(swarm_engine::RootStoreError::Refused("fixture-close")),
        };
        assert!(compound.configuration_may_be_recorded());
        let mut out = vec![];
        let r = report(compound, true, false, &mut out);
        assert_eq!(r.code, 2);
        let j: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert!(j["failure"]["message"]
            .as_str()
            .unwrap()
            .starts_with("Configuration enregistrée"));
        assert!(!CliError::Output(io::Error::other("fixture before apply"))
            .configuration_may_be_recorded());
    }

    /// Typed aggregation only: a Root-returned late failure must reach generic,
    /// planning/task-model CLI consumers even before their local commit flag.
    /// Compound close causes remain visible without claiming an OS close fault.
    #[test]
    fn cli_retains_root_recorded_work_through_compound_cleanup() {
        let recorded = swarm_engine::RootStoreError::WorkRecorded {
            primary: Box::new(swarm_engine::RootStoreError::Refused("fixture-release")),
            work: "w-fixture".into(),
            revision: 3,
        };
        let error = CliError::RootCleanup {
            primary: Box::new(CliError::Root(Box::new(recorded))),
            cleanup: Box::new(swarm_engine::RootStoreError::Refused("fixture-extra-close")),
        };
        assert_eq!(error.recorded_work(), Some(("w-fixture".into(), 3)));
        let causes = safe_causes(&error);
        assert!(causes.contains(&"work-state-preserved".into()));
        assert!(causes.contains(&"fixture-release".into()));
        assert!(causes.contains(&"fixture-extra-close".into()));
        let committed: Option<(String, i64)> = None;
        let (work, revision) = committed.or_else(|| error.recorded_work()).unwrap();
        let error = CliError::StatePreserved {
            primary: Box::new(error),
            work,
            revision,
            english: true,
        };
        assert!(error.to_string().contains("w-fixture"));
        assert!(error.to_string().contains('3'));
        let refusal = CliError::Root(Box::new(swarm_engine::RootStoreError::Refused(
            "fixture-before-commit",
        )));
        assert!(refusal.recorded_work().is_none());
    }

    /// Typed settings/publication metadata survives two cleanup branches and is
    /// localized by the actual public diagnostic encoder, without string matching.
    #[test]
    fn cli_settings_publication_survives_late_close_in_json_and_text() {
        for json in [false, true] {
            for english in [false, true] {
                let error = CliError::RootCleanup {
                    primary: Box::new(CliError::Root(Box::new(
                        swarm_engine::RootStoreError::Published {
                            primary: Box::new(swarm_engine::RootStoreError::Refused(
                                "fixture-settings-release",
                            )),
                        },
                    ))),
                    cleanup: Box::new(swarm_engine::RootStoreError::Refused("fixture-extra-close")),
                };
                assert!(error.configuration_may_be_recorded());
                let mut output = vec![];
                let outcome = report(error, json, english, &mut output);
                assert_eq!(outcome.code, 2);
                assert!(outcome.diagnostic_error.is_none());
                let text = String::from_utf8(output).unwrap();
                assert!(text.contains(if english {
                    "Configuration is recorded"
                } else {
                    "Configuration enregistrée"
                }));
                if json {
                    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
                    let causes = v["failure"]["causes"].as_array().unwrap();
                    assert!(causes.contains(&serde_json::json!("root-state-preserved")));
                    assert!(causes.contains(&serde_json::json!("fixture-settings-release")));
                    assert!(causes.contains(&serde_json::json!("fixture-extra-close")));
                }
            }
        }
        let before = CliError::Root(Box::new(swarm_engine::RootStoreError::Refused(
            "fixture-before-publication",
        )));
        assert!(!before.configuration_may_be_recorded());
    }

    /// Standard IO source() skips its custom error itself. Preserve typed Work/
    /// config markers via get_ref(), not only the marker's own primary source.
    /// These are error-aggregation fixtures, not actual OS failures/mutations.
    #[test]
    fn cli_preserved_state_wrapped_in_io_keeps_work_and_config_markers() {
        let direct = CliError::Root(Box::new(swarm_engine::RootStoreError::WorkRecorded {
            primary: Box::new(swarm_engine::RootStoreError::Refused("fixture-release")),
            work: "w-io".into(),
            revision: 7,
        }));
        assert_eq!(direct.recorded_work(), Some(("w-io".into(), 7)));
        let error = CliError::Input(io::Error::other(
            swarm_engine::RootStoreError::WorkRecorded {
                primary: Box::new(swarm_engine::RootStoreError::Refused("fixture-release")),
                work: "w-io".into(),
                revision: 7,
            },
        ));
        assert!(safe_causes(&error).contains(&"work-state-preserved".into()));
        assert_eq!(
            error.recorded_work(),
            Some(("w-io".into(), 7)),
            "IO skipped the typed preserved-state marker"
        );
        let error = CliError::Input(io::Error::other(swarm_engine::RootStoreError::Published {
            primary: Box::new(swarm_engine::RootStoreError::Refused("fixture-visible")),
        }));
        assert!(error.configuration_may_be_recorded());
        let mut stderr = vec![];
        let outcome = report(error, true, true, &mut stderr);
        assert_eq!(outcome.code, 2);
        assert!(outcome.diagnostic_error.is_none());
        let v: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert!(v["error"]
            .as_str()
            .unwrap()
            .contains("Configuration is recorded"));
    }
    /// Native commit primary/cleanup errors both cross the actual JSON failure
    /// formatter. Private IO text must stay hidden while safe cause families,
    /// original failure status and diagnostic flush remain observable.
    #[test]
    fn cli_native_commit_projection_retains_both_causes_without_private_text() {
        let cause = |code, kind, text| swarm_engine::AgentLaunchError::Cause {
            code,
            cause: Box::new(io::Error::new(kind, text)),
        };
        let native = swarm_engine::AgentLaunchError::Commit {
            cause: Box::new(cause(
                "fixture-primary",
                io::ErrorKind::PermissionDenied,
                "PRIVATE primary path",
            )),
            rollback_confirmed: false,
            cleanup: Some(Box::new(cause(
                "fixture-cleanup",
                io::ErrorKind::BrokenPipe,
                "PRIVATE cleanup context",
            ))),
        };
        let error = CliError::Root(Box::new(swarm_engine::RootStoreError::Launch(Box::new(
            native,
        ))));
        let mut stderr = vec![];
        let outcome = report(error, true, true, &mut stderr);
        assert_eq!(outcome.code, 2);
        assert!(outcome.diagnostic_error.is_none());
        let document: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        let causes = document["failure"]["causes"].as_array().unwrap();
        for expected in [
            "fixture-primary",
            "fixture-cleanup",
            "io-kind:PermissionDenied",
            "io-kind:BrokenPipe",
        ] {
            assert!(
                causes.iter().any(|v| v.as_str() == Some(expected)),
                "A native cleanup cause was omitted"
            );
        }
        assert!(!String::from_utf8(stderr).unwrap().contains("PRIVATE"));
    }
}

#[cfg(target_os = "linux")]
mod preparation;

mod budget_history;

#[cfg(target_os="linux")]
mod project_profile;

#[cfg(test)]
mod configuration_report_metadata_tests {
    use super::*;
    /// Machine-readable preservation follows typed configuration wrappers even
    /// outside native-web errors; unrelated refusal is never reported as rollback.
    #[test]
    fn recorded_configuration_json_survives_non_web_output_and_cleanup_errors() {
        let published=||swarm_engine::RootStoreError::Published { primary:Box::new(swarm_engine::RootStoreError::Refused("fixture-late-configuration-error")) };
        let cases=vec![
            CliError::ConfigurationStatePreserved { primary:Box::new(CliError::Output(io::Error::new(io::ErrorKind::BrokenPipe,"owned fixture sink"))),english:true },
            CliError::Root(Box::new(published())),
            CliError::RootCleanup { primary:Box::new(CliError::Root(Box::new(published()))),cleanup:Box::new(swarm_engine::RootStoreError::Refused("fixture-close-error")) },
        ];
        for error in cases {
            let mut bytes=Vec::new();let outcome=report(error,true,true,&mut bytes);
            assert_eq!(outcome.code,2);assert!(outcome.diagnostic_error.is_none());
            let wire:serde_json::Value=serde_json::from_slice(&bytes).unwrap();
            assert_eq!(wire["configuration_may_be_recorded"],true);
        }
        let mut bytes=Vec::new();report(CliError::Usage,true,true,&mut bytes);
        let wire:serde_json::Value=serde_json::from_slice(&bytes).unwrap();
        assert!(wire.get("configuration_may_be_recorded").is_none());
    }
}

#[cfg(target_os="linux")]
mod preparation_worker;
