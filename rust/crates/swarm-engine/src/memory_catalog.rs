//! Versioned-memory public catalog ported from the retained Go
//! `internal/engine/project_memory_public.go` `projectMemoryCatalog` and
//! `internal/engine/dialogue_context.go` `foldDialogue`. Pure projection over a
//! caller-owned coherent Work inventory plus the active-use inventory: one
//! entry per stored version with its current status/reason, literal query
//! filtering over the folded body+conditions, and the active attempts holding
//! each exact reference. No admission, no authority, no Work mutation and no
//! empty-catalogue fallback on a read error (an inventory refusal stays an
//! error, never a valid empty result).

use crate::models::{MemoryVersion, Work};
use rusqlite::Connection;

/// One catalog line: the exact reference, the stored version, its availability
/// status with a stable French reason, the Work revision/title at read time and
/// the active attempts currently bound to this exact reference. Mirrors the Go
/// map shape (keys kept identical for cross-runtime comparability).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MemoryCatalogEntry {
    #[serde(rename = "ref")]
    pub reference: crate::MemoryReference,
    pub version: MemoryVersion,
    /// "historical", "active" or "unavailable".
    pub status: String,
    /// Stable diagnostic; "historical"/"active" carry Go's default reasons.
    pub reason: String,
    pub work_revision: i64,
    pub source_title: String,
    pub used_by: Vec<crate::MemoryUse>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MemoryCatalog {
    pub entries: Vec<MemoryCatalogEntry>,
    /// Always "literal": the only supported matching method.
    pub method: String,
    /// Trimmed query actually applied (empty = unfiltered).
    pub query: String,
}

/// Go `foldDialogue` (dialogue_context.go:97): `strings.ToLower` first, then
/// the fixed French accented table. The order matters — an uppercase accented
/// letter folds twice (É → é → e). Sequential `replace` is equivalent to Go's
/// single-pass Replacer here because every replacement output is ASCII no
/// later rule can re-match. Pure and total over `&str`.
pub(crate) fn fold_dialogue(s: &str) -> String {
    s.to_lowercase()
        .replace('é', "e")
        .replace('è', "e")
        .replace('ê', "e")
        .replace('ë', "e")
        .replace('à', "a")
        .replace('â', "a")
        .replace('ä', "a")
        .replace('î', "i")
        .replace('ï', "i")
        .replace('ô', "o")
        .replace('ö', "o")
        .replace('ù', "u")
        .replace('û', "u")
        .replace('ü', "u")
        .replace('ç', "c")
        .replace('œ', "oe")
}

/// Go `projectMemoryCatalog`: one entry per stored version of every Work in
/// the caller-owned inventory. The last version of an entry is "active" with
/// its reason "Version courante ; conditions à revérifier", downgraded to
/// "unavailable" with the typed refusal reason when full validation fails;
/// earlier versions are "historical" (non selectable). The literal query
/// filters over the folded body+conditions of each version. Validation runs
/// the same `validate_memory_version` guard as selection/resolution, so a
/// version never reads as available when a real consumer would refuse it.
/// The inventory and the use list come precomputed: this function performs no
/// I/O of its own and never converts a caller read error into an empty result.
pub fn memory_catalog(
    works: &[Work],
    uses: &[crate::MemoryUse],
    project_hash: &str,
    verified_source: &dyn Fn(&Work, &crate::Retex) -> Result<(), crate::project_memory::MemoryError>,
    query: &str,
) -> MemoryCatalog {
    // Go folds the trimmed query before filtering AND returns it folded.
    let query = fold_dialogue(query.trim());
    let mut entries = Vec::new();
    for work in works {
        for entry in work.project_memory.iter().flatten() {
            let versions = entry.versions.as_deref().unwrap_or_default();
            for (index, version) in versions.iter().enumerate() {
                let (status, reason) = if index + 1 == versions.len() {
                    match crate::project_memory::validate_memory_version(
                        work,
                        entry,
                        version,
                        project_hash,
                        verified_source,
                    ) {
                        Ok(()) => ("active", "Version courante ; conditions à revérifier"),
                        Err(error) => ("unavailable", &*go_error_text(&error)),
                    }
                } else {
                    ("historical", "Version historique, non sélectionnable")
                };
                if !query.is_empty()
                    && !fold_dialogue(&format!("{}\n{}", version.body, version.conditions))
                        .contains(&query)
                {
                    continue;
                }
                let reference = crate::MemoryReference {
                    work: work.id.clone(),
                    id: entry.id.clone(),
                    version: version.version,
                    sha256: version.sha256.clone(),
                };
                let used_by = uses
                    .iter()
                    .filter(|use_| use_.reference == reference)
                    .cloned()
                    .collect();
                entries.push(MemoryCatalogEntry {
                    reference,
                    version: version.clone(),
                    status: status.to_owned(),
                    reason: reason.to_owned(),
                    work_revision: work.revision,
                    source_title: work.title.clone(),
                    used_by,
                });
            }
        }
    }
    MemoryCatalog {
        entries,
        method: "literal".to_owned(),
        query,
    }
}

/// Exact Go error text for a version-validation failure: the catalog reason is
/// Go's `e.Error()`, so each mapped refusal code carries the retained Go
/// message (typographic apostrophes included). Codes outside the
/// `validate_memory_version` surface cannot arise here in practice (row-level
/// incoherence refuses the whole inventory upstream, like Go's `memoryWorks` /
/// `memoryUses`); the fallback keeps the stable code visible rather than
/// inventing a message.
fn go_error_text(error: &crate::project_memory::MemoryError) -> String {
    match error {
        crate::project_memory::MemoryError::Refused(code) => match *code {
            "project-memory-imported-or-foreign" => {
                "Mémoire importée ou d’un autre projet : admission locale requise".to_owned()
            }
            "project-memory-version-altered" => "Version mémoire altérée".to_owned(),
            "project-memory-version-withdrawn" => "Version mémoire retirée".to_owned(),
            "project-memory-source-changed" => {
                "Source RETEX modifiée : réexaminer la mémoire".to_owned()
            }
            "retex-missing" => "RETEX inconnu".to_owned(),
            "project-memory-source-unfresh" => {
                "Un fait sourcé et une tâche APEX acceptée avec des preuves fraîches sont requis."
                    .to_owned()
            }
            "project-memory-acceptance-source-changed" => {
                "Source de recette absente ou modifiée".to_owned()
            }
            "dialogue-exchange-missing" => {
                "Échange absent ou modifié dans ce travail : refaire la sélection.".to_owned()
            }
            other => format!("project-memory-refused: {}", other),
        },
        other => format!("{}", other),
    }
}

/// Read-only catalog over one opened Root: the full Work inventory (rowid
/// descending) and the active-use inventory come from the same connection, so
/// one consistent reader boundary covers both; any read/decode failure refuses
/// instead of publishing a partial collection. The caller owns Root open/close
/// and binding; this function performs no settings read, reservation or write.
pub fn memory_catalog_from_connection(
    connection: &Connection,
    root: &std::path::Path,
    lifecycle: &crate::TaskLifecycleContext<'_>,
    settings: Option<&crate::EngineSettingsDocument>,
    query: &str,
) -> Result<MemoryCatalog, crate::project_memory::MemoryError> {
    let works = crate::list_recorded_works(connection).map_err(|e| match e {
        crate::recorded_work::RecordedWorkError::Sql(sql) => {
            crate::project_memory::MemoryError::Sql(sql)
        }
        // A decode/identity failure on one row: typed WorkError cause retained.
        crate::recorded_work::RecordedWorkError::Work(work_error) => {
            crate::project_memory::MemoryError::Row(work_error)
        }
        crate::recorded_work::RecordedWorkError::Missing => {
            crate::project_memory::MemoryError::Missing
        }
        crate::recorded_work::RecordedWorkError::Finalize { .. } => {
            // The inventory was read but reader cleanup failed; no partial
            // catalog is published. WorkError keeps no source chain, so the
            // storage-cleanup cause is dropped here — RootStoreError never
            // wraps this through a path that would surface it. Refusal, not
            // an empty catalog.
            crate::project_memory::MemoryError::Row(crate::WorkError::Incoherent)
        }
    })?;
    let uses = crate::project_memory::memory_uses(connection)?;
    let project_hash = crate::agent_launch_preflight::hash(root.to_string_lossy().as_bytes());
    let verified_source = |work: &Work, retex: &crate::Retex| {
        crate::project_memory::verified_memory_source(work, retex, lifecycle, settings)
    };
    Ok(memory_catalog(&works, &uses, &project_hash, &verified_source, query))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MemoryReference, ProjectMemory, Retex};

    const PROJECT: &str = "project-hash-fixture";

    fn retex(id: &str) -> Retex {
        Retex {
            id: id.to_owned(),
            status: "verifie".to_owned(),
            nature: "fait".to_owned(),
            problem: "problème".to_owned(),
            proposal: "proposition".to_owned(),
            criteria: "critère".to_owned(),
            proof: "preuve".to_owned(),
            task: "task-1".to_owned(),
            ..Default::default()
        }
    }

    fn version(version: i64, body: &str) -> MemoryVersion {
        MemoryVersion {
            version,
            action: if version == 1 { "admit" } else { "replace" }.to_owned(),
            kind: "procedure".to_owned(),
            body: body.to_owned(),
            source_retex: "retex-1".to_owned(),
            project_sha256: PROJECT.to_owned(),
            ..Default::default()
        }
    }

    fn sealed(mut v: MemoryVersion) -> MemoryVersion {
        // The fixture RETEX must validate like the real cycle: the version's
        // source digest binds the work's own retex record at fixture time.
        let work = Work {
            schema_version: 1,
            revision: 1,
            id: "work-1".to_owned(),
            retex: Some(vec![retex("retex-1")]),
            ..Default::default()
        };
        let retex = retex("retex-1");
        v.source_sha256 = crate::project_memory::memory_source_sha(&work, &retex)
            .expect("fixture source digest");
        v.sha256 = crate::project_memory::memory_version_sha(&v).unwrap();
        v
    }

    fn noop() -> impl Fn(&Work, &Retex) -> Result<(), crate::project_memory::MemoryError> {
        |_work, _retex| Ok(())
    }

    #[test]
    fn fold_dialogue_matches_go_replacement_table() {
        // ToLower first, then the table: uppercase accented letters fold twice.
        assert_eq!(fold_dialogue("Vérifier ÉLÉMENT à droite"), "verifier element a droite");
        assert_eq!(fold_dialogue("œuf ïs Île"), "oeuf is ile");
        assert_eq!(fold_dialogue("çà où Ü"), "ca ou u");
        // Only the fixed table folds; other characters pass through lowercased.
        assert_eq!(fold_dialogue("ß LIGATURE ﬁ"), "ß ligature ﬁ");
        assert_eq!(fold_dialogue(""), "");
    }

    #[test]
    fn catalog_reports_active_historical_and_filtered_entries() {
        let mut work = Work {
            schema_version: 1,
            revision: 3,
            id: "work-1".to_owned(),
            title: "Mémoire conservée".to_owned(),
            retex: Some(vec![retex("retex-1")]),
            ..Default::default()
        };
        // Two versions: v1 historical, v2 active. Active validates under noop.
        work.project_memory = Some(vec![ProjectMemory {
            id: "lesson".to_owned(),
            imported: false,
            versions: Some(vec![
                sealed(version(1, "Première version vérifiée Étape")),
                sealed(version(2, "Version courante revalidée à jour")),
            ]),
        }]);
        let uses = vec![crate::MemoryUse {
            agent: "agent-1".to_owned(),
            work: "work-1".to_owned(),
            task: "task-9".to_owned(),
            status: "running".to_owned(),
            reference: MemoryReference {
                work: "work-1".to_owned(),
                id: "lesson".to_owned(),
                version: 2,
                sha256: work.project_memory.as_ref().unwrap()[0]
                    .versions
                    .as_ref()
                    .unwrap()[1]
                    .sha256
                    .clone(),
            },
        }];
        let catalog = memory_catalog(&[work.clone()], &uses, PROJECT, &noop(), "");
        assert_eq!(catalog.method, "literal");
        assert_eq!(catalog.query, "");
        assert_eq!(catalog.entries.len(), 2);
        let historical = &catalog.entries[0];
        assert_eq!(historical.status, "historical");
        assert_eq!(historical.reason, "Version historique, non sélectionnable");
        assert!(historical.used_by.is_empty());
        assert_eq!(historical.work_revision, 3);
        assert_eq!(historical.source_title, "Mémoire conservée");
        let active = &catalog.entries[1];
        assert_eq!(active.status, "active");
        assert_eq!(active.reason, "Version courante ; conditions à revérifier");
        assert_eq!(active.used_by.len(), 1);
        assert_eq!(active.used_by[0].agent, "agent-1");
        // Literal query folds accents and matches body or conditions only.
        let filtered = memory_catalog(&[work.clone()], &uses, PROJECT, &noop(), "  COURANTE ");
        assert_eq!(filtered.entries.len(), 1);
        assert_eq!(filtered.entries[0].reference.version, 2);
        let accented = memory_catalog(&[work], &uses, PROJECT, &noop(), "étape");
        assert_eq!(accented.entries.len(), 1);
        assert_eq!(accented.entries[0].reference.version, 1);
        // The returned query is the folded trimmed one, as in Go.
        let echoed = memory_catalog(&[], &[], PROJECT, &noop(), "  ÉTape ");
        assert_eq!(echoed.query, "etape");
    }

    #[test]
    fn catalog_downgrades_unavailable_active_with_typed_reason() {
        let mut work = Work {
            schema_version: 1,
            revision: 1,
            id: "work-1".to_owned(),
            retex: Some(vec![retex("retex-1")]),
            ..Default::default()
        };
        let mut v = version(1, "version active");
        v.action = "withdraw".to_owned();
        v.sha256 = crate::project_memory::memory_version_sha(&v).unwrap();
        work.project_memory = Some(vec![ProjectMemory {
            id: "lesson".to_owned(),
            imported: false,
            versions: Some(vec![v]),
        }]);
        let catalog = memory_catalog(&[work], &[], PROJECT, &noop(), "");
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].status, "unavailable");
        assert_eq!(catalog.entries[0].reason, "Version mémoire retirée");
    }

    #[test]
    fn catalog_marks_imported_entry_unavailable_not_historical() {
        let mut work = Work {
            schema_version: 1,
            revision: 1,
            id: "work-1".to_owned(),
            retex: Some(vec![retex("retex-1")]),
            ..Default::default()
        };
        let v = sealed(version(1, "imported elsewhere"));
        work.project_memory = Some(vec![ProjectMemory {
            id: "lesson".to_owned(),
            imported: true,
            versions: Some(vec![v]),
        }]);
        let catalog = memory_catalog(&[work], &[], PROJECT, &noop(), "");
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].status, "unavailable");
        assert_eq!(catalog.entries[0].reason, "Mémoire importée ou d’un autre projet : admission locale requise");
    }

    #[test]
    fn catalog_from_connection_refuses_instead_of_partial_catalog() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER,body BLOB);
                 CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,status TEXT,desired TEXT,body BLOB);",
            )
            .unwrap();
        // A valid row precedes an invalid row: a partial catalog must not pass.
        let valid = serde_json::to_vec(&crate::Work {
            schema_version: 1,
            revision: 1,
            id: "source".to_owned(),
            ..Default::default()
        })
        .unwrap();
        connection
            .execute(
                "INSERT INTO works(id,revision,body) VALUES('source',1,?1)",
                rusqlite::params![valid],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO works(id,revision,body) VALUES('broken',1,?1)",
                rusqlite::params![&b"{\"schema_version\":1,\"id\":\"broken\","[..]],
            )
            .unwrap();
        let lifecycle = crate::TaskLifecycleContext {
            root: std::path::Path::new(PROJECT),
            quality_policy: None,
            task_limits: &crate::TaskDefinitionLimits::legacy_go(),
            validation_limits: &crate::ValidationPolicyLimits::legacy_go(),
        };
        let error = memory_catalog_from_connection(
            &connection,
            std::path::Path::new(PROJECT),
            &lifecycle,
            None,
            "",
        )
        .unwrap_err();
        // The stored invalid row is retained exactly; refusal, never a catalog.
        let retained: Vec<u8> = connection
            .query_row(
                "SELECT body FROM works WHERE id='broken'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            retained,
            b"{\"schema_version\":1,\"id\":\"broken\",".to_vec()
        );
        assert!(matches!(
            error,
            crate::project_memory::MemoryError::Row(_)
        ));
    }
}
