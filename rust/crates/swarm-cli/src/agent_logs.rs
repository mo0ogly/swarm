//! Historical log command. Data observation grants no execution authority.
use super::{json_bytes, write_output, Arguments, CliError};
use std::{
    fs::OpenOptions, io::Write, os::fd::IntoRawFd, os::unix::fs::OpenOptionsExt, path::Path,
};

/// Validate before opening Root; read exact historical rows and close Root before
/// any output publication. Default output is a JSON array even without --json.
/// An output file is exclusive 0600 NDJSON, never an overwrite. Invalid cursors,
/// stored data, association, close, encoding, write and flush failures propagate.
/// File errors preserve uncertain publication; no DB mutation/provider launch.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if !(a.pos.len() == 3 || a.pos.len() == 4)
        || !a.input.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let after = match a.pos.get(3) {
        Some(raw) => raw.parse::<i64>().map_err(|_| CliError::Usage)?,
        None => 0,
    };
    let store = swarm_engine::RootStore::open(Path::new(&a.root), false)
        .map_err(|e| CliError::Root(Box::new(e)))?;
    let rows = store
        .historical_agent_logs(&a.pos[2], after)
        .map_err(|e| CliError::Root(Box::new(e)));
    let rows = match (rows, store.close()) {
        (Ok(rows), Ok(())) => rows,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(error)) => return Err(CliError::Root(Box::new(error))),
        (Err(primary), Err(cleanup)) => {
            return Err(CliError::RootCleanup {
                primary: Box::new(primary),
                cleanup: Box::new(cleanup),
            })
        }
    };
    if a.output.is_empty() {
        write_output(out, &json_bytes(&rows).map_err(CliError::Output)?)?;
    } else {
        let mut bytes = Vec::new();
        for row in rows {
            bytes.extend(
                swarm_engine::encode_public_model(&row)
                    .map_err(std::io::Error::other)
                    .map_err(CliError::Output)?,
            );
            bytes.push(b'\n');
        }
        publish_file(Path::new(&a.output), &bytes, a.language == "en")?;
    }
    Ok(0)
}

/// Create once with private Unix permissions; write/sync then explicitly close.
/// Bytes are fully encoded before this call. Both primary and close errors are
/// retained without retrying close or deleting a possibly partial output. An
/// opened handle does not prove complete publication or filesystem durability
/// after a directory crash; the destination's parent directory is not synced.
fn publish_file(path: &Path, bytes: &[u8], english: bool) -> Result<(), CliError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| CliError::OutputFile {
            primary: Some(Box::new(CliError::Output(e))),
            close: None,
            opened: false,
            english,
        })?;
    let primary = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .err()
        .map(|e| Box::new(CliError::Output(e)));
    let close = nix::unistd::close(file.into_raw_fd()).err().map(|e| {
        Box::new(CliError::Output(std::io::Error::from_raw_os_error(
            e as i32,
        )))
    });
    if primary.is_some() || close.is_some() {
        Err(CliError::OutputFile {
            primary,
            close,
            opened: true,
            english,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Public safe diagnostics must retain both IO branches, including a custom
    /// IO wrapper; source() alone loses the independent close failure.
    #[test]
    fn agent_logs_public_secondary_close_cause() {
        let error = CliError::OutputFile {
            primary: Some(Box::new(CliError::Output(
                std::io::Error::from_raw_os_error(28),
            ))),
            close: Some(Box::new(CliError::Output(
                std::io::Error::from_raw_os_error(9),
            ))),
            opened: true,
            english: true,
        };
        let nested = CliError::Input(std::io::Error::other(error));
        let mut diagnostic = Vec::new();
        let outcome = super::super::report(nested, true, true, &mut diagnostic);
        assert_ne!(outcome.code, 0);
        let json: serde_json::Value = serde_json::from_slice(&diagnostic).unwrap();
        let text = json.to_string();
        assert!(text.contains("os-errno:28"), "{text}");
        assert!(text.contains("os-errno:9"), "{text}");
    }
    /// Exclusive publication preserves prior bytes; empty history is a zero-byte
    /// file and failures preserve both typed branches without exposing IO text.
    #[test]
    fn agent_logs_export_and_cause_matrix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logs");
        publish_file(&path, b"original", false).unwrap_or_else(|e| panic!("{e}"));
        assert!(publish_file(&path, b"replace", true).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        let empty = dir.path().join("empty");
        publish_file(&empty, b"", true).unwrap_or_else(|e| panic!("{e}"));
        assert!(std::fs::read(empty).unwrap().is_empty());
        for english in [false, true] {
            for opened in [false, true] {
                let error = CliError::OutputFile {
                    primary: Some(Box::new(CliError::Output(
                        std::io::Error::from_raw_os_error(28),
                    ))),
                    close: Some(Box::new(CliError::Output(std::io::Error::other("PRIVATE")))),
                    opened,
                    english,
                };
                assert_eq!(error.code(), "storage_unavailable");
                assert!(!error.to_string().contains("PRIVATE"));
                let causes = error.typed_causes();
                assert_eq!(
                    causes
                        .iter()
                        .filter(|e| e.downcast_ref::<std::io::Error>().is_some())
                        .count(),
                    2
                );
                assert!(error.recorded_work().is_none());
                let nested = CliError::Input(std::io::Error::other(error));
                assert!(nested.typed_causes().iter().any(|e| e
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.raw_os_error() == Some(28))));
            }
        }
    }
}
