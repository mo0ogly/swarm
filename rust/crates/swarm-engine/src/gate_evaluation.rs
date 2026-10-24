//! Read-only score calculation. It grants no task acceptance or execution rights.
use crate::{
    gate_json::{Document, JsonError},
    local_file, Evaluation, GoFloat, QualityPolicySnapshot, QualityPolicyValues,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    fs::File,
    io::{self, Read},
    path::Path,
};
pub struct GateEvaluationError {
    reason: &'static str,
    cause: Option<Box<dyn Error + Send + Sync>>,
    cleanup: Option<io::Error>,
}
impl GateEvaluationError {
    fn invalid(reason: &'static str) -> Self {
        Self {
            reason,
            cause: None,
            cleanup: None,
        }
    }
    fn caused(reason: &'static str, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            reason,
            cause: Some(Box::new(cause)),
            cleanup: None,
        }
    }
    pub fn code(&self) -> &'static str {
        "evaluation-invalid"
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
    pub fn cleanup_cause(&self) -> Option<&io::Error> {
        self.cleanup.as_ref()
    }
}
impl fmt::Display for GateEvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "evaluation-invalid: {}", self.reason)
    }
}
impl fmt::Debug for GateEvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for GateEvaluationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as _)
    }
}
impl From<JsonError> for GateEvaluationError {
    fn from(e: JsonError) -> Self {
        Self::caused("json", e)
    }
}
type Result<T> = std::result::Result<T, GateEvaluationError>;
fn invalid(reason: &'static str) -> GateEvaluationError {
    GateEvaluationError::invalid(reason)
}
fn text(d: &Document, id: usize, key: &str) -> String {
    d.get(id, key).and_then(|i| d.text(i)).unwrap_or_default()
}
/// Inspect using the same loss-tolerant, nonrecursive decoder as evaluation.
/// This selects a calculation format; it grants no authority to use that format.
pub fn gate_document_method(raw: &[u8]) -> Result<String> {
    let d = Document::parse(raw)?;
    if d.object(0).is_none() {
        return Err(invalid("document-object-required"));
    }
    Ok(text(&d, 0, "method_version"))
}
fn number(d: &Document, id: usize, key: &str) -> Result<f64> {
    d.get(id, key)
        .and_then(|i| d.number(i))
        .ok_or_else(|| invalid("finite-nonnegative-number-required"))
}
pub(crate) fn records(d: &Document, id: Option<usize>) -> Result<Vec<usize>> {
    let values = id
        .and_then(|i| d.array(i))
        .ok_or_else(|| invalid("record-array-required"))?;
    let mut seen = BTreeSet::new();
    for &value in values {
        if d.object(value).is_none() {
            return Err(invalid("record-object-required"));
        }
        let id = text(d, value, "id");
        if crate::text::trim_space(&id).is_empty() || !seen.insert(id) {
            return Err(invalid("record-id-required-or-duplicate"));
        }
    }
    Ok(values.to_vec())
}
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Hash all bytes through the checked artifact descriptor without retaining the file.
/// Any path/regular-file/read/close failure refuses rather than hashing partial data.
pub(crate) fn artifact_hash(root: &Path, name: &str) -> Result<(std::path::PathBuf, String)> {
    artifact_access(root, name, |file| {
        let mut sha = Sha256::new();
        let mut bytes = [0u8; 8192];
        loop {
            let n = match file.read(&mut bytes) {
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(GateEvaluationError::caused("artifact-read", e)),
            };
            if n == 0 {
                break;
            }
            sha.update(&bytes[..n]);
        }
        Ok(format!("{:x}", sha.finalize()))
    })
}
/// Observe metadata under the contained regular-artifact path contract used by
/// gate hashing. Selected file symlinks may resolve to regular files inside the
/// project through local_file; no UID, private-mode or content-read requirement
/// is added. Linux uses an O_PATH descriptor (metadata access only), checks its
/// regular type and closes explicitly; other platforms use their stat operation.
/// Returned metadata is a point-in-time observation, not content, attribution,
/// a later-action lease or an atomic pathname/SQL snapshot. Path/open/metadata/
/// type/close failures retain their original causes, including both failures.
pub(crate) fn artifact_metadata(
    root: &Path,
    name: &str,
) -> Result<(std::path::PathBuf, std::fs::Metadata)> {
    let path =
        local_file(root, name).map_err(|e| GateEvaluationError::caused("artifact-path", e))?;
    #[cfg(target_os = "linux")]
    {
        let file = File::from(
            rustix::fs::open(
                &path,
                rustix::fs::OFlags::PATH
                    | rustix::fs::OFlags::CLOEXEC
                    | rustix::fs::OFlags::NOFOLLOW,
                rustix::fs::Mode::empty(),
            )
            .map_err(|e| GateEvaluationError::caused("artifact-open", io::Error::from(e)))?,
        );
        let result = (|| {
            let metadata = file
                .metadata()
                .map_err(|e| GateEvaluationError::caused("artifact-metadata", e))?;
            if !metadata.is_file() {
                return Err(invalid("artifact-regular-file-required"));
            }
            Ok(metadata)
        })();
        let closed = nix::unistd::close(file).map_err(io::Error::from);
        match (result, closed) {
            (Ok(metadata), Ok(())) => Ok((path, metadata)),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(e)) => Err(GateEvaluationError::caused("artifact-close", e)),
            (Err(mut error), Err(e)) => {
                error.cleanup = Some(e);
                Err(error)
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let metadata = std::fs::metadata(&path)
            .map_err(|e| GateEvaluationError::caused("artifact-metadata", e))?;
        if !metadata.is_file() {
            return Err(invalid("artifact-regular-file-required"));
        }
        Ok((path, metadata))
    }
}
/// Read the bytes that a context consumer will hash and parse from the same buffer.
/// Allocation is fallible, there is no implicit truncation/cap, and close errors
/// prevent success. The caller must compare the buffer's digest before parsing.
pub(crate) fn artifact_bytes(root: &Path, name: &str) -> Result<(std::path::PathBuf, Vec<u8>)> {
    artifact_bytes_limited(root, name, None)
}
/// Read a complete artifact under an explicit optional byte policy. Buffer8192 is
/// only a streaming chunk, not a policy cap. A finite cap permits reading one
/// overflow witness byte but never retaining, hashing or parsing a truncated proof.
/// Allocation/read/path/regular-file/close errors refuse with preserved causes;
/// descriptor/path observation provides no later lease or OS confinement.
pub(crate) fn artifact_bytes_limited(
    root: &Path,
    name: &str,
    limit: Option<usize>,
) -> Result<(std::path::PathBuf, Vec<u8>)> {
    artifact_access(root, name, |file| {
        let mut raw = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            let request = limit.map_or(buffer.len(), |cap| {
                cap.saturating_sub(raw.len())
                    .saturating_add(1)
                    .min(buffer.len())
            });
            let n = match file.read(&mut buffer[..request]) {
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(GateEvaluationError::caused("artifact-read", e)),
            };
            if n == 0 {
                break;
            }
            if limit.is_some_and(|cap| n > cap.saturating_sub(raw.len())) {
                return Err(invalid("artifact-byte-limit-exceeded"));
            }
            raw.try_reserve(n)
                .map_err(|e| GateEvaluationError::caused("artifact-buffer", e))?;
            raw.extend_from_slice(&buffer[..n]);
        }
        Ok(raw)
    })
}
/// Open one regular contained artifact, apply its reader and preserve primary/close errors.
/// Paths are observations, not an OS confinement or later-action lease.
fn artifact_access<T>(
    root: &Path,
    name: &str,
    read: impl FnOnce(&mut File) -> Result<T>,
) -> Result<(std::path::PathBuf, T)> {
    let path =
        local_file(root, name).map_err(|e| GateEvaluationError::caused("artifact-path", e))?;
    #[cfg(unix)]
    let mut file = File::from(
        rustix::fs::open(
            &path,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::NONBLOCK
                | rustix::fs::OFlags::CLOEXEC
                | rustix::fs::OFlags::NOFOLLOW,
            rustix::fs::Mode::empty(),
        )
        .map_err(|e| GateEvaluationError::caused("artifact-open", io::Error::from(e)))?,
    );
    #[cfg(not(unix))]
    let mut file =
        File::open(&path).map_err(|e| GateEvaluationError::caused("artifact-open", e))?;
    let result = (|| {
        if !file
            .metadata()
            .map_err(|e| GateEvaluationError::caused("artifact-metadata", e))?
            .is_file()
        {
            return Err(invalid("artifact-regular-file-required"));
        }
        read(&mut file)
    })();
    #[cfg(unix)]
    let closed = nix::unistd::close(file).map_err(io::Error::from);
    #[cfg(not(unix))]
    let closed = {
        drop(file);
        Ok::<_, io::Error>(())
    };
    match (result, closed) {
        (Ok(hash), Ok(())) => Ok((path, hash)),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(e)) => Err(GateEvaluationError::caused("artifact-close", e)),
        (Err(mut error), Err(e)) => {
            error.cleanup = Some(e);
            Err(error)
        }
    }
}
fn rank(phase: &str) -> Option<i64> {
    match phase {
        "entry" => Some(0),
        "validation" => Some(1),
        "delivery" | "audit" => Some(2),
        _ => None,
    }
}
fn severity(s: &str) -> Option<i64> {
    match s {
        "none" => Some(0),
        "minor" => Some(1),
        "major" => Some(2),
        "critical" => Some(3),
        _ => None,
    }
}
fn round2(n: f64) -> Result<GoFloat> {
    if !n.is_finite() {
        return Err(invalid("numeric-representation"));
    }
    let value = format!("{n:.2}")
        .parse()
        .map_err(|e| GateEvaluationError::caused("rounding-representation", e))?;
    Ok(GoFloat(value))
}
struct Row {
    id: String,
    domain: String,
    gate: String,
    severity: String,
    status: String,
    mandatory: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BoundGateEvaluation {
    pub evaluation: Evaluation,
    pub policy: QualityPolicySnapshot,
}
pub fn evaluate_gate_method2(raw: &[u8], root: &Path, phase: &str) -> Result<Evaluation> {
    let policy = QualityPolicyValues::legacy_method2()
        .map_err(|e| GateEvaluationError::caused("legacy-policy", e))?;
    evaluate(raw, root, phase, "2", &policy, None)
}
pub fn evaluate_gate_with_policy(
    raw: &[u8],
    root: &Path,
    phase: &str,
    policy: &QualityPolicySnapshot,
) -> Result<BoundGateEvaluation> {
    policy
        .validate_binding()
        .map_err(|e| GateEvaluationError::caused("policy-binding", e))?;
    let evaluation = evaluate(raw, root, phase, "3", &policy.values, Some(&policy.digest))?;
    Ok(BoundGateEvaluation {
        evaluation,
        policy: policy.clone(),
    })
}
fn evaluate(
    raw: &[u8],
    root: &Path,
    phase: &str,
    method: &str,
    policy: &QualityPolicyValues,
    policy_digest: Option<&str>,
) -> Result<Evaluation> {
    let phase_rank = rank(phase).ok_or_else(|| invalid("unknown-phase"))?;
    let mut d = Document::parse(raw)?;
    if d.object(0).is_none()
        || text(&d, 0, "method_version") != method
        || crate::text::trim_space(&text(&d, 0, "scope_id")).is_empty()
    {
        return Err(invalid("method-or-scope"));
    }
    let mut out = Evaluation {
        method_version: method.into(),
        scope_id: text(&d, 0, "scope_id"),
        phase: phase.into(),
        artifacts: Some(BTreeMap::new()),
        domain_scores: Some(BTreeMap::new()),
        blockers: Some(Vec::new()),
        max_severity: "none".into(),
        ..Default::default()
    };
    let artifacts = d
        .get(0, "artifacts")
        .and_then(|i| d.object(i))
        .filter(|m| !m.is_empty())
        .ok_or_else(|| invalid("artifacts-required"))?;
    let mut protected = BTreeSet::new();
    for (name, &id) in artifacts {
        let (path, digest) = artifact_hash(root, name)?;
        if Some(digest.as_str()) != d.text(id).as_deref() {
            return Err(invalid("artifact-stale"));
        }
        out.artifacts.as_mut().unwrap().insert(name.clone(), digest);
        protected.insert(path);
    }
    let domains_id = d
        .get(0, "domains")
        .ok_or_else(|| invalid("domains-required"))?;
    let domains = d
        .object(domains_id)
        .filter(|m| !m.is_empty())
        .ok_or_else(|| invalid("domains-required"))?;
    let mut weights = BTreeMap::new();
    for (name, &id) in domains {
        let n = d
            .number(id)
            .filter(|n| *n > 0.0)
            .ok_or_else(|| invalid("weight-invalid"))?;
        if crate::text::trim_space(name).is_empty() {
            return Err(invalid("weight-invalid"));
        }
        weights.insert(name.clone(), n);
    }
    let mut checks = records(&d, d.get(0, "checks"))?;
    if checks.is_empty() {
        return Err(invalid("checks-required"));
    }
    let results = records(&d, d.get(0, "results"))?;
    let known: BTreeSet<_> = checks.iter().map(|&c| text(&d, c, "id")).collect();
    let mut observations = BTreeMap::new();
    for r in results {
        let id = text(&d, r, "id");
        if !known.contains(&id) {
            return Err(invalid("unknown-result"));
        }
        observations.insert(id, r);
    }
    let mut rows = Vec::new();
    let mut applicable = Vec::new();
    let mut penalties = BTreeMap::<String, f64>::new();
    for &c in &checks {
        let id = text(&d, c, "id");
        let domain = text(&d, c, "domain");
        if let Some(value) = d.get(c, "domain") {
            if !d.null(value) && (d.text(value).is_none() || !weights.contains_key(&domain)) {
                return Err(invalid("unknown-domain"));
            }
        }
        let mandatory = d
            .get(c, "mandatory")
            .and_then(|i| d.boolean(i))
            .ok_or_else(|| invalid("mandatory-boolean-required"))?;
        let gate = text(&d, c, "gate");
        if !matches!(gate.as_str(), "entry" | "validation" | "delivery") {
            return Err(invalid("invalid-check-gate"));
        }
        let sev = text(&d, c, "severity");
        severity(&sev).ok_or_else(|| invalid("invalid-severity"))?;
        let penalty = number(&d, c, "penalty")?;
        let max = number(&d, c, "max_penalty")?;
        let (status, count, evidence, reason) = if let Some(&r) = observations.get(&id) {
            if d.object(r).unwrap().keys().any(|k| {
                !matches!(
                    k.as_str(),
                    "id" | "status" | "count" | "evidence" | "reason"
                )
            }) {
                return Err(invalid("result-cannot-change-policy"));
            }
            let status = text(&d, r, "status");
            let count = number(&d, r, "count")?;
            let evidence = match d.get(r, "evidence") {
                Some(i) => d
                    .array(i)
                    .ok_or_else(|| invalid("evidence-array-required"))?
                    .to_vec(),
                None => Vec::new(),
            };
            (status, count, evidence, text(&d, r, "reason"))
        } else {
            ("NON TESTÉ".into(), 0.0, Vec::new(), String::new())
        };
        if !matches!(
            status.as_str(),
            "PASS" | "FAIL" | "PARTIEL" | "NON TESTÉ" | "NON APPLICABLE"
        ) {
            return Err(invalid("invalid-result-status"));
        }
        if (status == "FAIL" && count == 0.0)
            || (matches!(status.as_str(), "PASS" | "NON APPLICABLE") && count != 0.0)
        {
            return Err(invalid("result-count-incompatible"));
        }
        if status == "NON APPLICABLE" && crate::text::trim_space(&reason).is_empty() {
            return Err(invalid("not-applicable-reason-required"));
        }
        if matches!(status.as_str(), "PASS" | "FAIL") && evidence.is_empty() {
            return Err(invalid("evidence-required"));
        }
        for item in evidence {
            let name = d.text(item).unwrap_or_default();
            let path = local_file(root, &name)
                .map_err(|e| GateEvaluationError::caused("evidence-path", e))?;
            if !protected.contains(&path) {
                return Err(invalid("evidence-not-in-inventory"));
            }
        }
        if status == "NON APPLICABLE" {
            continue;
        }
        applicable.push(id.clone());
        if !domain.is_empty() {
            let value = penalties.entry(domain.clone()).or_default();
            if status == "FAIL" {
                *value += (count * penalty).min(max)
            }
            if !matches!(status.as_str(), "PASS" | "FAIL") {
                out.provisional = true
            }
        }
        rows.push(Row {
            id,
            domain,
            gate,
            severity: sev,
            status,
            mandatory,
        });
    }
    let (mut sum, mut weight) = (0.0, 0.0);
    for (domain, &penalty) in &penalties {
        let score = (100.0 - penalty).max(0.0);
        out.domain_scores
            .as_mut()
            .unwrap()
            .insert(domain.clone(), GoFloat(score));
        sum += score * weights[domain];
        weight += weights[domain];
        if !sum.is_finite() || !weight.is_finite() {
            return Err(invalid("numeric-aggregate-representation"));
        }
    }
    let quality = if weight > 0.0 {
        let value = sum / weight;
        out.quality = Some(Box::new(round2(value)?));
        Some(value)
    } else {
        None
    };
    let mut blocks = BTreeSet::new();
    for row in &rows {
        if rank(&row.gate).unwrap() > phase_rank {
            continue;
        }
        out.progress.applicable = out
            .progress
            .applicable
            .checked_add(1)
            .ok_or_else(|| invalid("progress-representation"))?;
        if row.status == "PASS" {
            out.progress.passed += 1
        }
        let blocked = if phase == "audit" {
            !matches!(row.status.as_str(), "PASS" | "FAIL")
                || (row.domain.is_empty() && row.mandatory && row.status != "PASS")
        } else {
            (row.mandatory && row.status != "PASS")
                || (row.status == "FAIL" && severity(&row.severity).unwrap() >= 2)
        };
        if blocked {
            blocks.insert(row.id.clone());
        }
        if row.status == "FAIL"
            && severity(&row.severity).unwrap() > severity(&out.max_severity).unwrap()
        {
            out.max_severity = row.severity.clone()
        }
    }
    if out.progress.applicable == 0 {
        blocks.insert("empty-applicable-inventory".into());
    } else {
        out.progress.percent = Some(Box::new(round2(
            100.0 * out.progress.passed as f64 / out.progress.applicable as f64,
        )?));
    }
    if phase == "delivery" {
        if out.provisional || quality.is_none() {
            blocks.insert("quality-incomplete".into());
        }
        if quality.is_some_and(|q| q < (policy.quality_delivery_min_percent.0 as f64)) {
            blocks.insert(
                if method == "2" {
                    "quality-below-70"
                } else {
                    "quality-below-policy"
                }
                .into(),
            );
        }
        if out
            .domain_scores
            .as_ref()
            .unwrap()
            .values()
            .any(|s| s.0 < (policy.quality_domain_min_percent.0 as f64))
        {
            blocks.insert(
                if method == "2" {
                    "domain-below-50"
                } else {
                    "domain-below-policy"
                }
                .into(),
            );
        }
    }
    out.quality_band = "INCOMPLETE".into();
    if let Some(q) = quality.filter(|_| !out.provisional) {
        out.quality_band = if q >= policy.quality_band_ok_min_percent.0 as f64 {
            "OK"
        } else if q >= policy.quality_band_warn_min_percent.0 as f64 {
            "WARN"
        } else if q >= policy.quality_band_blocked_min_percent.0 as f64 {
            "BLOQUÉ"
        } else {
            "STOP"
        }
        .into();
    }
    out.allowed = blocks.is_empty();
    out.ship_allowed = phase == "delivery" && out.allowed;
    out.blockers = Some(blocks.into_iter().collect());
    checks.sort_by_key(|&c| text(&d, c, "id"));
    let ca = d.push_array(checks);
    let mut config = BTreeMap::from([("domains".into(), domains_id), ("checks".into(), ca)]);
    if let Some(digest) = policy_digest {
        let id = d.push_text(digest)?;
        config.insert("quality_policy_digest".into(), id);
    }
    let config = d.push_object(config);
    out.config_digest = hash(&d.canonical(config)?);
    applicable.sort();
    let mut ids = Vec::new();
    for id in applicable {
        ids.push(d.push_text(&id)?);
    }
    let ids = d.push_array(ids);
    out.applicability_digest = hash(&d.canonical(ids)?);
    Ok(out)
}
