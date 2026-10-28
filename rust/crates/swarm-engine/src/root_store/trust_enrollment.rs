//! Exact held enrollment consumer; no account creation, transport or local fallback.
use super::{finished, RootBinding, RootStore, RootStoreError as R};
use crate::{
    trust::{self, authentication::Source},
    IdentityPlanProjection, TrustReply,
};
use swarm_enrollment_authority::trust_action::{Identity, Operation, Request};

/// Owned storage witness and fresh exact permission request. It must stay alive
/// through signing and use; closing it invalidates reuse of this object. The
/// random binding prevents a newly opened object adopting an old signed request.
/// This is not an identity or an operator grant. Call close for checked release.
pub struct TrustEnrollmentRequest {
    binding: RootBinding,
    request: Request,
}
impl TrustEnrollmentRequest {
    /// Check this retained physical datastore before a transport can publish its
    /// session. Grants no identity, role or permission; no business mutation occurs; descriptor metadata is read.
    /// The caller must retain the same opened Root through transport binding.
    pub fn check_root(&self, root: &RootStore) -> Result<(), R> {
        if self.binding.matches(root)? { Ok(()) } else { Err(R::Refused("trust-enrollment-root")) }
    }
    /// Data to include in a separately authorized signed enrollment. Reading it
    /// authenticates nobody and cannot mutate the Root or issue an enrollment.
    pub fn request(&self) -> &Request {
        &self.request
    }
    /// Close both retained storage witnesses, preserving their independent errors.
    pub fn close(self) -> Result<(), R> {
        self.binding.close()
    }
}
impl RootStore {
    /// Derive exact bytes/operation and a fresh physical Root binding. No grant,
    /// plan migration or business mutation occurs. The owner must close the
    /// returned witness even when enrollment provisioning or authentication fails.
    pub fn trust_enrollment_request(
        &self,
        operation: Operation,
        raw: &[u8],
    ) -> Result<TrustEnrollmentRequest, R> {
        let nonce = crate::generate_identifier("trust-enrollment-").map_err(R::Runtime)?;
        let binding = self.bind_root()?;
        let result = (|| {
            let root=trust::digest(&serde_json::json!({"domain":"trust-enrollment-root-v1","instance":binding.instance()?,"nonce":nonce}))
                .map_err(|e|R::Trust(Box::new(e)))?;
            Request::new(&root, operation, &trust::hash(raw))
                .map_err(|_| R::Refused("trust-enrollment-request"))
        })();
        match result {
            Ok(request) => Ok(TrustEnrollmentRequest { binding, request }),
            Err(e) => finished(Err(e), binding.close()),
        }
    }
    /// Consume only the library's held current authority, matched to this owned
    /// request, Root instance, exact bytes, current SQL plan and Trust revision.
    /// SQL reservation spans file publication. A late SQL error retains a known
    /// business reply/publication; no error implies rollback of the Trust file.
    pub fn trust_as_enrolled(
        &self,
        request: &TrustEnrollmentRequest,
        identity: &Identity<'_>,
        raw: &[u8],
    ) -> Result<TrustReply, R> {
        let actual = identity.request();
        if actual.root_digest() != request.request.root_digest()
            || actual.operation() != request.request.operation()
            || actual.request_digest() != request.request.request_digest()
            || actual.request_digest() != trust::hash(raw)
            || identity.principal().kind != "human"
            || !request.binding.matches(self)?
        {
            return Err(R::Refused("trust-enrollment-binding"));
        }
        let tx = rusqlite::Transaction::new_unchecked(
            &self.db,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(R::Sql)?;
        let result = (|| {
            let scope = identity.scope();
            let version: i64 = tx
                .query_row(
                    "SELECT projection_version FROM main.agent_identity_plan_heads WHERE work_id=?",
                    [&scope.work_id],
                    |r| r.get(0),
                )
                .map_err(R::Sql)?;
            let projection = match version {
                1 => IdentityPlanProjection::V1,
                2 => IdentityPlanProjection::V2,
                _ => return Err(R::Refused("trust-enrollment-plan-projection")),
            };
            let stamp = crate::read_identity_plan_stamp(&tx, &scope.work_id, projection)
                .map_err(|e| R::SqlTransaction(Box::new(e)))?;
            if !stamp.matches_scope(scope) {
                return Err(R::Refused("trust-enrollment-plan"));
            }
            self.trust_scoped(
                actual.operation().as_str(),
                raw,
                Source::Enrolled(identity),
                true,
            )
        })();
        // No SQL writes were made. Explicit rollback releases the reservation;
        // its failure remains separate from any already published Trust reply.
        let publication = result.as_ref().ok().and_then(|r| r.publication);
        let cleanup = tx.rollback().map_err(R::Sql);
        finished(result, cleanup).map_err(|e| match publication {
            Some(publication) => R::TrustPublished {
                primary: Box::new(e),
                publication,
            },
            None => e,
        })
    }
}
