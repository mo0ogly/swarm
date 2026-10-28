use std::{fs, path::PathBuf};
use swarm_engine::{read_project_context, EngineSettingsValues};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("project-context-").unwrap());
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
#[test]
fn project_context_consumes_configured_caps_and_preserves_files_on_refusal_and_recovery() {
    let f = Fixture::new();
    let limits = EngineSettingsValues::installation_defaults()
        .unwrap()
        .context_policy_limits()
        .unwrap();
    assert!(read_project_context(&f.0, "reviewer", &limits)
        .unwrap()
        .is_none());
    fs::write(f.0.join("guide.md"), b"Project rules").unwrap();
    let profile=br#"{"version":1,"name":"Project","instructions":[{"path":"guide.md","roles":["reviewer","worker"]}]}"#;
    fs::write(f.0.join("swarm.project.json"), profile).unwrap();
    let observed = read_project_context(&f.0, "reviewer", &limits)
        .unwrap()
        .unwrap();
    assert_eq!(observed.context.name, "Project");
    assert!(observed.prompt.contains("Project rules"));
    assert_eq!(observed.context.sources.as_ref().unwrap().len(), 1);
    for which in ["name", "sources", "path", "document", "context"] {
        read_project_context(&f.0, "reviewer", &limits).unwrap();
        let mut small = limits.clone();
        match which {
            "name" => small.profile_name_bytes = Some(1),
            "sources" => small.profile_sources = Some(0),
            "path" => small.profile_path_bytes = Some(1),
            "document" => small.profile_document_bytes = Some(1),
            _ => small.context_bytes = Some(1),
        };
        assert!(read_project_context(&f.0, "reviewer", &small).is_err());
        assert_eq!(fs::read(f.0.join("swarm.project.json")).unwrap(), profile);
        assert_eq!(
            read_project_context(&f.0, "reviewer", &limits)
                .unwrap()
                .unwrap()
                .context
                .sha256,
            observed.context.sha256
        );
    }
    let mut caps = limits.clone();
    caps.context_bytes = Some(1);
    caps.worker_context_bytes = None;
    assert!(read_project_context(&f.0, "worker", &caps)
        .unwrap()
        .is_some());
    fs::write(
        f.0.join("guide.md"),
        b"API_KEY=abcdefghijklmnopqrstuvwxyz012345",
    )
    .unwrap();
    assert!(read_project_context(&f.0, "reviewer", &limits).is_err());
    fs::write(f.0.join("guide.md"), b"Project rules").unwrap();
    assert_eq!(
        read_project_context(&f.0, "reviewer", &limits)
            .unwrap()
            .unwrap()
            .context
            .sha256,
        observed.context.sha256
    );
    let mut bad: serde_json::Value = serde_json::from_slice(profile).unwrap();
    bad["instructions"][0]["path"] = "../outside.md".into();
    fs::write(
        f.0.join("swarm.project.json"),
        serde_json::to_vec(&bad).unwrap(),
    )
    .unwrap();
    assert!(read_project_context(&f.0, "reviewer", &limits).is_err());
    fs::write(f.0.join("swarm.project.json"), profile).unwrap();
    assert!(read_project_context(&f.0, "reviewer", &limits).is_ok());
}

#[test]
fn review_document_cap_and_managed_memory_alias_refusals_are_explicit() {
    let f = Fixture::new();
    let caps = EngineSettingsValues::installation_defaults()
        .unwrap()
        .context_policy_limits()
        .unwrap();
    fs::create_dir(f.0.join("docs")).unwrap();
    fs::write(f.0.join("docs/report.md"), b"Report evidence").unwrap();
    assert_eq!(
        swarm_engine::read_review_delivery(&f.0, "docs/report.md", &caps).unwrap(),
        "Report evidence"
    );
    let mut small = caps.clone();
    small.review_delivery_bytes = Some(1);
    assert!(swarm_engine::read_review_delivery(&f.0, "docs/report.md", &small).is_err());
    small.review_delivery_bytes = None;
    assert_eq!(
        swarm_engine::read_review_delivery(&f.0, "docs/report.md", &small).unwrap(),
        "Report evidence"
    );
    fs::create_dir_all(f.0.join(".claude/field-guide")).unwrap();
    let source = f.0.join(".claude/field-guide/lesson.md");
    fs::write(&source, b"Managed lesson").unwrap();
    for name in [
        ".claude/field-guide/lesson.md".to_string(),
        source.to_str().unwrap().to_string(),
    ] {
        let profile = serde_json::json!({"version":1,"name":"Memory","instructions":[{"path":name,"roles":["reviewer"]}]});
        fs::write(
            f.0.join("swarm.project.json"),
            serde_json::to_vec(&profile).unwrap(),
        )
        .unwrap();
        let error = read_project_context(&f.0, "reviewer", &caps)
            .err()
            .expect("managed memory must remain closed");
        assert_eq!(
            error.reason(),
            if PathBuf::from(&name).is_absolute() {
                "source-path"
            } else {
                "versioned-memory-source-guard-not-ported"
            },
            "{name}"
        );
    }
    #[cfg(unix)]
    {
        fs::write(f.0.join("ordinary.md"), b"Ordinary control").unwrap();
        std::os::unix::fs::symlink("ordinary.md", f.0.join("alias.md")).unwrap();
        let profile = serde_json::json!({"version":1,"name":"Alias","instructions":[{"path":"alias.md","roles":["reviewer"]}]});
        fs::write(
            f.0.join("swarm.project.json"),
            serde_json::to_vec(&profile).unwrap(),
        )
        .unwrap();
        read_project_context(&f.0, "reviewer", &caps).unwrap();
        fs::remove_file(f.0.join("alias.md")).unwrap();
        std::os::unix::fs::symlink(".claude/field-guide/lesson.md", f.0.join("alias.md")).unwrap();
        let error = read_project_context(&f.0, "reviewer", &caps).err().unwrap();
        assert_eq!(error.reason(), "versioned-memory-source-guard-not-ported");
    }
}
#[cfg(unix)]
#[test]
fn profile_sources_refuse_fifo_without_waiting_for_a_writer() {
    let f = Fixture::new();
    let caps = EngineSettingsValues::installation_defaults()
        .unwrap()
        .context_policy_limits()
        .unwrap();
    let profile=br#"{"version":1,"name":"Ordinary file control","instructions":[{"path":"pipe.md","roles":["reviewer"]}]}"#;
    fs::write(f.0.join("swarm.project.json"), profile).unwrap();
    fs::write(f.0.join("pipe.md"), b"Control").unwrap();
    read_project_context(&f.0, "reviewer", &caps).unwrap();
    fs::remove_file(f.0.join("pipe.md")).unwrap();
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        f.0.join("pipe.md"),
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    assert!(read_project_context(&f.0, "reviewer", &caps).is_err());
    fs::remove_file(f.0.join("pipe.md")).unwrap();
    fs::write(f.0.join("pipe.md"), b"Control").unwrap();
    read_project_context(&f.0, "reviewer", &caps).unwrap();
}

/// Staleness is a semantic mismatch after successful reads, never a catch-all
/// for malformed input or IO. Preserve causes and recover after restoring files.
#[test]
fn context_staleness_is_typed_and_does_not_hide_parse_failures() {
    use std::error::Error as _;
    let f = Fixture::new();
    let caps = EngineSettingsValues::installation_defaults()
        .unwrap()
        .context_policy_limits()
        .unwrap();
    swarm_engine::check_project_context_freshness(&f.0, None, "reviewer", &caps).unwrap();
    let old = swarm_engine::ProjectContext {
        sha256: "old-context".into(),
        ..Default::default()
    };
    let stale = swarm_engine::check_project_context_freshness(&f.0, Some(&old), "reviewer", &caps)
        .unwrap_err();
    assert!(stale.is_stale());
    assert!(stale.source().is_none());
    swarm_engine::check_project_context_freshness(&f.0, None, "reviewer", &caps).unwrap();
    fs::write(f.0.join("swarm.project.json"), b"?").unwrap();
    let malformed =
        swarm_engine::check_project_context_freshness(&f.0, None, "reviewer", &caps).unwrap_err();
    assert!(!malformed.is_stale());
    assert!(malformed.source().is_some());
    fs::remove_file(f.0.join("swarm.project.json")).unwrap();
    swarm_engine::check_project_context_freshness(&f.0, None, "reviewer", &caps).unwrap();
}
