//! In-process value substitution under a dedicated, ephemeral P02 mandate.
use super::{absent, finished, trust_store, RootStore, RootStoreError as R};
use crate::trust::{authentication, state::State, TrustError as E};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::{
    collections::{hash_map::Entry, HashMap},
    num::NonZeroUsize,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

/// Logical context chosen and verified by the trusted in-process caller.
/// Each identifier uses the Trust name schema. The opened Root binds the
/// actual datastore; these labels do not establish OS or network identity.
#[derive(Clone, PartialEq, Eq)]
pub struct P02Context {
    pub project: String,
    pub batch: String,
    pub audience: String,
}

/// Terms of one local resolution mandate. The issuer must choose a current
/// local Trust subject, a positive lifetime and explicit memory limits.
pub struct P02Grant {
    pub context: P02Context,
    pub resolver: String,
    pub lifetime: Duration,
    /// Total aliases issued, including revoked ones, to prevent reuse.
    pub max_associations: NonZeroUsize,
    pub max_value_bytes: NonZeroUsize,
}

/// One process-local batch, bound by ownership to the opened Root.
/// There is no serialization. Dropping the vault drops the mandate and zeroes
/// the owned value buffers. Values already delivered have their own lifetime.
pub struct P02Vault<'root> {
    root: &'root RootStore,
    grant: P02Grant,
    issuer: String,
    revision: i64,
    deadline: Instant,
    revoked: bool,
    values: HashMap<String, Option<Zeroizing<Vec<u8>>>>,
}

impl RootStore {
    /// Open an empty vault with a dedicated resolution mandate issued by an
    /// authenticated local I3 administrator. The resolver must already be a
    /// current local Trust identity on this Root. Terms are fixed until drop.
    /// Invalid terms, missing authority and storage errors refuse creation.
    /// This opens no provider, file of values, endpoint or business workflow.
    pub fn open_p02_vault(&self, grant: P02Grant, credential: &str) -> Result<P02Vault<'_>, R> {
        if ![
            &grant.context.project,
            &grant.context.batch,
            &grant.context.audience,
            &grant.resolver,
        ]
        .iter()
        .all(|name| swarm_identity_contract::name(name.as_bytes()))
            || grant.lifetime.is_zero()
        {
            return Err(trust_error(E::Invalid("p02-grant-terms")));
        }
        let (issuer, revision, deadline) =
            self.with_p02_authority(credential, |state, actor| {
                authentication::require_administrator(actor).map_err(trust_error)?;
                if !state.identities.iter().any(|identity| {
                    identity.subject == grant.resolver
                        && !identity.disabled
                        && identity.enrollment_principal.is_none()
                }) {
                    return Err(trust_error(E::Authorization));
                }
                let deadline = Instant::now()
                    .checked_add(grant.lifetime)
                    .ok_or_else(|| trust_error(E::Invalid("p02-grant-lifetime")))?;
                Ok((actor.subject.clone(), state.revision, deadline))
            })?;
        Ok(P02Vault {
            root: self,
            grant,
            issuer,
            revision,
            deadline,
            revoked: false,
            values: HashMap::new(),
        })
    }

    /// Observe supported current Trust state under the same reservation as
    /// configuration writers, using only the opened datastore. Run the
    /// decision before releasing the lock. Failed release discards the result
    /// and retains the primary failure, if any. No Trust state is written.
    fn with_p02_authority<T>(
        &self,
        credential: &str,
        decide: impl FnOnce(&State, &authentication::Actor) -> Result<T, R>,
    ) -> Result<T, R> {
        let initial = self.instance()?;
        let reservation = self.directory.reserve_trust(false).map_err(R::Storage)?;
        let result = (|| {
            if self.instance()? != initial {
                return Err(R::Refused("root-instance-changed"));
            }
            let raw = self
                .directory
                .read_bounded(trust_store::STATE, trust_store::MAX_STATE)
                .map_err(|error| {
                    if absent(&error) {
                        trust_error(E::Authorization)
                    } else {
                        R::Storage(error)
                    }
                })?;
            let state = State::decode(&raw).map_err(trust_error)?;
            state.validate().map_err(trust_error)?;
            state.require_supported().map_err(trust_error)?;
            let actor = authentication::authenticated(&state, credential).map_err(trust_error)?;
            decide(&state, &actor)
        })();
        finished(result, reservation.close().map_err(R::Storage))
    }
}

/// Preserve the original typed Trust diagnostic through the Root API.
fn trust_error(error: E) -> R {
    R::Trust(Box::new(error))
}

impl P02Vault<'_> {
    /// Check the latched Trust revision, exact logical context and monotonic
    /// deadline. A revoked, stale or out-of-scope mandate fails uniformly.
    fn require_current(&self, state: &State, context: &P02Context, now: Instant) -> Result<(), R> {
        if self.revoked
            || state.revision != self.revision
            || context != &self.grant.context
            || now >= self.deadline
        {
            return Err(trust_error(E::Data("p02-mandate-refused")));
        }
        Ok(())
    }

    /// Confirm that the original issuer still holds local I3/admin authority.
    /// Used for lifecycle operations. No administrative bypass of resolution.
    fn require_issuer(&self, actor: &authentication::Actor) -> Result<(), R> {
        authentication::require_administrator(actor).map_err(trust_error)?;
        if actor.subject != self.issuer {
            return Err(trust_error(E::Authorization));
        }
        Ok(())
    }

    /// Substitute a value before sending the field to an agent. The issuer
    /// supplies nonempty bytes within the declared bound. Equality is preserved
    /// inside this vault, while fresh associations use OS randomness. Capacity,
    /// entropy or authorization failure leaves the correspondence unchanged.
    pub fn tokenize(
        &mut self,
        context: &P02Context,
        value: &[u8],
        credential: &str,
    ) -> Result<String, R> {
        if value.is_empty() || value.len() > self.grant.max_value_bytes.get() {
            return Err(trust_error(E::Invalid("p02-value-size")));
        }
        let (token, is_new) = self.root.with_p02_authority(credential, |state, actor| {
            self.require_issuer(actor)?;
            self.require_current(state, context, Instant::now())?;
            if let Some((token, _)) = self.values.iter().find(|(_, bytes)| {
                bytes
                    .as_ref()
                    .is_some_and(|bytes| bytes.as_slice() == value)
            }) {
                return Ok((token.clone(), false));
            }
            if self.values.len() >= self.grant.max_associations.get() {
                return Err(R::Refused("p02-vault-capacity"));
            }
            // 32 bytes is the token wire representation, independent of policy.
            let mut randomness = [0; 32];
            getrandom::getrandom(&mut randomness)
                .map_err(|_| R::Refused("p02-entropy-unavailable"))?;
            Ok((URL_SAFE_NO_PAD.encode(randomness), true))
        })?;
        if is_new {
            match self.values.entry(token.clone()) {
                Entry::Vacant(slot) => {
                    slot.insert(Some(Zeroizing::new(value.to_vec())));
                }
                Entry::Occupied(_) => return Err(R::Refused("p02-token-collision")),
            }
        }
        Ok(token)
    }

    /// Return a protected copy to the exact authenticated local resolver.
    /// The lookup and copy are authorized inside the Trust read reservation.
    /// Missing or withdrawn aliases share a content-free refusal. The caller
    /// owns the returned buffer and must keep it out of logs and agent inputs.
    pub fn resolve(
        &self,
        context: &P02Context,
        token: &str,
        credential: &str,
    ) -> Result<Zeroizing<Vec<u8>>, R> {
        self.root.with_p02_authority(credential, |state, actor| {
            self.require_current(state, context, Instant::now())?;
            if actor.subject != self.grant.resolver {
                return Err(trust_error(E::Data("p02-mandate-refused")));
            }
            self.values
                .get(token)
                .and_then(Option::as_ref)
                .cloned()
                .ok_or_else(|| trust_error(E::Data("p02-mandate-refused")))
        })
    }

    /// Withdraw one alias and drop its owned value buffer. Only the current
    /// original issuer can do so, even after expiry or a Trust revision change.
    /// False means absent or already withdrawn. A tombstone prevents accidental
    /// reissue until the vault closes. Past resolutions retain their own copies.
    pub fn revoke(&mut self, token: &str, credential: &str) -> Result<bool, R> {
        self.root
            .with_p02_authority(credential, |_, actor| self.require_issuer(actor))?;
        Ok(self.values.get_mut(token).and_then(Option::take).is_some())
    }

    /// Revoke the entire mandate and clear its owned values. Repetition is safe.
    /// Current issuer authorization is required. On failure the caller can
    /// still drop the whole vault to discard its process-local contents.
    pub fn revoke_mandate(&mut self, credential: &str) -> Result<(), R> {
        self.root
            .with_p02_authority(credential, |_, actor| self.require_issuer(actor))?;
        self.revoked = true;
        self.values.clear();
        Ok(())
    }
}
