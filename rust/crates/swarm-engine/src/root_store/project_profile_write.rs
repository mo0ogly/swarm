//! Explicit project-profile configuration under retained project/datastore custody.
//! No source contents or native permission settings enter public metadata.
use super::{finished, LocalDirectory, ProjectProfileStatus, RootBinding, RootStore, RootStoreError as E};
use crate::{ContextPolicyLimits, ProjectContext};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

const ROLES: &[&str] = &["preparation", "planner", "subplanner", "worker", "reviewer"];
const CHOICES: &[(&str, &str, &str)] = &[
    ("claude-project", ".claude — consignes Claude du projet", ".claude/CLAUDE.md"),
    ("claude-root", "CLAUDE.md — consignes Claude", "CLAUDE.md"),
    ("agents-root", "AGENTS.md — consignes partagées", "AGENTS.md"),
    ("agents-directory", ".agents — consignes partagées", ".agents/AGENTS.md"),
    ("gemini-root", "GEMINI.md — consignes Gemini", "GEMINI.md"),
    ("gemini-directory", ".gemini — consignes Gemini", ".gemini/GEMINI.md"),
];

/// Versioned instruction references, not file contents or an execution grant.
#[derive(Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ProjectProfileDefinition {
    pub version: i64,
    pub name: String,
    pub instructions: Option<Vec<ProjectProfileInstruction>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source: String,
}
/// One explicitly selected project path and protocol roles. Source admission
/// remains the context reader's responsibility, including saved caps and secrets.
#[derive(Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ProjectProfileInstruction {
    pub path: String,
    pub roles: Option<Vec<String>>,
}
/// A suggested configuration and its current availability, never authority.
/// A safe reason is public; the original local cause stays out of serialization.
#[derive(Serialize)]
pub struct ProjectProfileChoice {
    pub id: String,
    pub title: String,
    pub available: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
    pub profile: ProjectProfileDefinition,
    #[serde(skip)]
    cause: Option<Box<E>>,
}
impl ProjectProfileChoice {
    /// Inspect the original unavailable-source cause locally without rendering
    /// it in a CLI/API response. None is not proof of authority or availability.
    pub fn cause(&self) -> Option<&E> { self.cause.as_deref() }
}
/// Current configuration digest and six explicit suggestions. An invalid active
/// JSON document keeps its digest for CAS repair and a visible safe diagnostic;
/// its name/source are not invented as observations of a valid active profile.
#[derive(Serialize)]
pub struct ProjectProfileCatalog {
    pub active_sha256: String,
    pub active_name: String,
    pub active_source: String,
    pub choices: Vec<ProjectProfileChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_error: Option<&'static str>,
    #[serde(skip)]
    cause: Option<Box<E>>,
}
impl ProjectProfileCatalog {
    /// Preserve the original malformed-active-document cause for local callers;
    /// public metadata exposes only active_error and the exact observed digest.
    pub fn active_cause(&self) -> Option<&E> { self.cause.as_deref() }
}
/// Readonly candidate validation. Role digests are snapshots, not permissions,
/// source-content approvals or a promise that a later apply will see those bytes.
#[derive(Serialize)]
pub struct ProjectProfilePreview {
    pub active_sha256: String,
    pub profile: ProjectProfileDefinition,
    pub roles: Vec<ProjectContext>,
}

/// Hash exact observed bytes, not semantic equivalence, freshness or identity.
fn hash(raw: &[u8]) -> String { format!("{:x}", Sha256::digest(raw)) }
/// Preserve a typed local cause behind a static profile diagnostic; source
/// content and arbitrary paths must never be rendered from the cause chain.
fn boundary(code: &'static str, cause: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Context(crate::project_context::profile_boundary_error(code, cause))
}
/// Go instruction-reference shape. Ordered aliases/nulls precede conversion;
/// a valid wire object alone grants no path, role or execution permission.
fn instruction_fields() -> crate::model_wire::Fields {
    use crate::model_wire::Kind;
    &[("path",Kind::String),("roles",Kind::Array(&Kind::String))]
}
/// Go profile struct shape, shared by request and active-document decoding.
/// This describes representation; all-role/source admission remains separate.
fn profile_fields() -> crate::model_wire::Fields {
    use crate::model_wire::Kind;
    &[("version",Kind::Int64),("name",Kind::String),("instructions",Kind::Array(&Kind::Object(instruction_fields))),("source",Kind::String)]
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct ApplyRequest { profile: ProjectProfileDefinition, expected_sha256: String }
/// Project ordered aliases/nulls with the same struct shape used for stored Go
/// data. Unknown envelope/profile fields refuse before any publication or IO.
fn decode(raw: &[u8]) -> Result<ApplyRequest,E> {
    use crate::model_wire::Kind;
    crate::work::decode_mode(raw,&[("profile",Kind::Object(profile_fields)),("expected_sha256",Kind::String)],true)
        .map_err(|e|boundary("project-profile-request-invalid",e))
}
/// Canonical Go-compatible pretty JSON plus newline. This is representation,
/// not validation: the ordinary context reader must validate every role first.
fn canonical(profile: &ProjectProfileDefinition) -> Result<String,E> {
    let pretty=serde_json::to_string_pretty(profile).map_err(|e|boundary("project-profile-encode",e))?;
    let raw=serde_json::value::RawValue::from_string(pretty).map_err(|e|boundary("project-profile-encode",e))?;
    let mut bytes=swarm_wire_json::encode(&raw).map_err(|e|boundary("project-profile-encode",e))?;
    bytes.push(b'\n');
    String::from_utf8(bytes).map_err(|e|boundary("project-profile-encode",e))
}
/// Validate all five protocol roles without materializing a candidate profile.
/// Prompts are dropped; only configuration and source metadata can escape.
fn validate(root: &Path, profile: &ProjectProfileDefinition, limits: &ContextPolicyLimits) -> Result<(String,Vec<ProjectContext>),E> {
    let raw=canonical(profile)?;
    let roles=ROLES.iter().map(|role|crate::project_context::read_project_context_document(root,&raw,role,limits).map(|v|v.context).map_err(E::Context)).collect::<Result<Vec<_>,_>>()?;
    Ok((raw,roles))
}
/// Read bounded current bytes or observed absence, retaining path/text/close
/// failures. Never invent a digest or valid profile from an inaccessible file.
fn current(root: &Path, limits: &ContextPolicyLimits) -> Result<Option<String>,E> {
    crate::project_context::read_project_profile_document(root,limits).map_err(E::Context)
}
/// Only observed metadata NotFound means an optional suggestion is absent.
/// Unsafe/nonregular sources and other IO failures stay explicit, not omissions.
fn present(root: &Path, path: &str) -> Result<bool,E> {
    match std::fs::symlink_metadata(root.join(path)) {
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(false),
        Err(e)=>Err(boundary("project-profile-source-metadata",e)),
        Ok(_)=>crate::local_file(root,path).map(|_|true).map_err(E::Path),
    }
}
/// Project only safe typed reasons into unavailable-choice metadata. Unknown
/// error families retain their boxed cause and get an explicit generic failure,
/// never an empty successful profile or permission inference.
fn safe_reason(error: &E) -> &'static str {
    match error { E::Context(e)=>e.reason(),E::Path(e)=>e.reason(),E::Refused(code)=>code,_=>"project-profile-source-unavailable" }
}
/// Verify BOTH the held project's child datastore and its current pathname.
/// Child database identity uses O_PATH, never a second SQLite data descriptor.
/// This prevents authority transferring to a pathname replacement, not same-UID
/// OS confinement or an atomic snapshot of all source files.
fn verify(root: &RootStore, project: &LocalDirectory, binding: &RootBinding) -> Result<(),E> {
    let child=project.child(".swarm",false).map_err(E::Storage)?;
    let result=(||{
        let expected=binding.instance()?;
        if child.identity().map_err(E::Storage)?!=expected.directory || child.database_identity().map_err(E::Storage)?!=expected.database {
            return Err(E::Refused("project-profile-root-instance-changed"));
        }
        super::material_path::verify_material_path(root,binding)?;
        if !binding.matches(root)? {return Err(E::Refused("project-profile-root-instance-changed"));}
        Ok(())
    })();
    finished(result,child.close().map_err(E::Storage))
}

impl RootStore {
    /// Keep saved settings and project/datastore descriptors around a caller-
    /// authorized operation. Exclusive mode serializes cooperating Root writes;
    /// no extra timeout/retry, source-content approval or filesystem lease exists.
    /// The caller must mark a publication through postwrite/close failures.
    pub(super) fn with_project_profile<T>(&self, exclusive: bool, apply: impl FnOnce(&LocalDirectory,&ContextPolicyLimits)->Result<T,E>) -> Result<T,E> {
        let reservation=self.directory.reserve(exclusive,false).map_err(E::Storage)?;
        let result=(||{
            let binding=self.bind_root()?;
            let result=(||{
                let project=LocalDirectory::open_owned(&self.root).map_err(E::Storage)?;
                let result=(||{
                    verify(self,&project,&binding)?;
                    let limits=self.read_settings()?.values.context_policy_limits().map_err(E::Settings)?;
                    let value=apply(&project,&limits)?;
                    verify(self,&project,&binding)?;
                    Ok(value)
                })();
                finished(result,project.close().map_err(E::Storage))
            })();
            finished(result,binding.close())
        })();
        finished(result,reservation.close().map_err(E::Storage))
    }

    /// Observe the current digest and allowlisted suggestions under saved caps.
    /// No candidate or native permission settings are imported. Unavailable
    /// sources retain safe reasons/original causes; malformed active JSON remains
    /// visible with its digest for explicit repair, not a valid-profile claim.
    pub fn project_profile_catalog(&self) -> Result<ProjectProfileCatalog,E> {
        self.with_project_profile(false,|project,limits|{
            let root=project.process_path();let raw=current(&root,limits)?;
            let mut value=ProjectProfileCatalog{active_sha256:raw.as_ref().map(|s|hash(s.as_bytes())).unwrap_or_default(),active_name:String::new(),active_source:String::new(),choices:vec![],active_error:None,cause:None};
            if let Some(raw)=raw {
                match crate::work::decode_mode::<ProjectProfileDefinition>(raw.as_bytes(),profile_fields(),false) {
                    Ok(p) if !p.name.is_empty()=>{value.active_name=p.name;value.active_source=p.source;},
                    Ok(_)=>value.active_error=Some("project-profile-active-name-invalid"),
                    Err(e)=>{value.active_error=Some("project-profile-active-json-invalid");value.cause=Some(Box::new(boundary("project-profile-active-json-invalid",e)));},
                }
            }
            let name=if value.active_name.is_empty(){self.root.file_name().and_then(|s|s.to_str()).ok_or(E::Refused("project-profile-root-name-invalid"))?.to_owned()}else{value.active_name.clone()};
            let shared=present(&root,"AGENTS.md")?;let memory=present(&root,".claude/field-guide/index.md")?;
            for &(id,title,path) in CHOICES {
                let mut p=ProjectProfileDefinition{version:1,name:name.clone(),source:id.into(),instructions:None};
                if shared {p.instructions.get_or_insert_with(Vec::new).push(ProjectProfileInstruction{path:"AGENTS.md".into(),roles:Some(ROLES.iter().map(|s|s.to_string()).collect())});}
                let mut choice=ProjectProfileChoice{id:id.into(),title:title.into(),available:false,reason:String::new(),profile:p,cause:None};
                let result=(||{
                    if !present(&root,path)? {return Err(E::Refused("project-profile-source-missing"));}
                    if path!="AGENTS.md" {choice.profile.instructions.get_or_insert_with(Vec::new).push(ProjectProfileInstruction{path:path.into(),roles:Some(if shared{vec!["worker".into()]}else{ROLES.iter().map(|s|s.to_string()).collect()})});}
                    if memory {choice.profile.instructions.get_or_insert_with(Vec::new).push(ProjectProfileInstruction{path:".claude/field-guide/index.md".into(),roles:Some(ROLES.iter().map(|s|s.to_string()).collect())});}
                    validate(&root,&choice.profile,limits)?;Ok(())
                })();
                match result {Ok(())=>choice.available=true,Err(e)=>{choice.reason=safe_reason(&e).into();choice.cause=Some(Box::new(e));}}
                value.choices.push(choice);
            }
            Ok(value)
        })
    }

    /// Validate an explicit candidate and compare the current configuration SHA.
    /// No publication, temporary live profile, budget or agent is created; saved
    /// caps and all source guards apply. A later apply revalidates source bytes.
    pub fn project_profile_preview(&self, raw: &[u8]) -> Result<ProjectProfilePreview,E> {
        let request=decode(raw)?;
        self.with_project_profile(false,|project,limits|{
            let root=project.process_path();let actual=current(&root,limits)?.map(|v|hash(v.as_bytes())).unwrap_or_default();
            if actual!=request.expected_sha256 {return Err(E::Refused("project-profile-conflict"));}
            let (_,roles)=validate(&root,&request.profile,limits)?;
            Ok(ProjectProfilePreview{active_sha256:actual,profile:request.profile,roles})
        })
    }

    /// Apply one explicit configuration after digest CAS and all-role admission.
    /// Atomic private-file publication preserves the old file on pre-rename
    /// failure; absent targets use NOREPLACE. Known/possible publication survives
    /// durability, descriptor, status-read and later adapter failures via Published.
    /// No model/permission import, budget change or agent start follows. External
    /// editors are not confined; cooperating Root writes share the exclusive lock.
    pub fn project_profile_apply(&self, raw: &[u8]) -> Result<ProjectProfileStatus,E> {
        if !self.writable {return Err(E::Refused("root-read-only"));}
        let request=decode(raw)?;let mut published=false;
        let result=self.with_project_profile(true,|project,limits|{
            let root=project.process_path();let old=current(&root,limits)?;let actual=old.as_ref().map(|v|hash(v.as_bytes())).unwrap_or_default();
            if actual!=request.expected_sha256 {return Err(E::Refused("project-profile-conflict"));}
            let (raw,_)=validate(&root,&request.profile,limits)?;
            let result=if old.is_none(){project.publish_new_with_fault("swarm.project.json",raw.as_bytes(),crate::local_storage::AtomicStorageFault::None)}else{project.publish("swarm.project.json",raw.as_bytes())};
            if let Err(e)=result {published=e.publication!=crate::local_storage::PublicationState::NotPublished;return Err(E::Storage(e));}
            published=true;Ok(())
        }).and_then(|()|self.project_profile_status());
        match result {Err(primary) if published=>Err(E::Published{primary:Box::new(primary)}),other=>other}
    }

    /// Rebuild the named allowlisted suggestion and apply it with the supplied
    /// current SHA. This Go-compatible shorthand is explicit and launches no agent;
    /// UI clients may instead preview/apply the exact chosen profile definition.
    pub fn project_profile_select(&self, id: &str, raw: &[u8]) -> Result<ProjectProfileStatus,E> {
        #[derive(Default,Deserialize)]#[serde(default)]struct Request{expected_sha256:String}
        let request:Request=crate::work::decode_mode(raw,&[("expected_sha256",crate::model_wire::Kind::String)],true).map_err(|e|boundary("project-profile-request-invalid",e))?;
        let catalog=self.project_profile_catalog()?;
        let choice=catalog.choices.into_iter().find(|c|c.id==id).ok_or(E::Refused("project-profile-choice-unknown"))?;
        if !choice.available {return Err(choice.cause.map(|e|*e).unwrap_or(E::Refused("project-profile-choice-unavailable")));}
        #[derive(Serialize)]struct Apply<'a>{profile:&'a ProjectProfileDefinition,expected_sha256:&'a str}
        let raw=crate::encode_model(&Apply{profile:&choice.profile,expected_sha256:&request.expected_sha256}).map_err(E::Encode)?;
        self.project_profile_apply(&raw)
    }

    /// Decode the protected API's id/expected-SHA envelope with ordered Go
    /// alias/null semantics, then use the same selection and CAS as the CLI.
    /// No object normalization precedes validation and no execution right follows.
    pub fn project_profile_select_document(&self, raw: &[u8]) -> Result<ProjectProfileStatus,E> {
        #[derive(Default,Deserialize)]#[serde(default)]struct Selection{id:String,expected_sha256:String}
        let r:Selection=crate::work::decode_mode(raw,&[("id",crate::model_wire::Kind::String),("expected_sha256",crate::model_wire::Kind::String)],true).map_err(|e|boundary("project-profile-request-invalid",e))?;
        #[derive(Serialize)]struct Expected<'a>{expected_sha256:&'a str}
        let raw=crate::encode_model(&Expected{expected_sha256:&r.expected_sha256}).map_err(E::Encode)?;
        self.project_profile_select(&r.id,&raw)
    }
}
