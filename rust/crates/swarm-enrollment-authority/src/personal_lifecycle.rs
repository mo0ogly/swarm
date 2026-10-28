//! Owned protected-startup personal lifecycle. No transport, key generation,
//! first-caller adoption, session, human-presence claim or OS confinement.
use crate::{
    authenticated_control::{self, AuthenticatedControl, Context, Outcome},
    control::Request,
    personal::coordinator::{Account, CommitError},
    protected_signer::ProtectedSigner,
};
use swarm_enrollment_state::Head;
/// Owns private signing snapshot, two-source control and independently pinned
/// initial personal/business heads. No Clone/Serialize or public constructor.
/// Password/installation-secret buffers are borrowed, caller owns their erasure.
pub struct PersonalLifecycle {
    app: AuthenticatedControl,
    signer: ProtectedSigner,
    bootstrap_head: Head,
    business_head: Head,
    controller_uid: u32,
}
impl PersonalLifecycle {
    /// Only validated protected startup supplies these owned resources. No IO or
    /// implicit enrollment, no head/key/actor/controller role derived from a request
    /// or database. The UID is immutable declaration data, not an admitted peer.
    pub(crate) fn new(
        app: AuthenticatedControl,
        signer: ProtectedSigner,
        bootstrap_head: Head,
        business_head: Head,
        controller_uid: u32,
    ) -> Self {
        Self {
            app,
            signer,
            bootstrap_head,
            business_head,
            controller_uid,
        }
    }
    /// Controller role retained from the same independently pinned startup read.
    /// No IO, caller override or actual account/peer/human authentication. This
    /// value is a required transport comparison, never sufficient authorization.
    pub(crate) fn controller_uid(&self) -> u32 {
        self.controller_uid
    }
    /// Explicit provisioning: create an absent private checkpoint only after
    /// real current signed bootstrap matches the initial pinned head. No secret,
    /// account or authority is created. Existing/corrupt/stale state refuses;
    /// CommitError preserves publication/cleanup observations, no automatic reset.
    pub fn initialize(&self, ctx: &Context) -> Result<(), CommitError> {
        self.app
            .personal
            .initialize(&ctx.personal, &self.bootstrap_head)
    }
    /// Observe the exact current signed bootstrap at the startup-pinned initial
    /// head, using the existing semantic/context validator and transaction. Creates
    /// no checkpoint/account, checks no password/proof and confers no authority.
    /// A progressed/revoked/stale source or cancellation refuses with all original
    /// journal/semantic completion causes. This point-in-time check is not a permit.
    pub fn check_bootstrap(
        &self,
        ctx: &Context,
    ) -> Result<(), crate::personal::coordinator::Error> {
        self.app
            .personal
            .bootstrap_current(&ctx.personal, &self.bootstrap_head)
    }
    /// Consume actual installation proof once and enroll the borrowed password
    /// using only this startup-owned key. Signed intent becomes durable before
    /// journal consumption. No client actor/key/head or automatic retry; pending
    /// state requires recover without new material. Costs come from signed
    /// bootstrap, not defaults. No actual human presence is established here.
    pub fn enroll(
        &self,
        ctx: &Context,
        installation_secret: &[u8; 32],
        password: &[u8],
    ) -> Result<Account, CommitError> {
        self.app.personal.enroll(
            &ctx.personal,
            self.signer.signing_key(),
            installation_secret,
            password,
        )
    }
    /// Recover only the exact protected signed pending intent, or observe a
    /// committed current account at its protected checkpoint. No password/seed,
    /// new signature or learned DB head; preserves all Progress/error branches.
    pub fn recover(&self, ctx: &Context) -> Result<Account, CommitError> {
        self.app.personal.recover(&ctx.personal)
    }
    /// Authenticate actual current personal account and business grant through
    /// storage commitment. Uses the independently pinned business CAS head, not
    /// a request-selected actor/head. Later journal changes require a refreshed
    /// protected declaration/reopen. Outcome retains nested known/unknown effects.
    pub fn execute(&self, ctx: &Context, password: &[u8], request: Request) -> Outcome {
        self.app
            .execute(ctx, &self.business_head, password, request)
    }
    /// Consume this owner, attempt both storage closes and drop/zeroize signer
    /// even when closing refuses. No callable owner remains; diagnose all close
    /// branches before explicit protected reopen. Earlier known effects persist.
    /// This cannot be called by moving an owner while it remains borrowed for use.
    pub fn close(self) -> Result<(), authenticated_control::CloseError> {
        let result = self.app.close();
        drop(self);
        result
    }
    /// Disposable phase interruption through the real coordinator, test-fixtures
    /// only. Never set during an operation/reenter the owner or use on live state.
    #[cfg(feature = "test-fixtures")]
    pub fn fixture_hook(&self, hook: Option<crate::personal::coordinator::FixtureHook>) {
        self.app.personal.fixture_hook(hook)
    }
}
