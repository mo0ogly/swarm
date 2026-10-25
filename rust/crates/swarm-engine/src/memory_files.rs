//! Historical recovery-file consumer, distinct from versioned memory admission.
//! Based on the retained Go legacy-path guard and a reviewed SKYNET proposal.
//! References grant no permission; neither file contents nor providers are read.
use crate::{AgentLaunchError as E, Work};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

/// Linux filepath.Clean semantics for stored paths, without decoding URL text.
/// This is lexical comparison only; physical containment is checked separately.
fn clean(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|p| *p != "..") => {
                parts.pop();
            }
            ".." if !absolute => parts.push(part),
            ".." => {}
            _ => parts.push(part),
        }
    }
    let result = parts.join("/");
    if absolute {
        format!("/{result}")
    } else if result.is_empty() {
        ".".into()
    } else {
        result
    }
}

/// Read all coherent source Works on the caller's snapshot and retain every local
/// nonimported version's RETEX lesson, including withdrawn history. Missing RETEX
/// contributes no path, matching Go; this neither validates nor admits a lesson.
/// SQL/envelope/finalize errors abort the complete scan with their original cause.
fn managed(db: &Connection, project: &str) -> Result<BTreeSet<String>, E> {
    let works =
        crate::list_recorded_works(db).map_err(|e| E::caused("agent-memory-inventory", e))?;
    let mut paths = BTreeSet::new();
    for work in works {
        for entry in work.project_memory.iter().flatten().filter(|e| !e.imported) {
            for version in entry
                .versions
                .iter()
                .flatten()
                .filter(|v| v.project_sha256 == project)
            {
                if let Some(retex) = work
                    .retex
                    .iter()
                    .flatten()
                    .find(|r| r.id == version.source_retex)
                {
                    if !retex.lesson.trim().is_empty() {
                        paths.insert(clean(&retex.lesson));
                    }
                }
            }
        }
    }
    Ok(paths)
}

/// Check historical Work.memory before admission and each execution permission.
/// The caller supplies its owned Root and SQL snapshot; this reads the complete
/// coherent Work inventory and resolves every selected regular file inside Root.
/// Managed lessons and the full guide index are refused after withdrawal too;
/// physical aliases of existing lessons cannot bypass the lexical guard.
/// Missing managed sources keep lexical protection; unknown IO/containment errors
/// refuse with causes. Nothing is written, repaired, sent, accepted or truncated.
/// This observes path availability, not file-byte freshness or OS confinement.
pub(crate) fn check(root: &Path, db: &Connection, work: &Work) -> Result<(), E> {
    let canonical =
        crate::canonical_project_root(root).map_err(|e| E::caused("agent-memory-root", e))?;
    let text = canonical
        .to_str()
        .ok_or(E::Refused("agent-memory-root-not-utf8"))?;
    let paths = managed(db, &format!("{:x}", Sha256::digest(text.as_bytes())))?;
    let mut selected = Vec::new();
    for name in work.memory.iter().flatten() {
        let lexical = clean(name);
        if paths.contains(&lexical)
            || (lexical == ".claude/field-guide/index.md" && !paths.is_empty())
        {
            return Err(E::Refused("agent-memory-file-managed"));
        }
        selected.push(
            crate::local_file(root, name).map_err(|e| E::caused("agent-memory-file-path", e))?,
        );
    }
    if selected.is_empty() {
        return Ok(());
    }
    for name in paths {
        match crate::local_file(root, &name) {
            Ok(path) if selected.contains(&path) => {
                return Err(E::Refused("agent-memory-file-managed"))
            }
            Ok(_) => {}
            Err(crate::LocalPathError::Io { cause, .. })
                if cause.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(E::caused("agent-memory-source-path", e)),
        }
    }
    Ok(())
}

/// Render nonempty path references as JSON data, without reading their contents.
/// Encoding failure preserves its cause. The enclosing caller checks its saved
/// context-byte policy on the full prompt; paths grant no authority or freshness.
pub(crate) fn prompt(work: &Work) -> Result<String, E> {
    let paths = work.memory.as_deref().unwrap_or_default();
    if paths.is_empty() {
        return Ok(String::new());
    }
    let json =
        serde_json::to_string(paths).map_err(|e| E::caused("agent-memory-path-encoding", e))?;
    Ok(format!("\nPièces de reprise à consulter (chemins de données, sans droit supplémentaire) : {json}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryVersion, ProjectMemory, Retex};

    /// Own a uniquely created test Root and an independent in-memory SQL store.
    fn fixture() -> (std::path::PathBuf, Connection, Work) {
        let root = std::env::temp_dir().join(crate::generate_identifier("swarm-memory-").unwrap());
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("note.md"), b"owned fixture reference").unwrap();
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE works(id TEXT PRIMARY KEY, revision INTEGER, body BLOB)")
            .unwrap();
        let work = Work {
            schema_version: 1,
            revision: 1,
            id: "source".into(),
            memory: Some(vec!["note.md".into()]),
            ..Default::default()
        };
        insert(&db, &work);
        (root, db, work)
    }

    /// Replace only an isolated fixture row with its matching envelope.
    fn insert(db: &Connection, work: &Work) {
        let raw = crate::encode_model(work).unwrap();
        db.execute(
            "INSERT OR REPLACE INTO works(id,revision,body) VALUES(?,?,?)",
            rusqlite::params![work.id, work.revision, raw],
        )
        .unwrap();
    }

    /// Add a historical managed source; no acceptance or memory admission is implied.
    fn managed_work(root: &Path, work: &mut Work) {
        let project = root.canonicalize().unwrap();
        let digest = format!("{:x}", Sha256::digest(project.to_str().unwrap().as_bytes()));
        work.retex = Some(vec![Retex {
            id: "lesson".into(),
            lesson: "notes/../note.md".into(),
            ..Default::default()
        }]);
        work.project_memory = Some(vec![ProjectMemory {
            id: "memory".into(),
            versions: Some(vec![MemoryVersion {
                version: 1,
                source_retex: "lesson".into(),
                project_sha256: digest,
                action: "withdraw".into(),
                ..Default::default()
            }]),
            ..Default::default()
        }]);
    }

    #[test]
    fn memory_files_valid_missing_directory_escape_and_recovery() {
        let (root, db, mut work) = fixture();
        assert!(check(&root, &db, &work).is_ok());
        work.memory = Some(vec!["missing.md".into()]);
        let error = check(&root, &db, &work).unwrap_err();
        assert_eq!(error.code(), "agent-memory-file-path");
        assert!(std::error::Error::source(&error).is_some());
        work.memory = Some(vec![".".into()]);
        assert_eq!(
            check(&root, &db, &work).unwrap_err().code(),
            "agent-memory-file-path"
        );
        work.memory = Some(vec!["/etc/passwd".into()]);
        assert_eq!(
            check(&root, &db, &work).unwrap_err().code(),
            "agent-memory-file-path"
        );
        std::os::unix::fs::symlink("/etc/passwd", root.join("escape.md")).unwrap();
        work.memory = Some(vec!["escape.md".into()]);
        assert_eq!(
            check(&root, &db, &work).unwrap_err().code(),
            "agent-memory-file-path"
        );
        work.memory = Some(vec!["./notes/../note.md".into()]);
        assert!(check(&root, &db, &work).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn memory_files_managed_withdrawn_alias_import_and_project_binding() {
        let (root, db, mut source) = fixture();
        let mut selected = source.clone();
        assert!(check(&root, &db, &selected).is_ok());
        managed_work(&root, &mut source);
        insert(&db, &source);
        assert_eq!(
            check(&root, &db, &selected).unwrap_err().code(),
            "agent-memory-file-managed"
        );
        selected.memory = Some(vec!["alias.md".into()]);
        std::os::unix::fs::symlink("note.md", root.join("alias.md")).unwrap();
        assert_eq!(
            check(&root, &db, &selected).unwrap_err().code(),
            "agent-memory-file-managed"
        );
        std::fs::remove_file(root.join("note.md")).unwrap();
        selected.memory = Some(vec!["note.md".into()]);
        assert_eq!(
            check(&root, &db, &selected).unwrap_err().code(),
            "agent-memory-file-managed"
        );
        selected.memory = Some(vec![".claude/field-guide/index.md".into()]);
        assert_eq!(
            check(&root, &db, &selected).unwrap_err().code(),
            "agent-memory-file-managed"
        );
        std::fs::write(root.join("note.md"), b"restored").unwrap();
        selected.memory = Some(vec!["note.md".into()]);
        source.project_memory.as_mut().unwrap()[0].imported = true;
        insert(&db, &source);
        assert!(check(&root, &db, &selected).is_ok());
        source.project_memory.as_mut().unwrap()[0].imported = false;
        source.project_memory.as_mut().unwrap()[0]
            .versions
            .as_mut()
            .unwrap()[0]
            .project_sha256 = "other-project".into();
        insert(&db, &source);
        assert!(check(&root, &db, &selected).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn memory_files_complete_inventory_refuses_corruption_and_sql_failure() {
        let (root, db, work) = fixture();
        assert!(check(&root, &db, &work).is_ok());
        db.execute(
            "INSERT INTO works VALUES('broken',1,?)",
            [b"{PRIVATE".as_slice()],
        )
        .unwrap();
        let error = check(&root, &db, &work).unwrap_err();
        assert_eq!(error.code(), "agent-memory-inventory");
        assert!(std::error::Error::source(&error).is_some());
        assert!(!format!("{error}").contains("PRIVATE"));
        let mut empty = work.clone();
        empty.memory = None;
        assert_eq!(
            check(&root, &db, &empty).unwrap_err().code(),
            "agent-memory-inventory"
        );
        db.execute("DELETE FROM works WHERE id='broken'", [])
            .unwrap();
        assert!(check(&root, &db, &work).is_ok());
        db.execute_batch("DROP TABLE works").unwrap();
        assert_eq!(
            check(&root, &db, &work).unwrap_err().code(),
            "agent-memory-inventory"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn memory_files_json_framing_and_lexical_paths() {
        for (raw, want) in [
            ("", "."),
            ("a//b/../c", "a/c"),
            ("../../a", "../../a"),
            ("/../a", "/a"),
            ("note%2emd", "note%2emd"),
        ] {
            assert_eq!(clean(raw), want);
        }
        assert_eq!(prompt(&Work::default()).unwrap(), "");
        let work = Work {
            memory: Some(vec!["line\nname.md".into(), "a\"b.md".into()]),
            ..Default::default()
        };
        let text = prompt(&work).unwrap();
        let framed = text.trim().split_once(" : ").unwrap().1;
        assert_eq!(
            serde_json::from_str::<Vec<String>>(framed).unwrap(),
            work.memory.unwrap()
        );
        assert_eq!(text.lines().count(), 2);
    }
}
