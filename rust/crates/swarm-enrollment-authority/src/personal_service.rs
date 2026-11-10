//! Trusted personal-service core without a listener, peer admission, session or
//! OS protection. Trusted dispatch requires caller admission; controller_packet
//! binds an actual packet to a retained independently approved namespace anchor.
pub mod controller_packet;
#[cfg(target_env = "gnu")]
pub mod exchange;
use crate::{
    authenticated_control::{self, Context, Outcome},
    authenticated_startup,
    personal::coordinator::{Account, CommitError},
    personal_lifecycle::PersonalLifecycle,
    service_request::{Operation, Request},
};
use std::{error::Error as StdError, fmt, path::Path};
use subtle::ConstantTimeEq;

/// Complete typed operation outcome; inner/outer storage and after-use errors
/// remain intact. This is private service data, never serialized as authority.
pub enum OutcomeData {
    Initialized(Result<(), CommitError>),
    Account(Result<Account, CommitError>),
    Control(Outcome),
}
impl fmt::Debug for OutcomeData {
    /// Avoid private response/configuration/account/cause data in diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalServiceOutcome { <private> }")
    }
}
/// Echoed correlation and exact typed outcome; no password/proof or secret hash.
/// A challenge echo does not establish freshness, delivery or authorization.
pub struct Reply {
    pub challenge: [u8; 32],
    pub outcome: OutcomeData,
    operation: u8,
    request_digest: String,
}
impl Reply {
    /// Borrow every independent business failure, including AfterUse's known
    /// inner failure beside its authority error. No cloning, formatting, rollback
    /// or business success inferred; callers still inspect the typed outcome.
    pub fn error_branches(&self) -> Vec<&(dyn StdError + 'static)> {
        use crate::personal::coordinator::UseError;
        let mut out: Vec<&(dyn StdError + 'static)> = Vec::new();
        match &self.outcome {
            OutcomeData::Initialized(Err(e)) | OutcomeData::Account(Err(e)) => out.push(e),
            OutcomeData::Control(Ok(Err(e))) => out.push(e),
            OutcomeData::Control(Err(e)) => {
                out.push(e);
                if let UseError::AfterUse {
                    outcome: Err(inner),
                    ..
                } = e
                {
                    out.push(inner)
                }
            }
            _ => {}
        }
        out
    }

    /// Closed opcode retained from the validated request, not outcome inference.
    pub fn operation(&self) -> u8 {
        self.operation
    }
    /// Exact public request binding only; no proof/password or credential hash.
    pub fn request_digest(&self) -> &str {
        &self.request_digest
    }
}
impl fmt::Debug for Reply {
    /// Correlation and private outcome stay out of ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalServiceReply { <private> }")
    }
}
/// Startup/binding refusal before dispatch. Operation failures are preserved in
/// Reply, never converted to an empty successful business result.
pub enum Error {
    Startup(authenticated_startup::Error),
    Target,
}
impl Error {
    /// Safe static category; not an admission verdict or private cause string.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Startup(_) => "service-startup",
            Self::Target => "service-target",
        }
    }
}
impl fmt::Display for Error {
    /// No path, pin, key or nested private startup data in formatted errors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal service: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, including failed protected startup.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Actual startup cause including its independent typed cleanup branches.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Startup(e) => Some(e),
            Self::Target => None,
        }
    }
}

/// Kernel-subject precondition refusal, never a personal/business verdict.
/// Actual namespace/partial-cleanup causes remain typed; no UID is formatted.
pub enum ControllerError {
    Uid,
    Namespaces(swarm_linux_identity_channel::Error),
}
impl ControllerError {
    /// Static safe category; absence of a namespace cause does not admit a peer.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Uid => "service-controller-uid",
            Self::Namespaces(_) => "service-controller-namespaces",
        }
    }
    /// Complete namespace error, including every independent partial-close cause.
    /// Consumers must inspect its causes, not only the first StdError source.
    pub fn channel_error(&self) -> Option<&swarm_linux_identity_channel::Error> {
        match self {
            Self::Namespaces(error) => Some(error),
            Self::Uid => None,
        }
    }
}
impl fmt::Display for ControllerError {
    /// Omit credentials, namespace values, packet data and nested IO text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for ControllerError {
    /// Same private-data redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for ControllerError {
    /// Original namespace cause; its separate cleanup causes remain inspectable.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.channel_error()
            .map(|error| error as &(dyn StdError + 'static))
    }
}

/// Own one pinned startup lifecycle. No Clone/Serialize, request-selected source,
/// actor, key or authority. Mutable dispatch serializes this owner only; actual
/// source locks/current account/business grant still protect commitment.
pub struct Service {
    lifecycle: PersonalLifecycle,
    pin: String,
}
impl fmt::Debug for Service {
    /// Never expose protected startup paths, key or expected pin.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalService { <private> }")
    }
}
impl Service {
    /// Open actual independently pinned existing sources and signing snapshot.
    /// Starts no service/worker, initializes nothing and admits no peer. Caller
    /// must retain installer durability acknowledgement before first use and keep
    /// this owner/inputs outside the agent domain; this API proves neither that
    /// custody nor physical reconciliation of later account state. Partial-open
    /// causes/cleanup remain in the shared startup error; no retry or adoption.
    pub fn open(
        ctx: &swarm_enrollment_state::Context,
        path: &str,
        pin: &str,
        signer: &Path,
    ) -> Result<Self, Error> {
        let lifecycle = authenticated_startup::open_lifecycle(ctx, path, pin, signer)
            .map_err(Error::Startup)?;
        Ok(Self {
            lifecycle,
            pin: pin.to_owned(),
        })
    }
    /// Immutable controller UID from the same protected startup declaration.
    /// No IO or request-selected value. A transport must compare the actual kernel
    /// subject and independently admit its domain/rights; equality alone proves
    /// neither an existing account, a human, an image nor a current mandate.
    pub fn controller_uid(&self) -> u32 {
        self.lifecycle.controller_uid()
    }
    /// Compare the kernel packet subject to the retained controller role before
    /// any payload decoding or business operation, then observe namespaces through
    /// its held pidfd. No packet ownership/closure transfer or source mutation.
    /// Caller owns Packet and returned namespace closes, and must retain both
    /// through independent admission/use. Namespace equality, UID and this success
    /// are only observations: no image, mandate, human or current rights admitted.
    /// Wrong UID refuses without namespace lookup; actual namespace/partial-close
    /// causes remain intact. There is no declared-PID or connection-peer fallback.
    pub fn controller_namespaces(
        &self,
        packet: &swarm_linux_identity_channel::Packet,
    ) -> Result<swarm_linux_identity_channel::Namespaces, ControllerError> {
        if packet.credentials().1 != self.controller_uid() {
            return Err(ControllerError::Uid);
        }
        packet.namespaces().map_err(ControllerError::Namespaces)
    }
    /// Dispatch one already-admitted controller request after constant-time target
    /// comparison to protected startup, before any lifecycle operation. The request
    /// supplies no actor/clock/head/key; control rechecks current personal knowledge
    /// and business authority through commitment. Preserve complete operation
    /// errors/known effects in Reply. No automatic init/recovery/retry, session,
    /// replay cache, peer authentication, elapsed-time or concurrency policy.
    pub fn dispatch(&mut self, ctx: &Context, request: Request<'_>) -> Result<Reply, Error> {
        if !bool::from(request.target_pin().as_bytes().ct_eq(self.pin.as_bytes())) {
            return Err(Error::Target);
        }
        let challenge = *request.challenge();
        let operation = request.operation().code();
        let request_digest = request.public_digest();
        let outcome = match request.into_operation() {
            Operation::Initialize => OutcomeData::Initialized(self.lifecycle.initialize(ctx)),
            Operation::Enroll { proof, password } => {
                OutcomeData::Account(self.lifecycle.enroll(ctx, proof, password))
            }
            Operation::Recover => OutcomeData::Account(self.lifecycle.recover(ctx)),
            Operation::Control { password, request } => {
                OutcomeData::Control(self.lifecycle.execute(ctx, password, request))
            }
        };
        Ok(Reply {
            challenge,
            outcome,
            operation,
            request_digest,
        })
    }
    /// Check an actual kernel packet against the retained bootstrap UID and an
    /// independently approved namespace anchor before decoding and dispatch.
    /// The caller protects/approves the expected domain and keeps it and Packet
    /// alive through this call, then explicitly closes both. No first-caller
    /// adoption, actor/clock override, listener, current-rights bypass or retry.
    /// Observed namespace handles remain held through operation and close after;
    /// a late close refusal retains the complete Reply in controller_packet::Error.
    /// Success proves local binding only, not human/image/confinement approval.
    /// Namespace handles retain the observed objects, not a freeze of the peer:
    /// the peer may subsequently exit or change namespaces. No currentness lease
    /// or prevention of setns/unshare is established by this point-in-time check.
    pub fn dispatch_controller_packet(
        &mut self,
        ctx: &Context,
        packet: &swarm_linux_identity_channel::Packet,
        expected: &swarm_linux_identity_channel::Namespaces,
    ) -> Result<Reply, controller_packet::Error> {
        controller_packet::run(self.controller_uid(), packet, expected, |request| {
            self.dispatch(ctx, request)
        })
    }
    /// Require actual sender binding to a separately captured approved Child in
    /// addition to the retained bootstrap UID and namespace-domain checks.
    /// Caller owns launch approval/reaper custody and protects/retains ProcessAnchor,
    /// expected Namespaces and Packet through this operation and their closes.
    /// UID refusal precedes process observation; process mismatch/exit precedes
    /// namespace or payload work. Matching retains ALL namespace58/target/current
    /// business checks and late-close Reply/error behavior. No listener, new
    /// image/human/confinement/lease claim or automatic retry is introduced.
    pub fn dispatch_spawned_controller_packet(
        &mut self,
        ctx: &Context,
        packet: &swarm_linux_identity_channel::Packet,
        process: &swarm_linux_identity_channel::ProcessAnchor,
        expected: &swarm_linux_identity_channel::Namespaces,
    ) -> Result<Reply, controller_packet::Error> {
        controller_packet::run_spawned(self.controller_uid(), packet, process, expected, |r| {
            self.dispatch(ctx, r)
        })
    }
    /// Require a natively atomically-created controller owner, not a conditional
    /// numeric-PID capture. Borrow this owner/Packet/approved expected namespaces
    /// through ALL existing UID/process/domain/decode/target/current-control steps.
    /// Caller owns launch approval, credentials/policy and explicit wait/close.
    /// A fast exit still retains ownership; this operation refuses its stale
    /// process, preserving original errors/known Reply exactly as spawned dispatch.
    /// No image/human/confinement, daemon or separate-UID success is inferred.
    #[cfg(target_env = "gnu")]
    pub fn dispatch_atomic_controller_packet(
        &mut self,
        ctx: &Context,
        packet: &swarm_linux_identity_channel::Packet,
        controller: &swarm_linux_identity_channel::atomic_spawn::Spawned,
        expected: &swarm_linux_identity_channel::Namespaces,
    ) -> Result<Reply, controller_packet::Error> {
        controller_packet::run_atomic(self.controller_uid(), packet, controller, expected, |r| {
            self.dispatch(ctx, r)
        })
    }
    /// Receive one guarded packet, apply the unchanged strict atomic controller
    /// gate, explicitly close the packet then send one canonical response on the
    /// same channel. Any dispatch/close refusal suppresses output and retains all
    /// known results/causes; output errors retain their original Reply/observation.
    /// Caller owns buffer capacity, socket flags, cancellation and approved launch/
    /// expected domain. No listener, loop, retry, automatic init/recovery, policy,
    /// peer receipt, human/confinement or actual separate-UID proof is created.
    /// Success is transport completion only: inspect Sent.reply business outcome.
    #[cfg(target_env = "gnu")]
    pub fn exchange_atomic_controller_packet(
        &mut self,
        ctx: &Context,
        channel: &mut swarm_linux_identity_channel::Channel<'_>,
        buffer: zeroize::Zeroizing<Vec<u8>>,
        controller: &swarm_linux_identity_channel::atomic_spawn::Spawned,
        expected: &swarm_linux_identity_channel::Namespaces,
    ) -> Result<crate::service_response::Sent, exchange::Error> {
        exchange::run(channel, buffer, |packet| {
            self.dispatch_atomic_controller_packet(ctx, packet, controller, expected)
        })
    }
    /// Consume the owner and attempt both existing closes; signer drops/zeroizes
    /// even on refusal. Complete close branches and earlier operation effects
    /// remain caller-owned. Explicit protected reopen is separate from retry.
    pub fn close(self) -> Result<(), authenticated_control::CloseError> {
        self.lifecycle.close()
    }
}
