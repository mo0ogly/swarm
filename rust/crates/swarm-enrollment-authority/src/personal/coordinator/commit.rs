//! Enrollment ordering: durable exact intent -> signed atomic journal -> durable
//! confirmed checkpoint. File reservation never crosses an external business use.
use super::*;
impl Coordinator {
    /// Explicit protected provisioning only: validate positive independently
    /// supplied bootstrap head against real active signed context, then create
    /// the absent private witness. Never read/adopt a head from the database or
    /// replace existing state. No secret/password is checked, account consumed or
    /// human presence established. CommitError preserves checkpoint publication.
    pub fn initialize(&self, ctx: &Context, bootstrap_head: &Head) -> Result<(), CommitError> {
        let mut progress = Progress::default();
        let r = self
            .documents
            .reserve(&ctx.files)
            .map_err(|e| CommitError {
                cause: Error::Storage(e),
                progress: Progress::default(),
            })?;
        let result = (|| {
            if !bootstrap_head.valid() || bootstrap_head.sequence <= 0 {
                return Err(Error::Checkpoint);
            }
            match r.read(FILE) {
                Ok(_) => return Err(Error::Storage(config::Error::Exists)),
                Err(config::Error::NotConfigured) => (),
                Err(e) => return Err(Error::Storage(e)),
            }
            self.bootstrap_current(ctx, bootstrap_head)?;
            let raw =
                wire::encode(&self.bootstrap_checkpoint(bootstrap_head)).map_err(Error::Frame)?;
            match r.create(FILE, &raw) {
                Ok(p) => {
                    progress.checkpoint = p;
                    Ok(())
                }
                Err(e) => {
                    progress.checkpoint = e.publication;
                    Err(Error::Write(e))
                }
            }
        })();
        combined(result, r.finish().map_err(Error::Storage))
            .map_err(|cause| CommitError { cause, progress })
    }
    /// Verify actual installation proof and prepare once at the protected
    /// bootstrap head. Persist/fsync exact signed intent BEFORE journal apply;
    /// confirm independent checkpoint only after acknowledgement. The caller
    /// supplies the protected signing key, never request-provided key material.
    /// Existing pending state requires explicit recover, never fresh credentials.
    /// No callback/business effect or automatic retry is performed under FS lock.
    pub fn enroll(
        &self,
        ctx: &Context,
        key: &SigningKey,
        secret: &[u8; 32],
        password: &[u8],
    ) -> Result<Account, CommitError> {
        let mut progress = Progress::default();
        let r = self
            .documents
            .reserve(&ctx.files)
            .map_err(|e| CommitError {
                cause: Error::Storage(e),
                progress: Progress::default(),
            })?;
        let result = (|| {
            let mut loaded = self.load(ctx, &r)?;
            if loaded.prepared.is_some() {
                return Err(Error::Pending);
            }
            if loaded.checkpoint.credential_digest.is_some() {
                return Err(Error::Exists);
            }
            let p = self
                .source
                .prepare(&ctx.journal, &loaded.checkpoint.head, key, secret, password)
                .map_err(Error::Personal)?;
            let exact = self
                .source
                .encode_prepared(&ctx.journal, &p)
                .map_err(Error::Personal)?;
            loaded.checkpoint.pending = Some(URL_SAFE_NO_PAD.encode(exact));
            let pending = wire::encode(&loaded.checkpoint).map_err(Error::Frame)?;
            match r.replace(FILE, &loaded.raw, &pending) {
                Ok(p) => progress.intent = p,
                Err(e) => {
                    progress.intent = e.publication;
                    return Err(Error::Write(e));
                }
            }
            #[cfg(feature = "test-fixtures")]
            self.phase(FixturePhase::IntentDurable)?;
            self.complete(ctx, &r, loaded.checkpoint, &pending, &p, &mut progress)
        })();
        combined(result, r.finish().map_err(Error::Storage))
            .map_err(|cause| CommitError { cause, progress })
    }
    /// Recover ONLY an exact signed pending proposal from the protected witness,
    /// without another password/secret/signature/salt. Flush/readback of validated
    /// existing intent precedes replay. Completed recovery checks current account
    /// at its protected head; Observed is a read acknowledgement, not a new apply.
    /// Missing/corrupt/stale/divergent state refuses, never learns a database head.
    pub fn recover(&self, ctx: &Context) -> Result<Account, CommitError> {
        let mut progress = Progress::default();
        let r = self
            .documents
            .reserve(&ctx.files)
            .map_err(|e| CommitError {
                cause: Error::Storage(e),
                progress: Progress::default(),
            })?;
        let result = (|| {
            let loaded = self.load(ctx, &r)?;
            match loaded.prepared {
                Some(p) => {
                    r.sync().map_err(Error::Storage)?;
                    progress.intent = Publication::Durable;
                    self.complete(ctx, &r, loaded.checkpoint, &loaded.raw, &p, &mut progress)
                }
                None => {
                    if loaded.checkpoint.credential_digest.is_none() {
                        return Err(Error::NoPending);
                    }
                    let a = Self::account(&loaded.checkpoint)?;
                    r.sync().map_err(Error::Storage)?;
                    progress.checkpoint = Publication::Durable;
                    self.account_current(ctx, &a)?;
                    progress.journal = JournalOutcome::Observed(a.head.clone());
                    Ok(a)
                }
            }
        })();
        combined(result, r.finish().map_err(Error::Storage))
            .map_err(|cause| CommitError { cause, progress })
    }
    /// Apply exact protected bytes once and confirm them. Preserve independent
    /// journal acknowledgement and checkpoint publication even on framing/write,
    /// phase-hook or later release failure; no implicit compensation or reset.
    fn complete(
        &self,
        ctx: &Context,
        r: &store::documents::Reservation<'_>,
        mut checkpoint: Checkpoint,
        pending_raw: &[u8],
        p: &Prepared,
        progress: &mut Progress,
    ) -> Result<Account, Error> {
        progress.journal = JournalOutcome::Unknown;
        let head = self
            .source
            .enroll(&ctx.journal, p)
            .map_err(Error::Personal)?;
        progress.journal = JournalOutcome::Acknowledged(head.clone());
        if head != p.next {
            return Err(Error::Internal);
        }
        #[cfg(feature = "test-fixtures")]
        self.phase(FixturePhase::JournalApplied)?;
        checkpoint.head = head;
        checkpoint.credential_digest = Some(p.record_digest.clone());
        checkpoint.pending = None;
        let confirmed = wire::encode(&checkpoint).map_err(Error::Frame)?;
        match r.replace(FILE, pending_raw, &confirmed) {
            Ok(p) => progress.checkpoint = p,
            Err(e) => {
                progress.checkpoint = e.publication;
                return Err(Error::Write(e));
            }
        }
        #[cfg(feature = "test-fixtures")]
        self.phase(FixturePhase::CheckpointDurable)?;
        Self::account(&checkpoint)
    }
}
