#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
//! Read-only current Linux namespace metadata, never launch or protection authority.
//! No subprocess/NSS lookup, Root write, account, mapping, retry or policy change.
use serde::Serialize;
use std::{
    env,
    error::Error as StdError,
    ffi::OsString,
    fmt, fs, io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

/// Safe diagnostic plus original IO cause. JSON and ordinary formatting omit
/// private source text; callers may inspect the retained source deliberately.
#[derive(Serialize)]
pub struct Failure {
    pub code: &'static str,
    pub errno: Option<i32>,
    #[serde(skip)]
    pub cause: io::Error,
}
impl Failure {
    /// Preserve original IO and actual optional errno, never manufacture errno0.
    fn io(code: &'static str, cause: io::Error) -> Self {
        Self {
            code,
            errno: cause.raw_os_error(),
            cause,
        }
    }
}
impl fmt::Display for Failure {
    /// Static category only; underlying paths/text remain typed private data.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Debug for Failure {
    /// Same redaction as Display; does not print raw metadata or environment.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Failure {
    /// Retain the original cause for deliberate local diagnostics, not JSON logs.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// An unavailable or invalid source is never an observed empty assignment.
#[derive(Serialize, Debug)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Observation<T> {
    Observed { value: T },
    Unavailable { failure: Failure },
    Invalid { code: &'static str },
}
impl<T> Observation<T> {
    /// Borrow only a successfully parsed observation; grants no admission.
    fn value(&self) -> Option<&T> {
        if let Self::Observed { value } = self {
            Some(value)
        } else {
            None
        }
    }
}
/// Namespace-local kernel identity; neither host identity nor a human account.
#[derive(Serialize, Debug)]
pub struct Identity {
    pub euid: u32,
    pub egid: u32,
    pub groups: Vec<u32>,
}
/// Observed mapping interval, not a desired mapping or an owned namespace handle.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct Mapping {
    pub inside: u32,
    pub outside: u32,
    pub count: u32,
}
/// Explicit local assignment only; no exclusive lease or identity is selected.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct Range {
    pub start: u32,
    pub count: u32,
}
/// Racy metadata for the first executable-looking PATH candidate. ACLs, file
/// capabilities, held executable identity and successful execution are not proven.
#[derive(Serialize, Debug)]
pub struct Helper {
    pub path: String,
    pub owner: u32,
    pub mode: u32,
    pub metadata_safe: bool,
}
/// Fixed protocol names, not caller-selected filesystem inputs.
#[derive(Serialize, Debug)]
pub struct Maps {
    pub uid_map: Observation<Vec<Mapping>>,
    pub gid_map: Observation<Vec<Mapping>>,
}
/// Only the current local username/numeric UID assignments are disclosed.
#[derive(Serialize, Debug)]
pub struct Ranges {
    pub subuid: Observation<Vec<Range>>,
    pub subgid: Observation<Vec<Range>>,
}
/// Mapping helper metadata never substitutes for actual qualified UID separation.
#[derive(Serialize, Debug)]
pub struct Helpers {
    pub newuidmap: Observation<Helper>,
    pub newgidmap: Observation<Helper>,
}
/// Version2 native observation; distinct from historical Python version1 shape.
/// All protection booleans remain false even when no metadata prerequisite is
/// missing. CLI/server observe their own process namespace, not a future agent.
#[derive(Serialize, Debug)]
pub struct Report {
    pub schema_version: u32,
    pub kind: &'static str,
    pub scope: &'static str,
    pub identity: Observation<Identity>,
    pub local_username: Observation<String>,
    pub maps: Maps,
    pub assigned_ranges: Ranges,
    pub mapping_helpers: Helpers,
    pub missing_prerequisites: Vec<&'static str>,
    pub read_only: bool,
    pub launch_verified: bool,
    pub separate_host_uid_verified: bool,
    pub confinement_verified: bool,
    pub human_verified: bool,
}
/// Actual source locations and PATH snapshot. Private fixture injection only;
/// production supplies Linux protocol paths, never request-controlled paths.
struct Sources {
    passwd: PathBuf,
    subuid: PathBuf,
    subgid: PathBuf,
    uid_map: PathBuf,
    gid_map: PathBuf,
    path: Option<OsString>,
}
/// Parse ASCII decimal only; all-ones UID is a Linux invalid representation.
/// This bound is representation, never an operator allocation ceiling.
fn number(value: &str) -> Result<u32, &'static str> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("metadata-decimal");
    }
    let n = value.parse::<u32>().map_err(|_| "metadata-decimal")?;
    if n == u32::MAX {
        return Err("metadata-uid-sentinel");
    }
    Ok(n)
}
/// Nonzero interval lengths may cover the full valid UID extent. Endpoints use
/// widened arithmetic to reject overflow without truncation or hidden policy.
fn count(value: &str, start: u32) -> Result<u32, &'static str> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("metadata-count");
    }
    let n = value.parse::<u32>().map_err(|_| "metadata-count")?;
    if n == 0 || u64::from(start) + u64::from(n) > u64::from(u32::MAX) {
        return Err("metadata-extent");
    }
    Ok(n)
}
/// Decode actual three-column kernel maps, including the initial full-range map.
/// Empty/malformed/overflowing records refuse; no first packet/domain is adopted.
fn mappings(text: &str) -> Result<Vec<Mapping>, &'static str> {
    let mut out = Vec::new();
    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 3 {
            return Err("metadata-map");
        }
        let inside = number(fields[0])?;
        let outside = number(fields[1])?;
        let n = count(fields[2], inside)?;
        count(fields[2], outside)?;
        out.push(Mapping {
            inside,
            outside,
            count: n,
        });
    }
    if out.is_empty() {
        return Err("metadata-empty-map");
    }
    Ok(out)
}
/// Retain only explicit current-user ranges. Malformed own assignments, zero
/// starts/counts, overlap and overflow refuse; unrelated records are not exposed.
fn ranges(text: &str, name: Option<&str>, uid: u32) -> Result<Vec<Range>, &'static str> {
    let numeric = uid.to_string();
    let mut out: Vec<Range> = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split(':').collect();
        if fields[0] != numeric && Some(fields[0]) != name {
            continue;
        }
        if fields.len() != 3 {
            return Err("metadata-own-range");
        }
        let start = number(fields[1])?;
        let n = count(fields[2], start)?;
        if start == 0 {
            return Err("metadata-own-range");
        }
        if out.iter().any(|r| {
            u64::from(start) < u64::from(r.start) + u64::from(r.count)
                && u64::from(r.start) < u64::from(start) + u64::from(n)
        }) {
            return Err("metadata-overlapping-range");
        }
        out.push(Range { start, count: n });
    }
    Ok(out)
}
/// Resolve exactly one local passwd name for current UID, without NSS/network
/// lookup. Missing/ambiguous/malformed own record stays an explicit refusal.
fn username(text: &str, uid: u32) -> Result<String, &'static str> {
    let mut found = None;
    for line in text.lines() {
        let f: Vec<_> = line.split(':').collect();
        if f.len() < 3 || number(f[2]).ok() != Some(uid) {
            continue;
        }
        if f.len() != 7 || f[0].is_empty() || found.is_some() {
            return Err("metadata-local-user");
        }
        found = Some(f[0].to_owned());
    }
    found.ok_or("metadata-local-user-missing")
}
/// Read one known metadata source, keeping original IO separate from format
/// refusal. No source content, fallback, retry or mutation on failure.
fn parsed<T>(path: &Path, parse: impl FnOnce(&str) -> Result<T, &'static str>) -> Observation<T> {
    match fs::read_to_string(path) {
        Err(cause) => Observation::Unavailable {
            failure: Failure::io("metadata-read", cause),
        },
        Ok(text) => match parse(&text) {
            Ok(value) => Observation::Observed { value },
            Err(code) => Observation::Invalid { code },
        },
    }
}
/// Read candidate and ancestors only. Metadata safety means root-owned setuid
/// regular executable-looking file and root-owned non-group/other-writable path;
/// it does not prove ACL access, file capabilities, stable custody or execution.
fn helper_metadata(path: &Path) -> io::Result<Helper> {
    let real = fs::canonicalize(path)?;
    let m = fs::metadata(&real)?;
    let mut safe = m.is_file()
        && m.uid() == 0
        && m.mode() & 0o4000 != 0
        && m.mode() & 0o111 != 0
        && m.mode() & 0o022 == 0;
    for p in real.ancestors().skip(1) {
        let m = fs::metadata(p)?;
        safe &= m.is_dir() && m.uid() == 0 && m.mode() & 0o022 == 0;
    }
    let path = real
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "helper-path-encoding"))?
        .to_owned();
    Ok(Helper {
        path,
        owner: m.uid(),
        mode: m.mode() & 0o7777,
        metadata_safe: safe,
    })
}
/// Search the actual PATH snapshot without executing a helper. A read error
/// other than absent candidate refuses instead of skipping an unknown entry.
/// Missing PATH stays invalid; no guessed system directory fallback is added.
fn helper(path: Option<&OsString>, name: &str) -> Observation<Helper> {
    let Some(path) = path else {
        return Observation::Invalid {
            code: "helper-path-unset",
        };
    };
    let mut missing = None;
    for directory in env::split_paths(path) {
        let p = directory.join(name);
        match fs::metadata(&p) {
            Ok(m) => {
                if !m.is_file() || m.mode() & 0o111 == 0 {
                    continue;
                }
                return match helper_metadata(&p) {
                    Ok(value) => Observation::Observed { value },
                    Err(cause) => Observation::Unavailable {
                        failure: Failure::io("helper-metadata", cause),
                    },
                };
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => missing = Some(e),
            Err(cause) => {
                return Observation::Unavailable {
                    failure: Failure::io("helper-search", cause),
                }
            }
        }
    }
    Observation::Unavailable {
        failure: Failure::io(
            "helper-not-found",
            missing.unwrap_or_else(|| io::Error::new(io::ErrorKind::NotFound, "helper-not-found")),
        ),
    }
}
/// Compose all independent observations even on individual failure. Missing
/// metadata is actionable, never successful empty data or an admission verdict.
fn collect(s: Sources, uid: u32, gid: u32, groups: io::Result<Vec<u32>>) -> Report {
    let local_username = parsed(&s.passwd, |text| username(text, uid));
    let name = local_username.value().map(String::as_str);
    let assigned_ranges = Ranges {
        subuid: parsed(&s.subuid, |text| ranges(text, name, uid)),
        subgid: parsed(&s.subgid, |text| ranges(text, name, uid)),
    };
    let maps = Maps {
        uid_map: parsed(&s.uid_map, mappings),
        gid_map: parsed(&s.gid_map, mappings),
    };
    let mapping_helpers = Helpers {
        newuidmap: helper(s.path.as_ref(), "newuidmap"),
        newgidmap: helper(s.path.as_ref(), "newgidmap"),
    };
    let identity = match groups {
        Ok(groups) => Observation::Observed {
            value: Identity {
                euid: uid,
                egid: gid,
                groups,
            },
        },
        Err(cause) => Observation::Unavailable {
            failure: Failure::io("identity-groups", cause),
        },
    };
    let mut missing_prerequisites = Vec::new();
    if identity.value().is_none() {
        missing_prerequisites.push("identity:groups")
    }
    if local_username.value().is_none() {
        missing_prerequisites.push("identity:local-passwd")
    }
    for (name, helper) in [
        ("helper:newuidmap", &mapping_helpers.newuidmap),
        ("helper:newgidmap", &mapping_helpers.newgidmap),
    ] {
        if !helper.value().is_some_and(|h| h.metadata_safe) {
            missing_prerequisites.push(name)
        }
    }
    for (name, range) in [
        ("range:subuid", &assigned_ranges.subuid),
        ("range:subgid", &assigned_ranges.subgid),
    ] {
        if !range.value().is_some_and(|r| !r.is_empty()) {
            missing_prerequisites.push(name)
        }
    }
    for (name, map) in [
        ("map:uid_map", &maps.uid_map),
        ("map:gid_map", &maps.gid_map),
    ] {
        if map.value().is_none() {
            missing_prerequisites.push(name)
        }
    }
    Report {
        schema_version: 2,
        kind: "paea-linux-prerequisite-observation",
        scope: "current-process-namespace",
        identity,
        local_username,
        maps,
        assigned_ranges,
        mapping_helpers,
        missing_prerequisites,
        read_only: true,
        launch_verified: false,
        separate_host_uid_verified: false,
        confinement_verified: false,
        human_verified: false,
    }
}
/// Observe this process's Linux identity, local passwd/ranges/maps and PATH helper
/// metadata. Never run a helper/NSS lookup, map UID/GID, mutate Root/host, select an
/// identity or claim human/confinement/launch proof. Independent IO/format errors
/// remain in the report with typed original causes and safe JSON code/errno.
/// Observation races and namespace-local values are unsuitable as launch approval;
/// authoritative admission must use owned handles and actual verified effects.
pub fn observe() -> Report {
    let groups = nix::unistd::getgroups()
        .map(|g| g.into_iter().map(|v| v.as_raw()).collect())
        .map_err(|e| io::Error::from_raw_os_error(e as i32));
    collect(
        Sources {
            passwd: "/etc/passwd".into(),
            subuid: "/etc/subuid".into(),
            subgid: "/etc/subgid".into(),
            uid_map: "/proc/self/uid_map".into(),
            gid_map: "/proc/self/gid_map".into(),
            path: env::var_os("PATH"),
        },
        nix::unistd::geteuid().as_raw(),
        nix::unistd::getegid().as_raw(),
        groups,
    )
}
#[cfg(test)]
mod tests;
