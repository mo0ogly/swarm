//! Open a pinned v3composition on existing protected sources. This starts no
//! service, initializes no account/checkpoint and authenticates no human/session.
use crate::{
    authenticated_control::AuthenticatedControl,
    control::{self, Control},
    personal::{self, coordinator::Coordinator},
    personal_lifecycle::PersonalLifecycle,
    protected_signer, startup,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::{error::Error as StdError, fmt, path::Path};
use swarm_enrollment_bootstrap::{self as bootstrap, AuthenticatedDocument};
use swarm_enrollment_config::{self as config, store::documents::Documents};
use swarm_enrollment_state::{Context, Head};
/// Safe primary refusal with independent cleanup branches. Neither Err nor a
/// close error implies absence of lockfiles/SQLite sidecars or an earlier effect.
pub enum Error {
    Bootstrap(bootstrap::Error),
    BusinessDefinition(startup::Error),
    Binding,
    Key(base64::DecodeError),
    Project(std::num::ParseIntError),
    Personal(personal::Error),
    Storage(config::Error),
    Signing(protected_signer::Error),
    Business(control::Error),
    Cleanup {
        cause: Box<Error>,
        source: Option<personal::Error>,
        coordinator: Option<personal::coordinator::Error>,
    },
}
impl Error {
    /// Safe category, no declaration/path/key/verifier/callback contents.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Bootstrap(e) => e.code(),
            Self::BusinessDefinition(e) => e.code(),
            Self::Binding | Self::Key(_) | Self::Project(_) => "binding",
            Self::Personal(e) => e.code(),
            Self::Storage(e) => e.code(),
            Self::Signing(e) => e.code(),
            Self::Business(e) => e.code(),
            Self::Cleanup { cause, .. } => cause.code(),
        }
    }
    /// Explicit source close failure after a later document-store open refusal.
    pub fn source_cleanup(&self) -> Option<&personal::Error> {
        match self {
            Self::Cleanup { source, .. } => source.as_ref(),
            _ => None,
        }
    }
    /// Independent coordinator close failure after business source open refusal;
    /// includes its own file/journal primary/secondary branches.
    pub fn coordinator_cleanup(&self) -> Option<&personal::coordinator::Error> {
        match self {
            Self::Cleanup { coordinator, .. } => coordinator.as_ref(),
            _ => None,
        }
    }
}
impl fmt::Display for Error {
    /// Do not format nested private sources, paths, keys or declarations.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "authenticated startup: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same redacted category, with typed causes available separately.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Primary typed cause. Inspect source_cleanup/coordinator_cleanup as well.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Bootstrap(e) => Some(e),
            Self::BusinessDefinition(e) => Some(e),
            Self::Key(e) => Some(e),
            Self::Project(e) => Some(e),
            Self::Personal(e) => Some(e),
            Self::Storage(e) => Some(e),
            Self::Signing(e) => Some(e),
            Self::Business(e) => Some(e),
            Self::Cleanup { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Resolve only already validated independent personal context. Never read pins,
/// identity, generation or current head from a journal, checkpoint or request.
fn personal_inputs(
    d: &AuthenticatedDocument,
) -> Result<(personal::Expected, [u8; 32], [u8; 32]), Error> {
    let key: [u8; 32] = URL_SAFE_NO_PAD
        .decode(&d.personal.authority_public_key)
        .map_err(Error::Key)?
        .try_into()
        .map_err(|_| Error::Binding)?;
    let mut project = [0; 32];
    for (index, byte) in project.iter_mut().enumerate() {
        *byte = u8::from_str_radix(
            &d.personal.project_context_digest[index * 2..index * 2 + 2],
            16,
        )
        .map_err(Error::Project)?;
    }
    let expected = personal::Expected {
        installation_digest: d.control.installation.pins.installation_digest.clone(),
        principal: d.control.installation.pins.principal.clone(),
        generation: d.personal.generation,
    };
    Ok((expected, key, project))
}
/// Load independent pinned v3declaration and existing actual personal/business
/// journal objects, then transfer ownership to the authenticated composition.
/// Loader inspects all private roots/files without mutation; subsequent opens can
/// create private lockfiles and SQLite may manage WAL/SHM. No checkpoint/account
/// initialization, head adoption, signing key, listener, password or permission
/// is supplied. bootstrap_head remains initial provisioning data, not current CAS.
/// Every partially acquired owner is explicitly closed; all independent close
/// causes survive. Expected business head comes solely from the protected document.
/// Actual human/OS confinement and immutable custody remain operator prerequisites.
pub fn open(ctx: &Context, path: &str, pin: &str) -> Result<(AuthenticatedControl, Head), Error> {
    open_inner(ctx, path, pin, Hooks::default())
}
/// Disposable post-close EIO injection only; absent from the normal profile.
#[derive(Default)]
struct Hooks {
    #[cfg(feature = "test-fixtures")]
    document_close: bool,
}
/// Same real source path with private-document close fault. Requires an isolated
/// valid v3fixture; never a production policy, authorization or live-state tool.
/// Actual descriptors still close before injected EIO is retained with primary.
#[cfg(feature = "test-fixtures")]
pub fn open_with_document_close_fault(
    ctx: &Context,
    path: &str,
    pin: &str,
) -> Result<(AuthenticatedControl, Head), Error> {
    open_inner(
        ctx,
        path,
        pin,
        Hooks {
            document_close: true,
        },
    )
}
/// Own each opened resource until transfer, explicitly close after a failed
/// prerequisite and preserve both typed primary and independent close branches.
fn open_inner(
    ctx: &Context,
    path: &str,
    pin: &str,
    hooks: Hooks,
) -> Result<(AuthenticatedControl, Head), Error> {
    let d = bootstrap::load_authenticated(path, pin).map_err(Error::Bootstrap)?;
    open_document(ctx, d, hooks)
}
/// Open one already validated protected declaration. Existing partial-cleanup
/// contracts remain shared; never expose this as a data-only/request constructor.
fn open_document(
    ctx: &Context,
    d: AuthenticatedDocument,
    hooks: Hooks,
) -> Result<(AuthenticatedControl, Head), Error> {
    let business = startup::resolve(&d.control).map_err(Error::BusinessDefinition)?;
    let (expected, key, project) = personal_inputs(&d)?;
    let source = personal::Source::open(ctx, &d.personal.journal_path, &key, project, expected)
        .map_err(Error::Personal)?;
    let documents = match Documents::open(
        Path::new(&d.personal.checkpoint_directory),
        d.control.installation.service_uid,
    ) {
        Ok(documents) => documents,
        Err(cause) => {
            return Err(match source.close() {
                Ok(()) => Error::Storage(cause),
                Err(cleanup) => Error::Cleanup {
                    cause: Box::new(Error::Storage(cause)),
                    source: Some(cleanup),
                    coordinator: None,
                },
            })
        }
    };
    #[cfg(feature = "test-fixtures")]
    if hooks.document_close {
        documents.fixture_faults(&[config::store::FixtureFault::StoreClose]);
    }
    let _ = hooks;
    let personal = Coordinator::new(source, documents);
    let control = match Control::open(ctx, business) {
        Ok(control) => control,
        Err(cause) => {
            return Err(match personal.close() {
                Ok(()) => Error::Business(cause),
                Err(cleanup) => Error::Cleanup {
                    cause: Box::new(Error::Business(cause)),
                    source: None,
                    coordinator: Some(cleanup),
                },
            })
        }
    };
    Ok((
        AuthenticatedControl::new(personal, control),
        d.control.authority_head,
    ))
}

/// Load one independent pinned v3 declaration, then its explicitly selected private
/// signing snapshot BEFORE opening journals/locks. Path and pin are protected
/// startup input, never fields from an enrollment request. New owner contains
/// preset initial personal/business heads; initializes/enrolls nothing implicitly.
/// Signer load failure creates no journal/lock/checkpoint. Later open failures
/// preserve existing cleanup branches and drop/zeroize signer. No listener, human
/// proof, actual OS separation, password generation or head adoption is supplied.
pub fn open_lifecycle(
    ctx: &Context,
    path: &str,
    pin: &str,
    signer_path: &Path,
) -> Result<PersonalLifecycle, Error> {
    open_lifecycle_inner(ctx, path, pin, signer_path, Hooks::default())
}
/// Disposable actual-document-close fault through the same lifecycle startup.
/// No production surface; valid isolated fixture required. Descriptors really
/// close before injected EIO, returned close branches retain prior known effects.
#[cfg(feature = "test-fixtures")]
pub fn open_lifecycle_with_document_close_fault(
    ctx: &Context,
    path: &str,
    pin: &str,
    signer_path: &Path,
) -> Result<PersonalLifecycle, Error> {
    open_lifecycle_inner(
        ctx,
        path,
        pin,
        signer_path,
        Hooks {
            document_close: true,
        },
    )
}
/// Read declaration once, load pinned key first and reuse actual two-source opens.
/// Signer ownership drops on any later refusal; no duplicate bootstrap read/race.
/// Retain controller UID from this same pinned declaration, not a later request.
fn open_lifecycle_inner(
    ctx: &Context,
    path: &str,
    pin: &str,
    signer_path: &Path,
    hooks: Hooks,
) -> Result<PersonalLifecycle, Error> {
    let d = bootstrap::load_authenticated(path, pin).map_err(Error::Bootstrap)?;
    let (_, key, _) = personal_inputs(&d)?;
    let signer = protected_signer::load(signer_path, key).map_err(Error::Signing)?;
    let initial = d.personal.bootstrap_head.clone();
    let controller_uid = d.control.installation.controller_uid;
    let (app, business) = open_document(ctx, d, hooks)?;
    Ok(PersonalLifecycle::new(
        app,
        signer,
        initial,
        business,
        controller_uid,
    ))
}
