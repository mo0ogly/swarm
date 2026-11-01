//! Byte provenance for the prompt assembled by the preparation producer.
//! Hashes describe observed bytes; they establish neither classification nor
//! identity, filesystem confinement, or permission to disclose those bytes.
use super::{preparation_read::cause, RootStoreError as E};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Wire fields consumed by the preparation producer. This is a schema inventory,
/// not a configurable limit. New inputs require an explicit provenance review.
const DATA_FIELDS: &[&str] = &[
    "adopted_brief", "documents", "earlier_exchanges_not_sent", "history",
    "history_mode", "method", "method_sources", "sources", "user_message",
];

/// Producer origin of a contiguous slice. PreparationData names the actual JSON
/// field, including empty/null values; omissions never imply a public label.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Origin {
    ProjectContext,
    EngineInstructions,
    PlanContract,
    DataFraming,
    PreparationData { field: String },
}

/// Ordered, nonoverlapping byte range in the exact transmitted prompt. No
/// private source text is duplicated in this metadata.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Part {
    origin: Origin,
    start: usize,
    end: usize,
    sha256: String,
}

/// Engine-produced provenance, retained only in the private native envelope.
/// The digest protects consistency, not authenticity against a datastore writer.
/// No classification or disclosure grant can be obtained from this structure.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    schema_version: u32,
    prompt_bytes: usize,
    prompt_sha256: String,
    parts: Vec<Part>,
}

/// Compute the digest of exactly the supplied bytes without normalization.
fn hash(raw: &[u8]) -> String { format!("{:x}", Sha256::digest(raw)) }

impl Manifest {
    /// Verify complete ordered coverage, UTF-8 boundaries, byte digests and
    /// origin-to-JSON-key correspondence in the producer's framing grammar.
    /// Refuses malformed/stale provenance without side effects or fallback.
    /// This checks consistency only; BLP attribution and sink admission remain
    /// separate required contracts, including for historical rows without it.
    pub(super) fn verify(&self, prompt: &str) -> Result<(), E> {
        let invalid = || E::Refused("preparation-prompt-provenance-invalid");
        if self.schema_version != 1 || self.prompt_bytes != prompt.len()
            || self.prompt_sha256 != hash(prompt.as_bytes()) || self.parts.is_empty()
        { return Err(invalid()); }
        let mut cursor = 0;
        let mut fields = std::collections::BTreeSet::new();
        for part in &self.parts {
            if part.start != cursor || part.end <= part.start { return Err(invalid()); }
            let bytes = prompt.get(part.start..part.end).ok_or_else(invalid)?;
            if hash(bytes.as_bytes()) != part.sha256 { return Err(invalid()); }
            if let Origin::PreparationData { field } = &part.origin {
                if !DATA_FIELDS.contains(&field.as_str()) || !fields.insert(field.as_str()) {
                    return Err(invalid());
                }
            }
            cursor = part.end;
        }
        if cursor != prompt.len() || fields.len() != DATA_FIELDS.len() { return Err(invalid()); }
        // Digests alone permit swapping origin names without changing bytes.
        // Bind each data origin to its real serialized key and check the entire
        // assembly grammar; no metadata-only relabel can pass this check.
        let framing = |index: usize, text: &str| self.parts.get(index).is_some_and(|part|
            matches!(part.origin, Origin::DataFraming) && prompt.get(part.start..part.end) == Some(text));
        let mut index = 0;
        if matches!(self.parts.get(index).map(|p| &p.origin), Some(Origin::ProjectContext)) { index += 1; }
        if !matches!(self.parts.get(index).map(|p| &p.origin), Some(Origin::EngineInstructions)) { return Err(invalid()); }
        index += 1;
        if matches!(self.parts.get(index).map(|p| &p.origin), Some(Origin::PlanContract)) { index += 1; }
        if !framing(index, "\nDONNEES_JSON :\n{") { return Err(invalid()); }
        index += 1;
        for ordinal in 0..DATA_FIELDS.len() {
            let Some(Part { origin: Origin::PreparationData { field }, .. }) = self.parts.get(index + 1)
                else { return Err(invalid()); };
            let key = serde_json::to_string(field).map_err(|e| cause("preparation-prompt-encode", e))?;
            let expected = format!("{}{key}:", if ordinal == 0 { "" } else { "," });
            if !framing(index, &expected) { return Err(invalid()); }
            index += 2;
        }
        if !framing(index, "}") || index + 1 != self.parts.len() { return Err(invalid()); }
        Ok(())
    }
}

/// Append observed bytes and their producer origin, omitting only zero-length
/// slices. Source JSON still represents every field, even when its value is null.
fn append(prompt: &mut String, parts: &mut Vec<Part>, origin: Origin, text: &str) {
    if text.is_empty() { return; }
    let start = prompt.len();
    prompt.push_str(text);
    parts.push(Part { origin, start, end: prompt.len(), sha256: hash(text.as_bytes()) });
}

/// Assemble the existing preparation wire prompt with provenance over every byte.
/// Inputs are producer-captured context, engine instructions, optional plan
/// contract, nonempty engine instructions, and the exact nine-field preparation
/// data object. Unknown/missing
/// fields or encoding failures refuse. Returns prompt plus private metadata;
/// performs no IO, policy decision, source classification or provider call.
pub(super) fn assemble(project: Option<&str>, instructions: &str, plan: Option<&str>, data: &Value)
    -> Result<(String, Manifest), E>
{
    let data = data.as_object().ok_or(E::Refused("preparation-prompt-data-invalid"))?;
    if data.len() != DATA_FIELDS.len() || DATA_FIELDS.iter().any(|key| !data.contains_key(*key)) {
        return Err(E::Refused("preparation-prompt-data-invalid"));
    }
    let mut prompt = String::new();
    let mut parts = Vec::new();
    if let Some(project) = project { append(&mut prompt, &mut parts, Origin::ProjectContext, project); }
    append(&mut prompt, &mut parts, Origin::EngineInstructions, instructions);
    if let Some(plan) = plan { append(&mut prompt, &mut parts, Origin::PlanContract, plan); }
    append(&mut prompt, &mut parts, Origin::DataFraming, "\nDONNEES_JSON :\n{");
    for (index, (key, value)) in data.iter().enumerate() {
        let key_wire = serde_json::to_string(key).map_err(|e| cause("preparation-prompt-encode", e))?;
        let framing = format!("{}{key_wire}:", if index == 0 { "" } else { "," });
        append(&mut prompt, &mut parts, Origin::DataFraming, &framing);
        let wire = serde_json::to_string(value).map_err(|e| cause("preparation-prompt-encode", e))?;
        append(&mut prompt, &mut parts, Origin::PreparationData { field: key.clone() }, &wire);
    }
    append(&mut prompt, &mut parts, Origin::DataFraming, "}");
    let manifest = Manifest { schema_version: 1, prompt_bytes: prompt.len(), prompt_sha256: hash(prompt.as_bytes()), parts };
    manifest.verify(&prompt)?;
    Ok((prompt, manifest))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// All producer fields, including retained secrets as inert fixture text;
    /// no real data, identity or provider is involved.
    fn data() -> Value {
        serde_json::json!({"documents":{"besoin":"échéance"},"sources":[{"text":"source"}],
            "adopted_brief":null,"method":"apex","method_sources":["instructions"],
            "history":[{"question":"Q","answer":"A"}],"history_mode":"full",
            "earlier_exchanges_not_sent":0,"user_message":"courant\n\"x\""})
    }
    #[test]
    fn actual_wire_bytes_and_all_sources_are_preserved() {
        for project in [None, Some("profil é\n")] {
            for plan in [None, Some("contrat\n")] {
                let data = data();
                let (prompt, manifest) = assemble(project, "instructions\n", plan, &data).unwrap();
                assert_eq!(prompt, format!("{}instructions\n{}\nDONNEES_JSON :\n{}", project.unwrap_or(""), plan.unwrap_or(""), serde_json::to_string(&data).unwrap()));
                manifest.verify(&prompt).unwrap();
                for part in &manifest.parts {
                    if let Origin::PreparationData { field } = &part.origin {
                        assert_eq!(&prompt[part.start..part.end], serde_json::to_string(&data[field]).unwrap());
                    }
                }
                let wire = serde_json::to_string(&manifest).unwrap();
                assert!(!wire.contains("échéance")); assert!(!wire.contains("courant"));
                assert!(!wire.contains("classification"));
                serde_json::from_str::<Manifest>(&wire).unwrap().verify(&prompt).unwrap();
            }
        }
    }
    #[test]
    fn mutated_prompt_refuses_and_original_recovers() {
        let (prompt, manifest) = assemble(None, "instruction", None, &data()).unwrap();
        manifest.verify(&prompt).unwrap();
        for changed in [prompt.replace("courant", "altéré"), format!("{prompt}x"), prompt[..prompt.len()-1].to_string()] {
            assert!(manifest.verify(&changed).is_err());
        }
        manifest.verify(&prompt).unwrap();
    }
    #[test]
    fn gaps_overlap_truncation_and_false_origins_refuse() {
        let (prompt, original) = assemble(Some("é"), "instruction", None, &data()).unwrap();
        original.verify(&prompt).unwrap();
        let mut cases = Vec::new();
        let mut m = original.clone(); m.parts[1].start += 1; cases.push(m);
        let mut m = original.clone(); m.parts[1].start -= 1; cases.push(m);
        let mut m = original.clone(); m.parts[0].end = 1; cases.push(m);
        let mut m = original.clone(); m.parts.pop(); cases.push(m);
        let mut m = original.clone(); m.parts[0].sha256 = hash(b"forged"); cases.push(m);
        let mut m = original.clone(); m.schema_version = 2; cases.push(m);
        let mut m = original.clone(); m.parts.iter_mut().find(|p| matches!(p.origin, Origin::PreparationData { .. })).unwrap().origin = Origin::DataFraming; cases.push(m);
        let mut m = original.clone();
        let data_parts: Vec<_> = m.parts.iter_mut().filter(|p| matches!(p.origin, Origin::PreparationData { .. })).take(2).collect();
        let [first, second]: [&mut Part; 2] = data_parts.try_into().ok().unwrap();
        std::mem::swap(&mut first.origin, &mut second.origin); cases.push(m);
        for m in cases { assert!(m.verify(&prompt).is_err()); }
    }
    #[test]
    fn new_or_missing_producer_inputs_require_schema_review() {
        let mut d = data(); assemble(None, "instructions", None, &d).unwrap();
        d["future_source"] = "new input".into(); assert!(assemble(None, "instructions", None, &d).is_err());
        d.as_object_mut().unwrap().remove("future_source");
        d.as_object_mut().unwrap().remove("user_message"); assert!(assemble(None, "instructions", None, &d).is_err());
        assert!(assemble(None, "instructions", None, &Value::Null).is_err());
    }
}
