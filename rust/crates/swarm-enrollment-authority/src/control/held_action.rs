//! One held exact-permission gate shared by typed Trust and PAEA capabilities.
//! Crate-only entry requires a proof borrowed from the current personal account.
use super::trust_action::Use;
use super::{validate_record, validate_time, Control, Error};
use crate::{credentials, Document};
use swarm_enrollment_state::{Context, Head};
use swarm_identity_contract::Permission;
use time::OffsetDateTime;

impl Control {
    /// Hold the configured business record through exactly one callback and
    /// completion. Validate current personal binding, signed record, permission
    /// and time before use, and expiry after use. No retry or rollback. Retain
    /// callback outcomes even if completion or after-use validation fails.
    pub(super) fn exact_action<T, E>(
        &self,
        proof: &credentials::Proof<'_>,
        ctx: &Context,
        head: &Head,
        permission: &Permission,
        f: impl FnOnce(&Document) -> Result<T, E>,
    ) -> Use<T, E> {
        let mut action = None;
        let mut refusal = None;
        if proof.principal() != &self.binding.principal
            || proof.installation_digest() != self.installation
        {
            return Use {
                authority: Err(Error::PersonalBinding),
                action,
            };
        }
        let _guard = match self.lock() {
            Ok(g) => g,
            Err(e) => {
                return Use {
                    authority: Err(e),
                    action,
                }
            }
        };
        let authority =
            self.authority
                .with_active(ctx, head, &self.binding.enrollment_id, |record| {
                    let checked = (|| {
                        let document = validate_record(&record, &self.binding, &self.project)?;
                        validate_time(&document, OffsetDateTime::now_utc())?;
                        if !document.permissions.contains(permission) {
                            return Err(Error::Permission);
                        }
                        action = Some(f(&document));
                        validate_time(&document, OffsetDateTime::now_utc())
                    })();
                    refusal = checked.err();
                    Ok(())
                });
        let authority = match (authority, refusal) {
            (Err(cause), Some(refusal)) => Err(Error::AuthorityRefusal {
                cause,
                refusal: Box::new(refusal),
            }),
            (Err(cause), None) => Err(Error::Authority(cause)),
            (Ok(()), Some(error)) => Err(error),
            (Ok(()), None) => Ok(()),
        };
        Use { authority, action }
    }
}
