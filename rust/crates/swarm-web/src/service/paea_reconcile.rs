//! Session-protected read-only PA-09 effect-journal reconciliation of the
//! service Root against the saved separate settlement ledger Root.
use super::{close, reader_bearer_token, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::ReaderVerdict;

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
    /// in every refusal. Three ordered gates keep that promise:
    ///
    /// 1. BEFORE any Root/settings/ledger IO, a bare session (no reader
    ///    credential) is refused: the typed non-success refusal (403) carries
    ///    ONLY the stable guard code `paea-reader-authentication-required`
    ///    (GET: in the typed failure body; HEAD: empty body), so the refusal
    ///    stays observable and cannot pass for a success. No journal error,
    ///    ledger state or settings detail can precede or accompany it.
    /// 2. AFTER the service Root is opened (read-only) and still BEFORE any
    ///    settings read, ledger open or journal read, a PRESENT credential is
    ///    decided by the shared engine identity precheck
    ///    (`RootStore::paea_reader_identity_precheck`) against the service
    ///    Root's CURRENT trust state: an unknown or disabled token receives
    ///    only the typed `TrustError::Authorization` refusal (403
    ///    `authorization_required`); an absent trust journal refuses as
    ///    `paea-reader-policy-required`; a recognized credential proceeds. No
    ///    settings, ledger or reconciliation state or error can reach an
    ///    unrecognized credential. The precheck grants no authorization: the
    ///    full verdict of gate 3 is still evaluated on real current bytes.
    /// 3. AFTER the reconciliation completes, the canonical digest of the
    ///    report encoding is recomputed HERE and the CURRENT reader verdict
    ///    (`RootStore::paea_reader_verdict`) is evaluated on the service root
    ///    — the root whose Git journals the report reads — BEFORE the report
    ///    bytes are returned. The verdict re-runs the same shared identity
    ///    observation on a FRESH read of the current trust state (a
    ///    credential recognized at gate 2 is re-authenticated, never served
    ///    from a cached identity) and enforces policy currency, attribution
    ///    and clearance on the exact report digest. The attribution is the
    ///    existing `blp-object-attribute` control operation binding that
    ///    exact report digest, this project root and the current BLP policy
    ///    revision (see `trust::confidentiality`); clearance alone never
    ///    invents provenance or attribution, and the verdict is re-read at
    ///    every consumption. A stale policy, an unattributed object or an
    ///    insufficient reader refuse with ONLY the stable reason; an unknown
    ///    or disabled credential propagates the typed
    ///    `TrustError::Authorization` refusal instead (non-success status).
    ///    All of these are non-success refusals.
    ///    All gates mutate nothing, journal nothing and never yield a
    ///    partial report; the route recovers: a later correctly attributed
    ///    read reuses the same public operation.
    ///
    /// HTTP contract (documented alongside GET): every reader refusal is the
    /// typed `Refused` error mapped to 403 for BOTH verbs — observably a
    /// refusal, never a success status that a report read would also carry.
    /// GET carries ONLY the stable guard code in the typed failure body
    /// (`error`/`failure.code`, the same surface every typed refusal uses);
    /// HEAD is stripped to the empty 403 body, so an empty reply can never
    /// pass for the success mirror. A HEAD success stays the empty 200 mirror
    /// of GET. No refusal ever carries a report byte, identifier, journal
    /// state or internal cause.
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
        let Some(token) = token else {
            // Bare session: refuse BEFORE any settings read, Root open, ledger
            // open or journal read, so no state or error of those IOs can be
            // revealed or even observed ahead of the refusal. The code is the
            // guard's stable no-credential code; GET carries it as the
            // explicit projection, HEAD as the typed 403 refusal.
            return Self::reader_refusal("paea-reader-authentication-required", head);
        };
        let service = swarm_engine::RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        // Gate 2 — shared identity precheck on the service Root's CURRENT
        // trust state, BEFORE any settings/ledger/reconciliation IO: an
        // unrecognized credential learns only the typed identity refusal,
        // never a ledger/separation or settings state or error. Recognized
        // credentials proceed to the full contract of `reconcile_with`,
        // whose final verdict re-authenticates on fresh current bytes.
        let outcome = match service.paea_reader_identity_precheck(Some(token.as_str())) {
            Ok(None) => self.reconcile_with(&service, &token, head),
            Ok(Some(reason)) => Self::reader_refusal(reason, head),
            Err(e) => Err(root("paea-reader-precheck", e)),
        };
        close(outcome, service.close())
    }
    /// The route's explicit reader refusal: the typed `Refused` error mapped
    /// to 403 for BOTH verbs — observably a refusal, never a success status.
    /// GET carries the stable guard code in the typed failure body
    /// (`error`/`failure.code`, the same surface every typed refusal uses and
    /// the field the served consumers read); HEAD is stripped to the empty
    /// 403 body. No report byte, identifier, journal state or internal cause
    /// is ever included; the code names only the guard state.
    fn reader_refusal(reason: &'static str, _head: bool) -> Result<RouteBody, E> {
        Err(E::Refused(reason))
    }
    /// Reconcile against the saved ledger root with the ledger opened/closed
    /// and its close failure paired ahead of the service close. Typed causes
    /// survive; the report is withheld when any close fails.
    ///
    /// Reader frontier: the reconciliation completes first (both roots opened
    /// read-only, separation guard applied, closes paired), then the exact
    /// digest of the report encoding is recomputed HERE and the CURRENT reader
    /// verdict decides whether those bytes may be served. A refusal is the
    /// typed 403 `Refused` error with ONLY the stable guard code (HEAD: empty
    /// body) and no report byte; an authorization error propagates typed. No
    /// report byte, identifier or journal state can reach a refused reader, a
    /// log line or a close cause.
    fn reconcile_with(
        &self,
        service: &swarm_engine::RootStore,
        token: &str,
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
                    // report through the engine's single canonical
                    // encoding+hash domain (the same bytes `encode_model`
                    // would serve; never a browser-supplied hash or label),
                    // and the guard re-reads the CURRENT persisted BLP record
                    // itself. Evaluated before the report bytes are returned,
                    // so a refusal can never carry a finding, identifier,
                    // journal state or any other report byte.
                    let report_digest = swarm_engine::digest_trust(&report).map_err(|e| {
                        root(
                            "reconcile-report-digest",
                            swarm_engine::RootStoreError::Trust(Box::new(e)),
                        )
                    })?;
                    let verdict = service
                        .paea_reader_verdict(Some(token), &report_digest, &self.root)
                        .map_err(|e| root("paea-reconcile-reader", e))?;
                    match verdict {
                        ReaderVerdict::Allowed => swarm_engine::encode_model(&report)
                            .map(|bytes| {
                                if head {
                                    // HEAD mirrors the allowed read without a
                                    // body; the paired close stays observable.
                                    RouteBody::plain(Vec::new())
                                } else {
                                    RouteBody::plain(bytes)
                                }
                            })
                            .map_err(E::Model),
                        ReaderVerdict::Refused(reason) => Self::reader_refusal(reason, head),
                    }
                }),
            ledger.close(),
        )
    }
}
