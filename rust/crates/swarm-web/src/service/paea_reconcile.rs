//! Session-protected read-only PA-09 effect-journal reconciliation of the
//! service Root against the saved separate settlement ledger Root.
use super::{close, reader_bearer_token, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{ReaderVerdict, RootStore};

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
    ///
    /// BLP reader frontier (BLP-RET-05 / PA-09): the reconciliation report is
    /// classified material — its findings disclose durable effect identifiers
    /// (`intent_id`, `action_digest`) and per-row verdicts. The session alone
    /// is never authorization for it, under GET as under HEAD, in success as
    /// in every refusal. The served authorization binds the EXACT bytes this
    /// route serves: after the reconciliation completes, the canonical digest
    /// of the report encoding is recomputed HERE (never a hash or label from
    /// the browser) and the CURRENT reader verdict
    /// (`RootStore::paea_reader_verdict`) is evaluated on the service root —
    /// the root whose Git journals the report reads — BEFORE the report bytes
    /// are returned. The attribution is the existing `blp-object-attribute`
    /// control operation binding that exact report digest, this project root
    /// and the current BLP policy revision (see `trust::confidentiality`);
    /// clearance alone never invents provenance or attribution, and the
    /// verdict is re-read at every consumption. A bare session, an unknown
    /// provenance, a stale policy or an unattributed report refuse with an
    /// explicit `available:false` reply carrying ONLY the stable reason code —
    /// no finding, identifier, digest, journal state or error detail — even
    /// for the highest-cleared reader; HEAD answers the same refusal with an
    /// empty body. An unknown or disabled credential propagates the typed
    /// `TrustError::Authorization` refusal instead (non-success status); both
    /// paths mutate nothing, never journal anything and never yield a partial
    /// report. Refusals stay observable and the route recovers: a later
    /// correctly attributed read reuses the same public operation.
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
        let head = request.method() == Method::HEAD;
        // Reader credential shape is validated BEFORE any Root open; None is
        // the bare session the guard treats as never-authorization.
        let token = reader_bearer_token(request)?;
        let service = swarm_engine::RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        close(self.reconcile_with(&service, token.as_deref(), head), service.close())
    }
    /// Reconcile against the saved ledger root with the ledger opened/closed
    /// and its close failure paired ahead of the service close. Typed causes
    /// survive; the report is withheld when any close fails.
    ///
    /// Reader frontier: the reconciliation completes first (both roots opened
    /// read-only, separation guard applied, closes paired), then the exact
    /// digest of the report encoding is recomputed HERE and the CURRENT reader
    /// verdict decides whether those bytes may be served. A refusal answers
    /// the explicit `available:false` projection with the stable reason and no
    /// report byte (HEAD: empty body); an authorization error propagates
    /// typed. No report byte, identifier or journal state can reach a refused
    /// reader, a log line or a close cause.
    fn reconcile_with(
        &self,
        service: &swarm_engine::RootStore,
        token: Option<&str>,
        head: bool,
    ) -> Result<RouteBody, E> {
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
                    // Reader verdict on the EXACT bytes this route would serve:
                    // the digest is recomputed here from the just-computed
                    // report (never a browser-supplied hash or label), and the
                    // guard re-reads the CURRENT persisted BLP record itself.
                    // Evaluated before the report bytes are returned, so a
                    // refusal can never carry a finding, identifier, journal
                    // state or any other report byte.
                    let report_digest = swarm_engine::digest_trust(&report)
                        .map_err(|e| {
                            root("reconcile-report-digest", swarm_engine::RootStoreError::Trust(Box::new(e)))
                        })?;
                    let verdict = service
                        .paea_reader_verdict(token, &report_digest, &self.root)
                        .map_err(|e| root("paea-reconcile-reader", e))?;
                    match verdict {
                        ReaderVerdict::Allowed => swarm_engine::encode_model(&report)
                            .map(|bytes| {
                                if head {
                                    RouteBody::plain(Vec::new())
                                } else {
                                    RouteBody::plain(bytes)
                                }
                            })
                            .map_err(E::Model),
                        ReaderVerdict::Refused(reason) => {
                            if head {
                                // HEAD mirrors the refusal status without a
                                // body; the paired close stays observable.
                                Ok(RouteBody::plain(Vec::new()))
                            } else {
                                let reply = serde_json::json!({
                                    "schema_version":1,
                                    "event":"paea-effect-reconcile-report",
                                    "available":false,
                                    "reason":reason,
                                });
                                Ok(RouteBody::plain(
                                    serde_json::to_vec(&reply).map_err(E::ResponseJson)?,
                                ))
                            }
                        }
                    }
                }),
            ledger.close(),
        )
    }
}
