//! Shared helper-side exec transport. It supplies no identity or permission.
#![cfg(target_os = "linux")]
#[path = "native_provider_biba.rs"]
mod biba;
pub(crate) use biba::{output_workspace, OutputRole};
pub(crate) use biba::{prepare as prepare_boundary, enforce as enforce_boundary, close as close_boundary};
use crate::{root_store::finished as finish, AgentLaunchError, Provider, RootStoreError as E};
use std::{
    collections::BTreeSet,
    convert::Infallible,
    error::Error,
    ffi::CString,
    fs::File,
    io::{Seek, SeekFrom, Write},
    os::{fd::AsRawFd, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
};

/// Retain the existing worker transport diagnostic family without exposing
/// caller paths, arguments, prompt or environment values in rendered failures.
fn caused(code: &'static str, error: impl Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(AgentLaunchError::caused(code, error)))
}

/// Engine-created C-string vectors and immutable anonymous stdin. Construction
/// performs no exec and grants no capability. Only a separately verified private
/// gate may consume it, after Root closure and fresh identity/policy checks.
pub(crate) struct PreparedExec {
    input: File,
    path: CString,
    args: Vec<CString>,
    environment: Vec<CString>,
    workspace: PathBuf,
    workspace_identity: Option<(u64, u64)>,
}
impl PreparedExec {
    /// Encode exact CLI arguments, observe only the supplied validated environment
    /// names, and seal complete prompt bytes in a CLOEXEC memfd. Requires absolute
    /// image/workspace paths; duplicate/invalid environment names and embedded NUL
    /// refuse before allocating stdin. Missing allowed variables remain absent.
    /// This checks transport shape, not executable ownership, effective isolation,
    /// model capability, provider health, pricing or authorization. Callers must
    /// establish and freshly recheck those contracts separately. No policy cap,
    /// timeout, shell/PATH fallback or inferred default is introduced.
    /// Codec/kernel/write/seek/seal/close causes survive; failed preparation closes
    /// its owned descriptor. The legacy memfd label is a technical identifier.
    pub(crate) fn prepare(
        provider: &Provider,
        environment_names: &[String],
        workspace: &Path,
        prompt: &[u8],
    ) -> Result<Self, E> {
        use rustix::fs::{fcntl_add_seals, memfd_create, MemfdFlags, SealFlags};
        if !Path::new(&provider.command).is_absolute() || !workspace.is_absolute() {
            return Err(E::Refused("agent-execution-absolute-path-required"));
        }
        let cstring = |bytes: &[u8]| {
            CString::new(bytes).map_err(|e| caused("agent-execution-argument-invalid", e))
        };
        let path = cstring(provider.command.as_bytes())?;
        cstring(workspace.as_os_str().as_bytes())?;
        let mut args = vec![path.clone()];
        for arg in provider.args.as_deref().unwrap_or_default() {
            args.push(cstring(arg.as_bytes())?);
        }
        let mut names = BTreeSet::new();
        let mut environment = Vec::new();
        for name in environment_names {
            if !crate::agent_launch_policy::environment_name(name) || !names.insert(name) {
                return Err(E::Refused("agent-execution-environment-invalid"));
            }
            if let Some(value) = std::env::var_os(name) {
                let mut entry = name.as_bytes().to_vec();
                entry.push(b'=');
                entry.extend_from_slice(value.as_os_str().as_bytes());
                environment.push(cstring(&entry)?);
            }
        }
        let fd = memfd_create(
            "swarm-agent-input",
            MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
        )
        .map_err(|e| caused("agent-execution-input-create", e))?;
        // A helper can intentionally close stdin before transport preparation.
        // Keep the owned memfd outside stdio so attach/close cannot close fd0.
        let fd = if fd.as_raw_fd() < 3 {
            let high = rustix::io::fcntl_dupfd_cloexec(&fd, 3)
                .map_err(|e| caused("agent-execution-input-relocate", e));
            finish(high, nix::unistd::close(fd)
                .map_err(|e| caused("agent-execution-input-close", e)))?
        } else { fd };
        let mut input = File::from(fd);
        let fill = (|| {
            input
                .write_all(prompt)
                .map_err(|e| caused("agent-execution-input-write", e))?;
            input
                .seek(SeekFrom::Start(0))
                .map_err(|e| caused("agent-execution-input-rewind", e))?;
            fcntl_add_seals(
                &input,
                SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE,
            )
            .map_err(|e| caused("agent-execution-input-seal", e))?;
            Ok(())
        })();
        if let Err(error) = fill {
            return finish(
                Err(error),
                nix::unistd::close(input).map_err(|e| caused("agent-execution-input-close", e)),
            );
        }
        Ok(Self {
            input,
            path,
            args,
            environment,
            workspace: workspace.to_owned(),
            workspace_identity: None,
        })
    }

    /// Attach the private gate's current workspace observation, never a caller
    /// authority tuple. The consuming Linux profile compares its pinned inode
    /// before granting output access. Missing/mismatched observations refuse.
    pub(crate) fn with_workspace_identity(mut self, identity: (u64, u64)) -> Self {
        self.workspace_identity = Some(identity);
        self
    }

    /// Append engine-computed variables (LIVE-UX-03 telemetry exporter) after all
    /// gate/freshness checks. Names must be valid environment names and must not
    /// already be present: an inherited variable is never overridden
    /// (`agent-execution-environment-conflict`), and duplicates refuse. Values
    /// may hold a per-attempt secret; they are only kept for `execve`. On
    /// refusal the prepared stdin descriptor is closed and the cause returned.
    pub(crate) fn add_environment(mut self, entries: Vec<(String, String)>) -> Result<Self, E> {
        let check = (|| {
            for (name, value) in &entries {
                let prefix = format!("{name}=");
                if !crate::agent_launch_policy::environment_name(name)
                    || self.environment.iter().any(|e| e.as_bytes().starts_with(prefix.as_bytes()))
                {
                    return Err(E::Refused("agent-execution-environment-conflict"));
                }
                self.environment.push(
                    CString::new(format!("{name}={value}")).map_err(|e| caused("agent-execution-argument-invalid", e))?,
                );
            }
            Ok(())
        })();
        match check {
            Ok(()) => Ok(self),
            Err(error) => finish(Err(error), self.close()),
        }
    }

    /// Insert engine-computed arguments right after the program name (LIVE-UX-03
    /// Codex telemetry: global `-c otel.…` overrides, which Codex accepts before
    /// its subcommand), after all gate/freshness checks. They must hold no
    /// secret (argv is visible to other local processes); embedded NUL refuses
    /// (`agent-execution-argument-invalid`) and closes the prepared stdin.
    pub(crate) fn add_leading_arguments(mut self, entries: Vec<String>) -> Result<Self, E> {
        let encoded = entries
            .iter()
            .map(|a| CString::new(a.as_bytes()).map_err(|e| caused("agent-execution-argument-invalid", e)))
            .collect::<Result<Vec<_>, _>>();
        match encoded {
            Ok(encoded) => {
                self.args.splice(1..1, encoded);
                Ok(self)
            }
            Err(error) => finish(Err(error), self.close()),
        }
    }

    /// Explicitly close immutable stdin after a later gate/freshness refusal.
    /// No exec/release occurs; caller must retain any accompanying primary cause.
    pub(crate) fn close(self) -> Result<(), E> {
        nix::unistd::close(self.input).map_err(|e| caused("agent-execution-input-close", e))
    }

    /// Replace an already authorized private helper with the exact absolute image,
    /// argv and observed environment. Requires an independently checked committed
    /// gate, current identity/policy/source receipts and closed Root. This function
    /// cannot establish those preconditions itself and is never a public JSON API.
    /// Require the validated Root and an attributed isolated I0 workspace. Prepare
    /// and enforce the mandatory Linux boundary; no unconfined fallback exists.
    /// Changes cwd and fd0 only in the consuming helper, then calls execve directly
    /// without ENOEXEC shell fallback or PATH search. Successful exec never returns.
    /// All returning paths preserve setup/close/kernel failures; cwd/fd0 changes
    /// are not restored because the refusing helper must exit. It accepts no task.
    pub(crate) fn execute(self, root: &Path) -> Result<Infallible, E> {
        let Some(identity) = self.workspace_identity else {
            return finish(Err(E::Refused("agent-execution-workspace-witness-required")), self.close());
        };
        let sandbox = match biba::prepare(root, &self.workspace, identity) {
            Ok(sandbox) => sandbox,
            Err(error) => return finish(Err(error), self.close()),
        };
        let setup = rustix::stdio::dup2_stdin(&self.input)
            .map_err(|e| caused("agent-execution-input-attach", e));
        let attached = finish(
            setup,
            nix::unistd::close(self.input).map_err(|e| caused("agent-execution-input-close", e)),
        );
        if let Err(error) = attached {
            return finish(Err(error), biba::close(sandbox));
        }
        biba::enforce(sandbox)?;
        nix::unistd::execve(&self.path, &self.args, &self.environment)
            .map_err(|e| caused("agent-provider-exec", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// Construct transport only: no filesystem image is executed or trusted.
    fn provider() -> Provider {
        Provider {
            command: "/owned-fixture/provider".into(),
            args: Some(vec!["--json".into()]),
            ..Default::default()
        }
    }

    /// Engine-computed variables append after a witness; an inherited name is
    /// never overridden, and invalid or duplicate names refuse.
    #[test]
    fn added_environment_appends_and_never_overrides_inherited_names() {
        let fixture = || PreparedExec::prepare(&provider(), &["PATH".into()], Path::new("/owned-fixture"), b"").unwrap();
        let p = fixture().add_environment(vec![("OTEL_LOGS_EXPORTER".into(), "otlp".into())]).unwrap();
        assert!(p.environment.iter().any(|e| e.to_bytes() == b"OTEL_LOGS_EXPORTER=otlp"));
        p.close().unwrap();
        assert!(std::env::var_os("PATH").is_some(), "test needs an inherited PATH");
        for entries in [
            vec![("PATH".to_owned(), "/evil".to_owned())],
            vec![("A=B".to_owned(), "x".to_owned())],
            vec![("OTEL_X".to_owned(), "1".to_owned()), ("OTEL_X".to_owned(), "2".to_owned())],
            vec![("OTEL_X".to_owned(), "nul\0".to_owned())],
        ] {
            let refused = fixture().add_environment(entries).err().unwrap();
            assert!(matches!(refused.code(), "agent-execution-environment-conflict" | "agent-execution-argument-invalid"), "{}", refused.code());
        }
    }

    /// Leading arguments land right after the program name, before the
    /// provider's own arguments; an embedded NUL refuses.
    #[test]
    fn leading_arguments_follow_the_program_and_refuse_nul() {
        let fixture = || PreparedExec::prepare(&provider(), &[], Path::new("/owned-fixture"), b"").unwrap();
        let p = fixture().add_leading_arguments(vec!["-c".into(), "otel.log_user_prompt=false".into()]).unwrap();
        let argv: Vec<_> = p.args.iter().map(|a| a.to_str().unwrap().to_owned()).collect();
        assert_eq!(argv, ["/owned-fixture/provider", "-c", "otel.log_user_prompt=false", "--json"]);
        p.close().unwrap();
        let refused = fixture().add_leading_arguments(vec!["nul\0".into()]).err().unwrap();
        assert_eq!(refused.code(), "agent-execution-argument-invalid");
        let unchanged = fixture().add_leading_arguments(Vec::new()).unwrap();
        assert_eq!(unchanged.args.len(), 2, "nothing added when telemetry is off");
        unchanged.close().unwrap();
    }

    /// The actual memfd preserves binary bytes at offset0 and cannot be written,
    /// resized or unsealed. Empty environment excludes inherited parent secrets.
    #[test]
    fn native_exec_input_is_complete_sealed_and_cloexec() {
        use rustix::fs::{fcntl_get_seals, SealFlags};
        let input = b"owned\0input\n";
        let mut p =
            PreparedExec::prepare(&provider(), &[], Path::new("/owned-fixture"), input).unwrap();
        assert_eq!(
            p.args.iter().map(|a| a.to_bytes()).collect::<Vec<_>>(),
            [b"/owned-fixture/provider".as_slice(), b"--json".as_slice()]
        );
        assert!(p.environment.is_empty());
        let mut read = Vec::new();
        p.input.read_to_end(&mut read).unwrap();
        assert_eq!(read, input);
        assert!(fcntl_get_seals(&p.input)
            .unwrap()
            .contains(SealFlags::SEAL | SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK));
        assert!(rustix::io::fcntl_getfd(&p.input)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC));
        assert_eq!(
            p.input.write_all(b"mutation").unwrap_err().raw_os_error(),
            Some(rustix::io::Errno::PERM.raw_os_error())
        );
        p.close().unwrap();
    }

    /// Invalid transport cannot silently become relative PATH lookup, shell argv
    /// or a successful empty command; diagnostics never include supplied text.
    #[test]
    fn native_exec_rejects_relative_paths_and_nul_arguments() {
        let mut provider = provider();
        provider.command = "PRIVATE-relative".into();
        let e = PreparedExec::prepare(&provider, &[], Path::new("/owned-fixture"), b"")
            .err()
            .unwrap();
        assert_eq!(e.code(), "agent-execution-absolute-path-required");
        assert!(!e.to_string().contains("PRIVATE"));
        provider = super::tests::provider();
        assert_eq!(
            PreparedExec::prepare(&provider, &[], Path::new("PRIVATE-relative"), b"")
                .err()
                .unwrap()
                .code(),
            "agent-execution-absolute-path-required"
        );
        provider.args = Some(vec!["PRIVATE\0argument".into()]);
        let e = PreparedExec::prepare(&provider, &[], Path::new("/owned-fixture"), b"")
            .err()
            .unwrap();
        assert_eq!(e.code(), "agent-execution-argument-invalid");
        assert!(!e.to_string().contains("PRIVATE"));
        assert!(e.source().is_some());
    }

    /// Duplicate/invalid environment names are refused, not silently ignored or
    /// converted into child assignments. No global process environment is edited.
    #[test]
    fn native_exec_rejects_ambiguous_environment_allowlists() {
        for names in [
            vec!["HOME".into(), "HOME".into()],
            vec!["PRIVATE=value".into()],
            vec![String::new()],
        ] {
            let e = PreparedExec::prepare(&provider(), &names, Path::new("/owned-fixture"), b"")
                .err()
                .unwrap();
            assert_eq!(e.code(), "agent-execution-environment-invalid");
            assert!(!e.to_string().contains("PRIVATE"));
        }
        PreparedExec::prepare(
            &provider(),
            &["HOME".into()],
            Path::new("/owned-fixture"),
            b"",
        )
        .unwrap()
        .close()
        .unwrap();
    }
    /// Preserve the original public Root cleanup classification when transport
    /// fails and closing its input also fails. Healthy single-failure control
    /// precedes compound injection; raw private causes must remain unrendered.
    #[test]
    fn native_exec_preserves_root_cleanup_error_contract() {
        let error = || {
            caused(
                "agent-execution-input-write",
                std::io::Error::other("PRIVATE write cause"),
            )
        };
        let control = finish::<()>(Err(error()), Ok(())).unwrap_err();
        assert_eq!(control.code(), "agent-execution-input-write");
        assert!(control.source().is_some());
        let result = finish::<()>(
            Err(error()),
            Err(caused(
                "agent-execution-input-close",
                std::io::Error::other("PRIVATE close cause"),
            )),
        )
        .unwrap_err();
        assert_eq!(result.code(), "root-cleanup");
        assert!(!result.to_string().contains("PRIVATE"));
        assert!(result.source().is_some());
    }
}
