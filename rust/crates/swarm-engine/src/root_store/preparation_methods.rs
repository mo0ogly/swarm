//! Finite method capabilities and actual local-or-shipped byte observations.
use super::{preparation_read::PreparationReadObservation, RootStoreError as E};
use crate::PreparationMethod;
use sha2::{Digest, Sha256};
use std::path::Path;

// Editorial capability catalogue, not execution policy or operator-selected paths.
const METHODS: &[(&str, &str, &[&str], &[&str])] = &[
    ("apex", "Analyse et planification", &["Analyze", "Plan"], &[".claude/skills/apex/SKILL.md"]),
    ("ks-feature", "Parcours guidé — préparer une évolution", &["Cadrage", "Plan"], &[".claude/commands/ks-feature.md", ".claude/commands/ks-plan.md", ".claude/skills/product-planning/SKILL.md"]),
    ("ks-product", "Parcours de création d’application", &["Cadrage produit", "Parcours et stories", "Architecture", "Conception", "Plan"], &[".claude/skills/product-planning/SKILL.md", ".claude/skills/product-delivery/SKILL.md", ".claude/skills/product-review/SKILL.md"]),
    ("debug", "Diagnostiquer et corriger un problème", &["Diagnostic", "Plan"], &[".claude/skills/debug/SKILL.md"]),
    ("audit-pdca", "Examiner et améliorer — préparer l’examen", &["Plan"], &[".claude/skills/audit-pdca/SKILL.md"]),
];
/// Test finite capability membership before any source IO. This is protocol
/// vocabulary, not a default, filesystem path, provider or execution grant.
pub(super) fn known(id:&str)->bool {METHODS.iter().any(|(name,_,_,_)|*name==id)}
/// Resolve only packaged capability paths. A missing entry is an internal build
/// contract error, never permission to read an arbitrary path or hide failure.
fn bundled(path: &str) -> Result<&'static [u8], E> {
    match path {
        ".claude/skills/apex/SKILL.md" => Ok(include_bytes!("../../../../../.claude/skills/apex/SKILL.md")),
        ".claude/commands/ks-feature.md" => Ok(include_bytes!("../../../../../.claude/commands/ks-feature.md")),
        ".claude/commands/ks-plan.md" => Ok(include_bytes!("../../../../../.claude/commands/ks-plan.md")),
        ".claude/skills/product-planning/SKILL.md" => Ok(include_bytes!("../../../../../.claude/skills/product-planning/SKILL.md")),
        ".claude/skills/product-delivery/SKILL.md" => Ok(include_bytes!("../../../../../.claude/skills/product-delivery/SKILL.md")),
        ".claude/skills/product-review/SKILL.md" => Ok(include_bytes!("../../../../../.claude/skills/product-review/SKILL.md")),
        ".claude/skills/debug/SKILL.md" => Ok(include_bytes!("../../../../../.claude/skills/debug/SKILL.md")),
        ".claude/skills/audit-pdca/SKILL.md" => Ok(include_bytes!("../../../../../.claude/skills/audit-pdca/SKILL.md")),
        "tools/agent-workflows/CONTRACT.md" => Ok(include_bytes!("../../../../../tools/agent-workflows/CONTRACT.md")),
        _ => Err(E::Refused("preparation-method-bundle-missing")),
    }
}
/// Observe each finite method's complete source bytes, using the supplied saved
/// byte cap for both local and bundled files. Unknown/unsafe IO produces an
/// explicit unavailable entry and retains original causes in the sidecar; it is
/// never a fresh method or bundled fallback. Hash order/zero separator/newline
/// follow Go. No documents are returned or changed and no plan is accepted.
pub(super) fn catalogue(root: &Path, cap: Option<usize>) -> Result<PreparationReadObservation<Vec<PreparationMethod>>, E> {
    let mut methods = Vec::new(); let mut causes = Vec::new();
    for (id, title, phases, paths) in METHODS {
        let mut method = PreparationMethod { id: (*id).into(), title: (*title).into(),
            phases: phases.iter().map(|s| (*s).into()).collect(),
            paths: paths.iter().chain(std::iter::once(&"tools/agent-workflows/CONTRACT.md")).map(|s| (*s).into()).collect(),
            sha256: String::new(), available: true, reason: String::new() };
        let mut joined = String::new();
        for path in &method.paths {
            let fallback = bundled(path)?;
            match super::preparation_source::read_source(root, path, fallback, cap) {
                Ok(bytes) => joined.push_str(&format!("{path}\0{:x}\n", Sha256::digest(bytes))),
                Err(error) => { method.available = false; method.reason = error.code().into(); causes.push(error); break; }
            }
        }
        if method.available { method.sha256 = format!("{:x}", Sha256::digest(joined.as_bytes())); }
        methods.push(method);
    }
    Ok(PreparationReadObservation { value: methods, causes })
}

/// Read the selected finite method's full UTF-8 sources and verify their exact
/// aggregate against the caller's current catalogue hash. Sources are private
/// prompt input only; no arbitrary path, silent fallback on IO, or execution
/// permission is introduced. Concurrent byte changes refuse before admission.
pub(super) fn prompt_sources(root:&Path,method:&PreparationMethod,cap:Option<usize>)->Result<std::collections::BTreeMap<String,String>,E>{
    let mut sources=std::collections::BTreeMap::new();let mut joined=String::new();
    for path in &method.paths {
        let bytes=super::preparation_source::read_source(root,path,bundled(path)?,cap)?;
        joined.push_str(&format!("{path}\0{:x}\n",Sha256::digest(&bytes)));
        sources.insert(path.clone(),String::from_utf8(bytes).map_err(|e|super::preparation_read::cause("preparation-method-utf8",e))?);
    }
    if format!("{:x}",Sha256::digest(joined.as_bytes()))!=method.sha256{return Err(E::Refused("preparation-stale-method"));}
    Ok(sources)
}
