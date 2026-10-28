//! Pinned startup declaration and point-in-time current-source observation.
//! The independent digest/checkpoint and actual human enrollment are operator
//! prerequisites. A successful check is never a transferable action permit.
use crate::control::{self, Binding, Control, Startup};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::Serialize;
use std::{error::Error as StdError, fmt};
use swarm_enrollment_bootstrap::{self as bootstrap, ControlDocument};
use swarm_enrollment_state::{self as state, BoundJournal, Context, Head};
use time::OffsetDateTime;

/// Primary refusal plus a possible independent SQLite close failure. Diagnostic
/// rendering never includes private paths or payloads; typed causes remain usable.
#[derive(Debug)]
pub enum Error {
    Bootstrap(bootstrap::Error),
    Authority(state::Error),
    Control(control::Error),
    Key(base64::DecodeError),
    Project(std::num::ParseIntError),
    Joined {
        cause: Box<Error>,
        cleanup: state::Error,
    },
}
impl Error {
    /// Stable diagnostic code; Joined keeps the original refusal's category.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Bootstrap(e) => e.code(),
            Self::Authority(e) => e.code(),
            Self::Control(e) => e.code(),
            Self::Key(_) | Self::Project(_) => "binding",
            Self::Joined { cause, .. } => cause.code(),
        }
    }
    /// Secondary explicit-close failure, distinct from the primary source chain.
    pub fn cleanup_error(&self) -> Option<&state::Error> {
        match self {
            Self::Joined { cleanup, .. } => Some(cleanup),
            _ => None,
        }
    }
}
impl fmt::Display for Error {
    /// Safe category only; original typed errors are preserved outside this text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "enrollment startup check: {}", self.code())
    }
}
impl StdError for Error {
    /// Primary cause only. Call cleanup_error as well when traversing Joined.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Bootstrap(e) => Some(e),
            Self::Authority(e) => Some(e),
            Self::Control(e) => Some(e),
            Self::Key(e) => Some(e),
            Self::Project(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
        }
    }
}
/// Point-in-time observation, serialized only after authority COMMIT and close.
/// Neither the declaration's labels nor this report establish a human session,
/// confinement, an authorized business request or an active authority service.
#[derive(Debug, Serialize)]
pub struct Check {
    pub schema_version: i64,
    pub bootstrap_digest: String,
    pub declaration: ControlDocument,
    #[serde(serialize_with = "swarm_wire_json::timestamp::serialize")]
    pub checked_at: OffsetDateTime,
    pub current_enrollment_verified: bool,
    pub human_authentication_verified: bool,
    pub confinement_verified: bool,
    pub request_authorized: bool,
    pub service_started: bool,
}
/// Convert an already validated declaration to core inputs, without learning
/// any value from the journal. The caller remains responsible for independent
/// pin provenance. Errors retain invalid key/project encoding causes.
pub(crate) fn resolve(d: &ControlDocument) -> Result<Startup, Error> {
    let key: [u8; 32] = URL_SAFE_NO_PAD
        .decode(&d.installation.pins.public_key)
        .map_err(Error::Key)?
        .try_into()
        .map_err(|_| Error::Control(control::Error::Binding))?;
    let mut project = [0u8; 32];
    for (i, byte) in project.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&d.project_context_digest[i * 2..i * 2 + 2], 16)
            .map_err(Error::Project)?;
    }
    Ok(Startup {
        journal: d.journal_path.clone().into(),
        authority_key: key,
        project,
        configuration_directory: d.installation.configuration_directory.clone().into(),
        witness_directory: d.installation.witness_directory.clone().into(),
        owner: d.installation.service_uid,
        pins: d.installation.pins.clone(),
        binding: Binding {
            enrollment_id: d.enrollment_id.clone(),
            generation: d.generation,
            principal: d.installation.pins.principal.clone(),
            scope: d.scope.clone(),
            trust_revision: d.trust_configuration_revision,
        },
    })
}
/// Load a protected pinned declaration and open the production core, returning
/// its independently declared checkpoint for subsequent execute calls. Opening
/// may create Coordinator lockfiles, but neither grants rights nor starts a
/// listener; only a later held current-authority operation can change config.
/// Protected bootstrap/ancestors and actual human authentication are not supplied
/// by this API. All open errors retain their concrete causes.
pub fn open(ctx: &Context, path: &str, pin: &str) -> Result<(Control, Head), Error> {
    let d = bootstrap::load_control(path, pin).map_err(Error::Bootstrap)?;
    let inputs = resolve(&d)?;
    let c = Control::open(ctx, inputs).map_err(Error::Control)?;
    Ok((c, d.authority_head))
}
/// Check protected v2 inputs and the actual signed active record at the exact
/// independently approved checkpoint. Hold its reservation through validation,
/// COMMIT and then explicit close. No Coordinator/project/configuration is opened;
/// no events/grants are written. SQLite may manage its own WAL/SHM sidecars.
/// Human authentication, request authorization and confinement stay unverified.
/// A stale/revoked/missing source refuses; never relearn a checkpoint on failure.
pub fn check(ctx: &Context, path: &str, pin: &str) -> Result<Check, Error> {
    let d = bootstrap::load_control(path, pin).map_err(Error::Bootstrap)?;
    let inputs = resolve(&d)?;
    let authority = BoundJournal::open(ctx, &inputs.journal, &inputs.authority_key, inputs.project)
        .map_err(Error::Authority)?;
    let mut checked = None;
    let mut refusal = None;
    let result = authority.with_active(
        ctx,
        &d.authority_head,
        &inputs.binding.enrollment_id,
        |record| {
            let check = (|| {
                let document =
                    control::validate_record(&record, &inputs.binding, &d.project_context_digest)?;
                let at = OffsetDateTime::now_utc();
                control::validate_time(&document, at)?;
                checked = Some(at);
                Ok(())
            })();
            refusal = check.err();
            Ok(())
        },
    );
    let result = match (result, refusal) {
        (Ok(()), None) => checked.ok_or(Error::Control(control::Error::State)),
        (Ok(()), Some(e)) => Err(Error::Control(e)),
        (Err(e), None) => Err(Error::Authority(e)),
        (Err(cause), Some(refusal)) => Err(Error::Control(control::Error::AuthorityRefusal {
            cause,
            refusal: Box::new(refusal),
        })),
    };
    let checked_at = match (result, authority.close()) {
        (Ok(v), Ok(())) => v,
        (Err(e), Ok(())) => return Err(e),
        (Ok(_), Err(e)) => return Err(Error::Authority(e)),
        (Err(cause), Err(cleanup)) => {
            return Err(Error::Joined {
                cause: Box::new(cause),
                cleanup,
            })
        }
    };
    Ok(Check {
        schema_version: 1,
        bootstrap_digest: pin.into(),
        declaration: d,
        checked_at,
        current_enrollment_verified: true,
        human_authentication_verified: false,
        confinement_verified: false,
        request_authorized: false,
        service_started: false,
    })
}
