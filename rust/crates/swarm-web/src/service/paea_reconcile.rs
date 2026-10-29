//! Session-protected read-only PA-09 effect-journal reconciliation of the
//! service Root against the saved separate settlement ledger Root.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};

impl NativeHttpService {
    /// GET/HEAD, empty body/query, after the ordinary session/Host/CSRF gate.
    /// Read the saved separate-ledger root path from the service Root's
    /// persisted settings, then run the engine's read-only reconciliation with
    /// BOTH roots opened here (`false` = read-only) and the service binding
    /// checked. The engine consumer itself enforces two physically distinct
    /// roots (`paea-effect-reconcile-separate-ledger-required`); this route
    /// adds no authority: no effect, no policy mutation, no ledger
    /// initialization, no acceptance. The configured path designates nothing
    /// by itself; absence from the saved document refuses as
    /// `settings-paea-ledger-root-*`. Both closes are paired through the
    /// established `Close` error tree, the report is withheld when a close
    /// fails, and no partial report, repair, replay or journal mutation occurs.
    /// `verified=true` in the report is recomputed correspondence, never
    /// authenticity or acceptance.
    pub(super) fn reconcile_effects(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !request.body().is_empty() {
            return Err(E::Refused("paea-reconcile-body-invalid"));
        }
        if request.uri().query().is_some_and(|v| !v.is_empty()) {
            return Err(E::Refused("paea-reconcile-query-invalid"));
        }
        let service = swarm_engine::RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        close(self.reconcile_with(&service), service.close())
    }
    /// Reconcile against the saved ledger root with the ledger opened/closed
    /// and its close failure paired ahead of the service close. Typed causes
    /// survive; the report is withheld when any close fails.
    fn reconcile_with(&self, service: &swarm_engine::RootStore) -> Result<RouteBody, E> {
        self.check_binding(service)?;
        let ledger_path = service
            .settings()
            .map_err(|e| root("settings-read", e))?
            .values
            .paea_settlement_ledger_policy()
            .map_err(|e| root("settings-ledger-root", swarm_engine::RootStoreError::Settings(e)))?
            .to_owned();
        let ledger = swarm_engine::RootStore::open(std::path::Path::new(&ledger_path), false)
            .map_err(|e| root("ledger-open", e))?;
        close(
            service
                .paea_reconcile_effect_journals(&ledger)
                .map_err(|e| root("reconcile", e))
                .and_then(|report| {
                    swarm_engine::encode_model(&report)
                        .map(RouteBody::plain)
                        .map_err(E::Model)
                }),
            ledger.close(),
        )
    }
}
