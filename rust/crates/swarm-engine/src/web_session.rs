//! Private Go-compatible local web capability storage, not account/agent authentication.
#![cfg(target_os = "linux")]
use crate::{
    local_storage::{AtomicStorageFault, LocalDirectory},
    LocalStorageError, LocalStorageOperation, PublicationState, RuntimeError,
};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener},
};
/// Inherited Go credential representation, not an operator budget or strength setting.
const CREDENTIAL_BYTES: usize = 32;
/// Opaque bearer credential from a validated private Root file. No Serialize/Clone;
/// Debug is redacted. Explicit secret access is for the local gate/bootstrap only.
/// Filesystem ownership and OS entropy do not authenticate an agent/model/author.
/// No expiry, rotation, revocation watcher or secure memory erasure is established.
pub struct WebSessionCredential {
    address: SocketAddr,
    secret: String,
    created: bool,
}
impl WebSessionCredential {
    /// Sensitive bearer bytes; never log or persist outside the private session
    /// contract. Possession permits local request access, not business authority.
    pub fn secret(&self) -> &str {
        &self.secret
    }
    /// Actual bound listener address used for the file namespace, including port.
    /// Not proof the listener remains alive or that a different Root is trusted.
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    /// Whether this operation published new bytes, rather than adopting an
    /// existing/concurrent winner. Both successful paths observed complete state.
    pub fn was_created(&self) -> bool {
        self.created
    }
}
impl fmt::Debug for WebSessionCredential {
    /// Public metadata only; neither Debug nor its fields reveal the capability.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebSessionCredential")
            .field("address", &self.address)
            .field("created", &self.created)
            .finish_non_exhaustive()
    }
}
/// Safe local session failures, retaining typed storage/entropy/listener causes.
/// A publication-uncertain error may leave visible bytes; it is not rollback proof.
pub enum WebSessionError {
    Storage(LocalStorageError),
    Entropy(RuntimeError),
    Listener(std::io::Error),
    Encoding(std::str::Utf8Error),
    Refused(&'static str),
}
impl WebSessionError {
    /// Stable diagnostic family only; no raw path, token, body or error matching.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(e) if e.publication == PublicationState::RenamedDurabilityUnknown => {
                "web-session-state-uncertain"
            }
            Self::Storage(_) => "web-session-storage",
            Self::Entropy(_) => "web-session-entropy",
            Self::Listener(_) => "web-session-listener",
            Self::Encoding(_) => "web-session-credential-invalid",
            Self::Refused(code) => code,
        }
    }
    /// True only for a visible rename whose directory durability was not proven.
    /// False does not assert that an older/concurrent session file is absent.
    pub fn publication_uncertain(&self) -> bool {
        matches!(self,Self::Storage(e) if e.publication==PublicationState::RenamedDurabilityUnknown)
    }
}
impl fmt::Display for WebSessionError {
    /// Render a safe family; the owner may inspect original typed causes separately.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for WebSessionError {
    /// Keep both normal/debug diagnostics free of bearer material and paths.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for WebSessionError {
    /// Preserve the original cause; secondary cleanup remains in LocalStorageError.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(e) => Some(e),
            Self::Entropy(e) => Some(e),
            Self::Listener(e) => Some(e),
            Self::Encoding(e) => Some(e),
            Self::Refused(_) => None,
        }
    }
}
/// Go's on-disk session-prefix plus24ASCII-name representation, exactly32bytes.
/// Valid format does not prove trusted generation, ownership, freshness or entropy.
pub fn valid_web_session_credential(value: &str) -> bool {
    value.starts_with("session-")
        && value.len() == CREDENTIAL_BYTES
        && swarm_identity_contract::name(value.as_bytes())
}
/// Accept the exact local Go bindings from actual socket metadata, never a client
/// Host/address claim. No accept/read/write, timeout or external interface bind.
fn observed(listener: &TcpListener) -> Result<SocketAddr, WebSessionError> {
    let address = listener.local_addr().map_err(WebSessionError::Listener)?;
    if !matches!(address.ip(),IpAddr::V4(v) if v==Ipv4Addr::LOCALHOST)
        && !matches!(address.ip(),IpAddr::V6(v) if v==Ipv6Addr::LOCALHOST)
    {
        return Err(WebSessionError::Refused("web-listener-loopback-required"));
    }
    Ok(address)
}
/// Read one complete regular/private/owned/single-link file with checked close.
/// Exact32byte framing refuses excess bytes, UTF8/format errors and all unsafe
/// objects. Missing is distinct from invalid; invalid state is never regenerated.
fn read(directory: &LocalDirectory, name: &str) -> Result<Option<String>, WebSessionError> {
    match directory.read_bounded(name, CREDENTIAL_BYTES) {
        Ok(raw) => {
            let value = std::str::from_utf8(&raw)
                .map_err(WebSessionError::Encoding)?
                .to_owned();
            if !valid_web_session_credential(&value) {
                return Err(WebSessionError::Refused("web-session-credential-invalid"));
            }
            Ok(Some(value))
        }
        Err(e)
            if ordinary_errno(
                &e,
                LocalStorageOperation::OpenFile,
                rustix::io::Errno::NOENT,
            ) =>
        {
            Ok(None)
        }
        Err(e) => Err(WebSessionError::Storage(e)),
    }
}
/// Recognize only a single pre-publication OS condition at its precise operation;
/// cleanup/uncertain-publication errors must never become a recoverable collision.
fn ordinary_errno(
    error: &LocalStorageError,
    operation: LocalStorageOperation,
    errno: rustix::io::Errno,
) -> bool {
    error.operation == operation
        && error.publication == PublicationState::NotPublished
        && error.cleanup.is_none()
        && error
            .source()
            .and_then(|e| e.downcast_ref::<rustix::io::Errno>())
            .is_some_and(|e| *e == errno)
}
/// Reuse existing bytes or publish an OS-generated credential without replacement.
/// One atomic NOREPLACE collision reads the complete winning file once; this is
/// race resolution, not a retry loop or operator policy. Failed writes preserve
/// originals; directory-sync uncertainty remains typed. Root owns directory and
/// writable authority; no Root DB, Work, budget or attempt is mutated here.
pub(crate) fn load_or_create(
    directory: &LocalDirectory,
    listener: &TcpListener,
) -> Result<WebSessionCredential, WebSessionError> {
    load_or_create_using(
        directory,
        listener,
        || crate::generate_identifier("session-"),
        AtomicStorageFault::None,
    )
}
/// Internal entropy injection only for deterministic storage-fault/concurrency
/// witnesses; production calls the existing OS generator. No public fixture flag.
fn load_or_create_using(
    directory: &LocalDirectory,
    listener: &TcpListener,
    entropy: impl FnOnce() -> Result<String, RuntimeError>,
    fault: AtomicStorageFault,
) -> Result<WebSessionCredential, WebSessionError> {
    let address = observed(listener)?;
    let hash = format!("{:x}", Sha256::digest(address.to_string().as_bytes()));
    let name = format!("web-session-{}", &hash[..16]); // inherited Go namespace, not a secret-strength cutoff
    if let Some(secret) = read(directory, &name)? {
        return Ok(WebSessionCredential {
            address,
            secret,
            created: false,
        });
    }
    let secret = entropy().map_err(WebSessionError::Entropy)?;
    if !valid_web_session_credential(&secret) {
        return Err(WebSessionError::Refused("web-session-entropy-format"));
    }
    match directory.publish_new_with_fault(&name, secret.as_bytes(), fault) {
        Ok(()) => Ok(WebSessionCredential {
            address,
            secret,
            created: true,
        }),
        Err(e) if ordinary_errno(&e, LocalStorageOperation::Rename, rustix::io::Errno::EXIST) => {
            let secret = read(directory, &name)?
                .ok_or(WebSessionError::Refused("web-session-winner-missing"))?;
            Ok(WebSessionCredential {
                address,
                secret,
                created: false,
            })
        }
        Err(e) => Err(WebSessionError::Storage(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    /// Owned private directory for isolated actual publication operations. No Root/mission reused.
    fn fixture() -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(crate::generate_identifier("session-storage91-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    /// A valid witness precedes entropy/representation failures. Both keep causes,
    /// publish no bytes for the failed address and allow later explicit recovery.
    #[test]
    fn session_entropy_and_generator_format_refuse_without_publication_then_recover() {
        let path = fixture();
        let directory = LocalDirectory::open(&path).unwrap();
        let witness = TcpListener::bind("127.0.0.1:0").unwrap();
        assert!(load_or_create(&directory, &witness).unwrap().was_created());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let error = load_or_create_using(
            &directory,
            &listener,
            || {
                Err(RuntimeError::new(
                    crate::RuntimeOperation::Entropy,
                    std::io::Error::other("fixture entropy unavailable"),
                ))
            },
            AtomicStorageFault::None,
        )
        .unwrap_err();
        assert_eq!(error.code(), "web-session-entropy");
        assert!(error.source().unwrap().source().is_some());
        assert!(!error.publication_uncertain());
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
        let error = load_or_create_using(
            &directory,
            &listener,
            || Ok("invalid".to_owned()),
            AtomicStorageFault::None,
        )
        .unwrap_err();
        assert_eq!(error.code(), "web-session-entropy-format");
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
        assert!(load_or_create(&directory, &listener).unwrap().was_created());
        directory.close().unwrap();
        fs::remove_dir_all(path).unwrap();
    }
    /// Both reads are forced before either publication; NOREPLACE loser adopts winner.
    #[test]
    fn session_forced_publication_collision_adopts_complete_winner() {
        let path = fixture();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let barrier = std::sync::Barrier::new(2);
        let values = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|_| {
                    let path = &path;
                    let listener = &listener;
                    let barrier = &barrier;
                    scope.spawn(move || {
                        let directory = LocalDirectory::open(path).unwrap();
                        let result = load_or_create_using(
                            &directory,
                            listener,
                            || {
                                barrier.wait();
                                crate::generate_identifier("session-")
                            },
                            AtomicStorageFault::None,
                        )
                        .unwrap();
                        directory.close().unwrap();
                        result
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(values.iter().filter(|s| s.was_created()).count(), 1);
        assert_eq!(values[0].secret(), values[1].secret());
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
        fs::remove_dir_all(path).unwrap();
    }
    /// Compile-gated storage failures, not a real full disk/OS close failure.
    /// Valid publication witness precedes each injected fault; replay/recovery never overwrites.
    #[cfg(feature = "test-fixtures")]
    #[test]
    fn session_atomic_faults_preserve_publication_and_recovery_contract() {
        for fault in [
            AtomicStorageFault::PartialWriteNoSpace,
            AtomicStorageFault::SyncFile,
            AtomicStorageFault::CloseFile,
            AtomicStorageFault::Rename,
            AtomicStorageFault::SyncDirectory,
            AtomicStorageFault::SyncAndCleanup,
        ] {
            let path = fixture();
            let directory = LocalDirectory::open(&path).unwrap();
            let witness = TcpListener::bind("127.0.0.1:0").unwrap();
            assert!(load_or_create(&directory, &witness).unwrap().was_created());
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let error = load_or_create_using(
                &directory,
                &listener,
                || crate::generate_identifier("session-"),
                fault,
            )
            .unwrap_err();
            assert_eq!(
                error.publication_uncertain(),
                fault == AtomicStorageFault::SyncDirectory
            );
            if fault == AtomicStorageFault::SyncAndCleanup {
                assert!(matches!(&error,WebSessionError::Storage(e) if e.cleanup.is_some()));
            }
            if fault == AtomicStorageFault::PartialWriteNoSpace {
                assert!(
                    error
                        .source()
                        .unwrap()
                        .source()
                        .unwrap()
                        .downcast_ref::<std::io::Error>()
                        .unwrap()
                        .raw_os_error()
                        == Some(rustix::io::Errno::NOSPC.raw_os_error())
                );
            }
            let recovered = load_or_create(&directory, &listener).unwrap();
            assert_eq!(
                recovered.was_created(),
                fault != AtomicStorageFault::SyncDirectory
            );
            assert_eq!(
                load_or_create(&directory, &listener).unwrap().secret(),
                recovered.secret()
            );
            directory.close().unwrap();
            fs::remove_dir_all(path).unwrap();
        }
    }
}
