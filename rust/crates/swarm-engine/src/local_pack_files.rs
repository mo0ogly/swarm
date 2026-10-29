//! Owned private disk files for a future Git transfer job, not its process owner.
use super::{File, LocalDirectory, LocalStorageError as E};
use sha2::{Digest, Sha256};
use std::io::{self, Seek, SeekFrom, Write};
/// Fixed protocol files. Their parent and every original descriptor remain owned
/// across a caller's subprocess operation. The caller still owns source/dest and
/// unresolved process lifetime; this object starts no process or business effect.
pub(crate) struct GitPackFiles {
    directory: Option<LocalDirectory>,
    revision: Option<File>,
    pack: Option<File>,
    export_error: Option<File>,
    import_input: Option<File>,
    import_output: Option<File>,
    import_error: Option<File>,
}
/// Join operation and independent cleanup causes without replacing either.
fn finish<T>(result: Result<T, E>, cleanup: Result<(), E>) -> Result<T, E> {
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(mut e), Err(c)) => {
            e.append_cleanup(c);
            Err(e)
        }
    }
}
/// An impossible partial-access state refuses instead of adopting another file.
fn missing() -> E {
    E::new(
        super::LocalStorageOperation::OpenFile,
        io::Error::other("pack-files-not-open"),
    )
}
impl GitPackFiles {
    /// Create a strict new private directory and all output files. No existing
    /// artifact is adopted or truncated. On partial failure close every acquired
    /// handle; a directory/files may remain, so the caller retains its chosen name.
    pub(crate) fn create(parent: &LocalDirectory, name: &str) -> Result<Self, E> {
        let directory = parent.create_private_child(name)?;
        let mut files = Self {
            directory: Some(directory),
            revision: None,
            pack: None,
            export_error: None,
            import_input: None,
            import_output: None,
            import_error: None,
        };
        let result = (|| {
            let dir = files.directory.as_ref().ok_or_else(missing)?;
            files.revision = Some(dir.create_stream("candidate.oid")?);
            files.pack = Some(dir.create_stream("candidate.pack")?);
            files.export_error = Some(dir.create_stream("export.stderr")?);
            files.import_output = Some(dir.create_stream("import.stdout")?);
            files.import_error = Some(dir.create_stream("import.stderr")?);
            Ok(())
        })();
        match result {
            Ok(()) => Ok(files),
            Err(e) => finish(Err(e), files.close()),
        }
    }
    /// Write and sync exact caller-validated revision input, once into an empty
    /// file, then seek zero. The caller owes OID/schema/authority validation.
    pub(crate) fn write_revision(&mut self, bytes: &[u8]) -> Result<(), E> {
        let file = self.revision.as_mut().ok_or_else(missing)?;
        if file
            .metadata()
            .map_err(|e| E::new(super::LocalStorageOperation::Metadata, e))?
            .len()
            != 0
        {
            return Err(E::new(
                super::LocalStorageOperation::Write,
                io::Error::other("pack-revision-already-written"),
            ));
        }
        file.write_all(bytes)
            .map_err(|e| E::new(super::LocalStorageOperation::Write, e))?;
        file.sync_all()
            .map_err(|e| E::new(super::LocalStorageOperation::SyncFile, e))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| E::new(super::LocalStorageOperation::Read, e))?;
        Ok(())
    }
    /// Borrow original input and both disk export sinks. Clone only the input for
    /// child stdin; keep this owner and source capabilities through actual reap.
    pub(crate) fn export_streams(&mut self) -> Result<(&File, &mut File, &mut File), E> {
        Ok((
            self.revision.as_ref().ok_or_else(missing)?,
            self.pack.as_mut().ok_or_else(missing)?,
            self.export_error.as_mut().ok_or_else(missing)?,
        ))
    }
    /// Open the exported pack via the retained parent at offset zero. No pathname
    /// reopen outside this directory, alias adoption or second preparation allowed.
    pub(crate) fn prepare_import(&mut self) -> Result<(), E> {
        if self.import_input.is_some() {
            return Err(E::new(
                super::LocalStorageOperation::OpenFile,
                io::Error::other("pack-input-already-open"),
            ));
        }
        use std::os::unix::fs::MetadataExt;
        let input = self
            .directory
            .as_ref()
            .ok_or_else(missing)?
            .open_stream("candidate.pack")?;
        let checked = (|| {
            let actual = input
                .metadata()
                .map_err(|e| E::new(super::LocalStorageOperation::Metadata, e))?;
            let original = self
                .pack
                .as_ref()
                .ok_or_else(missing)?
                .metadata()
                .map_err(|e| E::new(super::LocalStorageOperation::Metadata, e))?;
            if (actual.dev(), actual.ino()) != (original.dev(), original.ino()) {
                return Err(E::new(
                    super::LocalStorageOperation::OpenFile,
                    io::Error::other("pack-input-rebound"),
                ));
            }
            Ok(())
        })();
        if let Err(error) = checked {
            return finish(Err(error), LocalDirectory::close_stream(input));
        }
        self.import_input = Some(input);
        Ok(())
    }
    /// Borrow parent-owned pack input and independent empty import output/error.
    /// A dependent runner error must not cause this owner to be dropped early.
    pub(crate) fn import_streams(&mut self) -> Result<(&File, &mut File, &mut File), E> {
        Ok((
            self.import_input.as_ref().ok_or_else(missing)?,
            self.import_output.as_mut().ok_or_else(missing)?,
            self.import_error.as_mut().ok_or_else(missing)?,
        ))
    }
    /// Hash the retained exported pack incrementally after the exporter is reaped.
    /// The fixed read chunk is a technical buffer, not a product size/volume cap.
    pub(crate) fn pack_identity(&mut self) -> Result<(u64, String), E> {
        use std::io::Read;
        let file = self.pack.as_mut().ok_or_else(missing)?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| E::new(super::LocalStorageOperation::Read, e))?;
        let mut h = Sha256::new();
        let mut bytes = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            let n = file
                .read(&mut buffer)
                .map_err(|e| E::new(super::LocalStorageOperation::Read, e))?;
            if n == 0 {
                break;
            }
            bytes = bytes.checked_add(n as u64).ok_or_else(|| {
                E::new(
                    super::LocalStorageOperation::Read,
                    io::Error::other("pack-length-representation"),
                )
            })?;
            h.update(&buffer[..n]);
        }
        Ok((bytes, format!("{:x}", h.finalize())))
    }
    /// Checked-close every parent file before its directory, even after failures.
    /// This deletes no artifact and does not establish subprocess completion;
    /// the future job must resolve all dependent children before calling close.
    pub(crate) fn close(mut self) -> Result<(), E> {
        let mut result = Ok(());
        for file in [
            &mut self.import_input,
            &mut self.import_output,
            &mut self.import_error,
            &mut self.export_error,
            &mut self.pack,
            &mut self.revision,
        ] {
            if let Some(file) = file.take() {
                result = finish(result, LocalDirectory::close_stream(file));
            }
        }
        if let Some(dir) = self.directory.take() {
            result = finish(result, dir.close());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::Read,
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::Path,
        process::Command,
    };
    /// Fresh private filesystem fixture, no live Root or positive business SQL.
    fn parent() -> (tempfile::TempDir, LocalDirectory) {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let dir = LocalDirectory::open(temp.path()).unwrap();
        (temp, dir)
    }
    /// Fixed real Git fixture command, no shell/global overrides or hooks.
    fn git(root: &Path, args: &[&str]) -> Vec<u8> {
        let mut command = Command::new("git");
        for (k, _) in std::env::vars_os() {
            if k.as_encoded_bytes().starts_with(b"GIT_") {
                command.env_remove(k);
            }
        }
        let out = command
            .args([
                "--no-pager",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "init.templateDir=",
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@invalid",
            ])
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(out.status.success(), "{:?}", out.stderr);
        out.stdout
    }
    /// Existing directory/file and a second revision write refuse byte-preserving;
    /// exact private modes and original inode establish which artifact survived.
    #[test]
    fn pack_files_exclusive_creation_and_revision_refusal() {
        let (temp, dir) = parent();
        let mut files = GitPackFiles::create(&dir, "pack").unwrap();
        let oid = format!("{}\n", "1".repeat(40));
        files.write_revision(oid.as_bytes()).unwrap();
        let original = fs::metadata(temp.path().join("pack/candidate.oid")).unwrap();
        assert_eq!(original.permissions().mode() & 0o777, 0o600);
        assert!(GitPackFiles::create(&dir, "pack").is_err());
        assert!(files.write_revision(b"replacement\n").is_err());
        assert_eq!(
            fs::read(temp.path().join("pack/candidate.oid")).unwrap(),
            oid.as_bytes()
        );
        assert_eq!(
            fs::metadata(temp.path().join("pack/candidate.oid"))
                .unwrap()
                .ino(),
            original.ino()
        );
        assert!(dir.create_private_child("../escape").is_err());
        files.close().unwrap();
        dir.close().unwrap();
    }
    /// NOFOLLOW and single-link checks reject symlink/hardlink inputs without
    /// changing originals; removing only the owned fault permits valid reopening.
    #[test]
    fn pack_files_link_refusal_and_recovery() {
        let (temp, dir) = parent();
        let mut file = dir.create_stream("plain").unwrap();
        file.write_all(b"original").unwrap();
        file.sync_all().unwrap();
        LocalDirectory::close_stream(file).unwrap();
        assert!(dir.create_stream("plain").is_err());
        std::os::unix::fs::symlink("plain", temp.path().join("symbol")).unwrap();
        assert!(dir.open_stream("symbol").is_err());
        fs::hard_link(temp.path().join("plain"), temp.path().join("hard")).unwrap();
        assert!(dir.open_stream("plain").is_err());
        assert!(dir.open_stream("hard").is_err());
        fs::remove_file(temp.path().join("hard")).unwrap();
        let mut file = dir.open_stream("plain").unwrap();
        let mut content = vec![];
        file.read_to_end(&mut content).unwrap();
        assert_eq!(content, b"original");
        LocalDirectory::close_stream(file).unwrap();
        dir.close().unwrap();
    }
    /// Real production pack runner uses these parent-owned files directly. Renaming
    /// the staging path cannot retarget the descriptor-relative import input. Both
    /// real repos and file owner stay alive until runner reap; refs never move.
    #[test]
    fn pack_files_real_git_streaming_and_retained_directory() {
        let (temp, dir) = parent();
        let source = dir.create_private_child("source").unwrap();
        let destination = dir.create_private_child("destination").unwrap();
        for root in [&source, &destination] {
            git(
                &root.process_path(),
                &["init", "--bare", "--object-format=sha1", "."],
            );
        }
        let tree =
            String::from_utf8(git(&source.process_path(), &["--git-dir", ".", "mktree"])).unwrap();
        let oid = String::from_utf8(git(
            &source.process_path(),
            &[
                "--git-dir",
                ".",
                "commit-tree",
                tree.trim(),
                "-m",
                "fixture",
            ],
        ))
        .unwrap();
        git(
            &source.process_path(),
            &[
                "--git-dir",
                ".",
                "update-ref",
                "refs/heads/keep",
                oid.trim(),
            ],
        );
        let before = git(&source.process_path(), &["--git-dir", ".", "for-each-ref"]);
        let settings = crate::EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture".into(),
            "2026-10-28T14:39:28Z".into(),
        )
        .unwrap();
        let mut files = GitPackFiles::create(&dir, "pack").unwrap();
        files.write_revision(oid.as_bytes()).unwrap();
        {
            let (input, out, err) = files.export_streams().unwrap();
            let result = crate::run_git_pack(
                &source.process_path(),
                crate::GitPackOperation::Export,
                input.try_clone().unwrap(),
                out,
                err,
                &settings,
            )
            .unwrap();
            assert!(result.exit_status.success());
        }
        let (bytes, digest) = files.pack_identity().unwrap();
        assert!(bytes > 0);
        let data = fs::read(temp.path().join("pack/candidate.pack")).unwrap();
        assert!(data.starts_with(b"PACK"));
        assert_eq!(data.len() as u64, bytes);
        assert_eq!(format!("{:x}", Sha256::digest(&data)), digest);
        fs::rename(temp.path().join("pack"), temp.path().join("held-pack")).unwrap();
        fs::create_dir(temp.path().join("pack")).unwrap();
        fs::set_permissions(temp.path().join("pack"), fs::Permissions::from_mode(0o700)).unwrap();
        files.prepare_import().unwrap();
        assert!(files.prepare_import().is_err());
        {
            let (input, out, err) = files.import_streams().unwrap();
            crate::run_git_pack(
                &destination.process_path(),
                crate::GitPackOperation::Import,
                input.try_clone().unwrap(),
                out,
                err,
                &settings,
            )
            .unwrap();
        }
        assert_eq!(
            git(
                &destination.process_path(),
                &["--git-dir", ".", "cat-file", "-t", oid.trim()]
            ),
            b"commit\n"
        );
        assert!(git(
            &destination.process_path(),
            &["--git-dir", ".", "for-each-ref"]
        )
        .is_empty());
        assert_eq!(
            git(&source.process_path(), &["--git-dir", ".", "for-each-ref"]),
            before
        );
        assert_eq!(fs::read_dir(temp.path().join("pack")).unwrap().count(), 0);
        files.close().unwrap();
        assert_eq!(
            fs::read(temp.path().join("held-pack/candidate.pack")).unwrap(),
            data
        );
        destination.close().unwrap();
        source.close().unwrap();
        dir.close().unwrap();
    }
    /// Opening a different inode under the same name refuses and preserves both
    /// owned originals. Restore only the controlled fixture fault before retry.
    #[test]
    fn pack_files_pack_substitution_refuses_and_recovers() {
        let (temp, dir) = parent();
        let mut files = GitPackFiles::create(&dir, "pack").unwrap();
        files
            .pack
            .as_mut()
            .unwrap()
            .write_all(b"original-pack")
            .unwrap();
        files.pack.as_mut().unwrap().sync_all().unwrap();
        let original = temp.path().join("pack/candidate.pack");
        let held = temp.path().join("pack/held.pack");
        fs::rename(&original, &held).unwrap();
        fs::write(&original, b"replacement").unwrap();
        fs::set_permissions(&original, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(files.prepare_import().is_err());
        assert_eq!(fs::read(&held).unwrap(), b"original-pack");
        assert_eq!(fs::read(&original).unwrap(), b"replacement");
        fs::remove_file(&original).unwrap();
        fs::rename(&held, &original).unwrap();
        files.prepare_import().unwrap();
        files.close().unwrap();
        dir.close().unwrap();
    }
}
