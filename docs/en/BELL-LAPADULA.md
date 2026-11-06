# Bell-LaPadula in Swarm — existing implementation

Documented state, 1 November 2026 · [Français](../architecture/BELL-LAPADULA.md).

**Bell-LaPadula is partially integrated into Swarm. Available controls do not
provide end-to-end confidentiality protection.** The engine capability keeps
`runtime_enforced:false` and `declassification_available:false`, including after
policy configuration.

## Available capabilities

| Capability | Implemented behavior | Assurance limit |
| --- | --- | --- |
| Confidentiality model | Ordered levels, compartments, clearances; dominance, conservative join, fixed calculation domain and unknown-classification refusal | Data contracts in `swarm-confidentiality`; no source/process authentication or OS confinement |
| Administration | Revisioned persistent policy; API/CLI show, preview, apply and diagnostic; Admin policy editor | Current Trust I3/admin identity for administration; no implicit mediation activation |
| Object attribution | `blp-object-attribute` records an explicit label under current Trust/BLP revisions for an exact digest/project path; replay/conflict controls | Actor also needs clearance for the label; no object-attribution Admin form or physical identity established by digest/path alone |
| Attribution monotonicity | Renewing the policy cannot lower the same digest/project label or remove compartments; an already-lowered history refuses on read | This rule alone does not track copies, changed content, summaries or new-digest objects |
| Targeted PAEA readers | Prepared Git-call preview and presentation require authenticated identity, current authority/policy, exact attribution and reader clearance | Presentation boundaries do not prove confidentiality of Git execution or every PAEA flow |
| Prompt provenance | New native v3 records cover every private prompt byte: instructions, project context, optional contract, nine JSON values and framing | Byte-consistency evidence without automatic producer labels or authority for earlier records |
| Recorded-prompt diagnostic | `blp-preparation-check` resolves current attributions for every slice, joins them and checks caller clearance; unknown sources, stale revision or invalid provenance refuse | Readonly post-recording observation; diagnostic caller is distinct from original sender; no dispatch authority or persisted response attribution |

Biba separately addresses integrity. I3, admin role, web session or provider
configuration cannot replace confidentiality clearance. Local Trust
authentication establishes neither a verified human nor declassification consent.

## Interfaces and usage

Existing `trust` operations:

| Operation | Purpose and authority |
| --- | --- |
| `blp-show` | Read policy/history/revisions; I3/admin |
| `blp-policy-preview` / `blp-policy-apply` | Examine then record explicit policy; I3/admin and current revisions |
| `blp-check` | Diagnostic over supplied labels; authenticated identity/current policy; request labels remain claims |
| `blp-object-attribute` | Explicit object attribution; cleared I3/admin, explicit intent/revisions |
| `blp-preparation-attribute` | Attribute a verified captured slice; cleared I3/admin, explicit parent/index/revisions, engine-derived digest/path; no send authority |
| `blp-preparation-check` | Check sources of a recorded native turn; cleared authenticated identity, current policy and attribution for every slice |

CLI: `swarm-rust trust OPERATION --root ROOT --input INTENT.json --json`; token
comes from `SWARM_TRUST_TOKEN`, never a declared identity field.
API: `POST /api/v1/trust-actions?action=OPERATION`; the verified local owner-key
flow uses Host/session/Origin/CSRF
and independent Trust authentication, including `Authorization: Bearer …`.
The session alone grants no authority for these operations. These BLP recipes
do not qualify the Authority/enrollment access modes developed in other packages.

`blp-preparation-check` accepts only `schema_version:1`, `expected_revision`
(observed BLP revision), `preparation_id` and `turn_id`. Result contains actor,
revision, prompt digest, part count and joined classification, with
`diagnostic_only:true` and `runtime_authorized:false`. No prompt text is returned.
Digests concern transmitted bytes including JSON escaping; attributing the raw
document is insufficient. Structured values remain aggregate slices and no
slice-digest inventory screen is provided.

HTTP diagnostic refusals distinguish invalid intent (400), identity/clearance
(403), missing turn (404) and a prerequisite requiring correction (409).
Incoherent provenance, incompatible private Root and storage/close failures remain
server errors. Historical rows without provenance acquire no authority: create
a new turn through the supported flow rather than rewrite old rows. See the
[technical contract](BIBA-RUST.md#preparation-source-diagnostic) for attribution
formats, errors, revisions and recovery.

## Delivered preparation quarantine

Local version299 adds `preparation_quarantine`, editable in Admin policy JSON and the
same API/CLI policy operations. True blocks new sends and controlled egress by
pending workers. Applying waits for already-engaged emission to finish. False/
omission retains legacy behavior without BLP assurance, not an authorized
classified-send route. The public journey verifies28 assertions and59 Admin checks in FR/EN, both
themes and1280/390 widths; delivery verifies56 version/asset checks. The real
policy was not activated by these recipes. See the
[contract](BIBA-RUST.md#explicit-preparation-send-quarantine).

## Unfinished work

- **BLP-01**: complete dynamic source/reader/output inventory. All 25 documented
  flows remain open for transversal BLP controls.
- **BLP-03**: explicitly migrate existing unclassified objects.
- **BLP-04**: establish producer attribution/source identities, bind authenticated
  sender to fixed domain and replay, then retain classification on answers,
  errors, summaries and memory.
- **BLP-05**: actual filesystem/network/provider/descendant mediation, binding
  bytes to current recipient, account and policy.
- **BLP-06**: declassification with established human identity and approval of
  exact bytes/recipients; no declassification command is available.
- **BLP-07**: transversal recipe, independent review and RETEX after integration.

The next integration chain remains BF-05 → BF-07 → BF-16: prompt sources →
provider engagement → result inheritance. A positive diagnostic is neither a
send ticket nor evidence of global acceptance.

## Retained evidence and scope

Quarantine is delivered in **v0.1.1-dev.299**, image
`75ad879faf4807d33b6b9c3ff5a71e570aa76d252f394da3b309d95ecadda5e9`,
compiled commit `d4e219eeafcae5f90307a105ddaa93ba9874dc4f`, modified=true.
[Process/delivery evidence](../plans/actions-confiance/verification-blp/20261101-preparation-quarantine-installed-01/delivery-scope.json).
One newer compiler input remains pending, as disclosed by the launcher.

Previous diagnostic delivery: **v0.1.1-dev.296**, image
`e4a802c4dc4d32d90d036169604168c64db15196695d692885369b748686ca72`.
Local version was reread during this update: same number, compiled commit
`b4cbfa22552140fe9a7bcf46ac91165b0abe120d`, disclosed modifications. This is a dated
observation, not a promise about a future instance.

| Retained evidence | What it establishes |
| --- | --- |
| [Public journey](../plans/actions-confiance/verification-blp/20261101-preparation-check-public-01/result.json) | 26 CLI/HTTP assertions on qualified normal image; actual prompt produced by inert local double, no paid provider |
| [Engine regression](../plans/actions-confiance/verification-blp/20261101-preparation-check-engine-02/result.json) | Attribution resolution/join/refusal/recovery on consumer fixture |
| [HTTP statuses](../plans/actions-confiance/verification-blp/20261101-preparation-http-status-01/result.json) | Business refusals distinguished from internal/cleanup failures |
| [Local delivery](../plans/actions-confiance/verification-blp/20261101-preparation-check-installed-01/delivery-scope.json) | Running image,13 matching sources,56 FR/EN/theme/version/asset browser checks and two CLI help checks |
| [Inventory](../plans/actions-confiance/verification-blp/20261101-preparation-flow-inventory-01/result.json) | Freshness of25 flows,50 anchors and46 files; neither exhaustive inventory nor disclosure prevention |

Fixture checks, production-profile qualification and local installation are
separate evidence. These results establish no live Trust bootstrap, live-data
classification, real provider, human declassification or independent global
acceptance.

[BLP plan and recovery](plans/actions-confiance/BLP-APEX.md) ·
[Flows and boundaries](BLP-FLOWS.md) ·
[Security positioning](SECURITY-POSITIONING.md).

Implementation anchors: [model](../../rust/crates/swarm-confidentiality/src/lib.rs),
[policy and attributions](../../rust/crates/swarm-engine/src/trust/confidentiality.rs),
[prompt diagnostic](../../rust/crates/swarm-engine/src/root_store/preparation_call.rs),
[PAEA reader](../../rust/crates/swarm-engine/src/root_store/reader_guard.rs),
[Admin policy editor](../../web/trust-confidentiality.js).

Quarantine evidence: [engine and lock ordering](../plans/actions-confiance/verification-blp/20261101-preparation-quarantine-locks-03/result.json), [real fixture CLI/HTTP and Admin journey](../plans/actions-confiance/verification-blp/20261101-preparation-quarantine-public-02/result.json). Historical refusals remain in the APEX record.

The `blp-preparation-attribute` producer attributes a verified captured slice under
a cleared I3/admin and current revisions; the engine supplies digest/path and
returns no source text. This explicit post-capture classification grants neither
send authority nor a persisted domain. Contract and example:
[BIBA-RUST](BIBA-RUST.md#attribution-from-a-captured-preparation-source).

Captured-source attribution is delivered in **v0.1.1-dev.304**, image
`341e71bb2e67d47d0c468c04f8c7ecd38f313fb0686b60af88ba675d50cbade6`.
Evidence covers103 public CLI/HTTP assertions,2 engine tests,1 HTTP unit and56
FR/EN delivery browser checks. The native profile contains no fixtures;21 runtime
files match the active image. No real data or policy was classified/changed. See
the [APEX plan](plans/actions-confiance/BLP-APEX.md#captured-source-producer-delivery--1-november-2026).

## Fixed domain before transport

Version309 adds explicit pre-admission source attribution, actual authenticated
Trust principal and a private immutable full-prompt capsule. Every slice needs
current classification; the domain is their join, not maximum clearance. This
capsule is neither an executable turn nor provider authority and reserves no
budget. Metadata reads revalidate owner/current policy/Root; no implicit refresh.

`blp-preparation-input-attribute`, `blp-preparation-admit` and
`blp-preparation-admission-show` are described in the
[technical contract](BIBA-RUST.md#immutable-pre-transport-admission).
74 public assertions and103 CLI/HTTP regressions pass on309. The active image
and22 runtime files are verified. The56 browser checks cover a fixture of the
same image; the real Authority Version dialog still requires an authenticated
session observation. Admission and fixture evidence do not establish end-to-end
confidentiality.

## Explicit recipient binding and last-use decision

BLP-05 adds two operations that tie the account/endpoint to a controlled
recipient, then arbitrate the send at last use — without emitting. Their full
technical contract is described in the
[technical contract](BIBA-RUST.md#recipient-binding-and-last-use-transport).

- **`blp-preparation-destination-attribute`** (privileged administrator):
  records a destination label for an `api-` connection identified by its
  `base_url`/`model`/config-digest triple, with mandatory `base_url` and
  `model` echo. The connection key is never an input; the canonical digest is
  computed by the current Root. The binding is re-recordable (last write wins)
  and dies with the policy Entry carrying it: every policy renewal wipes
  bindings just like slice attributions.
- **`blp-preparation-transport`** (principal owning an admitted capsule): under
  the shared egress lock, fully re-verifies the capsule, the
  connection/provider_source/settings freshness since admission, the current
  binding for that exact digest, the no-write-down dominance of the output
  label toward the destination label at last use, then observes the budget
  envelopes read-only. Fixed non-emissive reply: `runtime_authorized:false`,
  `runtime_enforced:false`, state `transport-authorized-not-sent`. No turn,
  reservation, provider IO or legacy modification.

Typed refusals: grammar (`blp-transport-invalid`, `blp-destination-invalid`),
echo (`blp-destination-echo-mismatch`), missing attribution
(`blp-destination-not-attributed`, 404), freshness
(`blp-transport-connection-changed|blp-transport-provider-source-changed|
blp-transport-settings-changed`, 409), write-down (`blp-access-denied`),
budget (`preparation-budget-exhausted|work-budget-exhausted`).

Scope and limits are unchanged: the decision does not engage the real transport
(BF-07 remains OPEN), inherits no results, confines no descendants and protects
no legacy send. Verification rests on the `preparation_transport.rs`
integration test and the public journey `tests/blp_transport_public.py` via the
recipe `tools/verification/recipes/blp-preparation-transport-public.json`.

## Worker emission hook (BF-07)

The API worker (`_prepare_turn`, Api branch of `swarm-cli/src/
preparation_worker.rs`) now calls the decision at last use before any
classified emission, through
`RootStore::preparation_worker_transport_gate(id, token, base_url, model)`.
The gate runs inside the shared egress reservation already held by the worker
and refuses before the HTTP request is built: no socket opens without a
decision.

- Zero admission receipt matching the turn
  (preparation_id, provider, request_hash) → immediate `Ok(())` before any
  Trust-state read: the legacy path is unchanged.
- ≥1 receipt: full re-verification of each capsule (principal read from the
  producer-written receipt, never supplied by the caller), settings/
  provider_source freshness, destination attribution for the digest resolved
  from the connection recorded in the turn, no-write-down at last use,
  read-only budget observation. The connection comes from the turn's
  `provider` field; the worker's actual `base_url`/`model` are the mandatory
  echo (closing the claim→send TOCTOU).
- Custody: re-read with token=SHA256(nonce) + turn PID/stamp/host
  (`preparation-worker-registration-mismatch` otherwise). The worker holds no
  principal; identity stays the receipt's re-verified one.
- Fail-closed scan: an unreadable/undecodable matching receipt refuses
  (`blp-admission-not-found`/`blp-admission-record-invalid`), never silence.
  The gate contacts nothing, reserves nothing, writes nothing to SQL.

Verification: integration test `preparation_worker_transport.rs` (token/
destination/echo refusals, exact pass, zero gate reservations, self-cleaning
fixture) and recipe
`tools/verification/recipes/blp-preparation-worker-gate.json` (gate, neighbor
suites, CLI compilation). Scope: the Rust worker's Api branch; the CLI path
keeps its own egress control. Result inheritance (BF-16) and OS/descendant
confinement remain open; nothing is declared activated in production or
accepted.

## Dialogue reader control (BF-16a)

The public read of a preparation's dialogue
(`RootStore::preparation_dialogue(id, reader_token)`; HTTP
`GET /api/v1/preparations/dialogue`; CLI `preparation dialogue` with
`SWARM_TRUST_TOKEN`) now applies, **only when at least one admission receipt
matches the turn being read**, the no-read-down control:

- Scan-then-decide: the uncontrolled SQL read runs first (decision input
  only, nothing is served before the verdict), then the admission-receipt
  scan (`blp-admission-{64 hex}.receipt`, fail-closed reads) filtered on
  `captured.preparation_id` and the turn's (provider, request_hash) triple.
- Zero matching receipt → the legacy path is served unchanged, no Trust
  state read: continuity for preparations without admission (dedicated
  test).
- ≥1 receipt: identity first (PA-09's `ReaderIdentityState`, observed
  without a reservation — the Trust→settings lock order is preserved):
  bare session → `blp-dialogue-authentication-required`; absent journal →
  `blp-dialogue-policy-required`; unknown/disabled token → typed refusal;
  stale policy → `blp-dialogue-policy-stale`. Then the three-pass
  recovery: (1) each matched receipt is fully re-verified
  (`preparation_admission::verify` in its original order, principal read
  from the producer-written receipt) and the result is classified once —
  the exact `blp-admission-policy-changed` refusal marks an
  identity-valid but stale receipt, any other refusal (invalid record,
  wrong Root, source re-join, storage) propagates unchanged and refuses
  the whole read even if a current equivalent exists; (2) a stale receipt
  is covered only by a fully verified CURRENT admission of the same
  principal, same opened Root instance, strictly equal output label and
  byte-identical captured lineage (`equivalent_current_covers`;
  admission_id/event_id/request_digest are legitimately redefined by a
  renewal and are not compared; a provenance serialization error refuses
  with its typed cause, never silently); (3) every matched admission,
  current or covered-stale, still passes the reader clearance
  `check_read(subject, classification)` on the receipt's recorded label →
  `blp-dialogue-access-denied`. Coverage requires strict label equality:
  a renewal that raises the label keeps the 403 staleness until
  re-admission at the equal label. A refusal serves zero dialogue bytes
  (no partial filtering).
- Web: the `reader_bearer_token` extractor (one `Authorization: Bearer
  <non-empty>` header, otherwise a typed refusal before any Root open); the
  four `blp-dialogue-*` codes map to 403 FORBIDDEN. CLI: the
  `SWARM_TRUST_TOKEN` variable (empty = bare session).
- The internal producer path (`preparation_call`, answer capture) keeps the
  uncontrolled read: the producer is the legitimate domain holder; BF-16a
  changes nothing on the capture side.

Verification: integration test `preparation_dialogue_readers.rs` (refusal
matrix bare session / unknown token / under-cleared reader, recovery by a
cleared reader, legacy continuity without admission) and recipe
`tools/verification/recipes/blp-preparation-dialogue-readers.json` (dialogue,
neighbor BF-07 gate, CLI compilation; evidence
`blp-dialogue-readers-proof-20261105b-2479227`, re-run on the evolved tree
after the staleness reception, verified archive).
The public HTTP boundary and its staleness recovery are covered by
`tests/blp_dialogue_reader_public.py` and the recipe
`tools/verification/recipes/blp-dialogue-reader-public.json`
(received on 3–4 November 2026 on qualified images 325/331: GET/HEAD 403
`blp-dialogue-policy-stale` after a public renewal, same-intent
re-admission, recovery 200 to the exact same DTO, stored-corruption guards
refusing 500 with an exact error oracle).
Scope: presentation reads only. Still open within BF-16: domain
inheritance on errors/propositions, file/network/descendant mediation,
migration of existing data, human declassification (BLP-06). Nothing is
declared activated in production or accepted.

### BF-16b — readers of show, history and release-review (6 November 2026)

The same reader verdict now applies to the preparation's other
presentation frontiers: `RootStore::preparation_show(id, reader_token)`,
`RootStore::preparation_history(id, kind, before, reader_token)` and
`RootStore::preparation_release_review(id, reader_token)` (HTTP
`GET /api/v1/preparations/show`, `/history`, `/release-review`; CLI
`preparation show|history|release-review` with `SWARM_TRUST_TOKEN`):

- The verdict is extracted into one shared function
  `preparation_dialogue::reader_verdict` (pub(super)) called by
  `read_controlled` (dialogue) and by show/history: same `blp-dialogue-*`
  codes, same scan-then-decide sweep, same three-pass recovery, same
  clearance; no preparation or document byte is served before `Ok(())`.
- Show/history read their decision keys through the `recorded_turn_keys`
  helper: THIS preparation's recorded turns are decoded (provider/
  request_hash live in the recorded body) under the same read snapshot,
  prompts cleared before the values are returned — decision input only,
  the helper serves nothing. A row identity mismatch refuses like the
  dialogue frontier (`preparation-dialogue-row-identity-mismatch`).
  Unlike the dialogue read, no active-history observation runs there
  (show/history observe documents, not live calls).
- Zero matched receipts → unchanged legacy path: bare sessions are kept
  on preparations without admission (dedicated continuity tests for show
  and history).
- Documents exposed by show/history inherit the source turn's domain
  (`source_turn`: adopted brief/plan): an under-cleared reader can no
  longer read through show/history what the dialogue refuses it.
- `preparation_release_review` forwards the token to `preparation_show`:
  a conversion review of a preparation carrying admitted classified
  output refuses an under-cleared reviewer with the same codes.
- Web: the three routes extract `reader_bearer_token` like dialogue; the
  four shared codes map to 403 FORBIDDEN (unchanged mapping). CLI:
  `SWARM_TRUST_TOKEN` (empty = bare session), resolved once for
  show/release-review/dialogue/history.

BF-16b verification: extended integration tests
`preparation_dialogue_readers.rs` — `show_and_history_inherit_the_
dialogue_reader_verdict` (bare session and under-cleared reader refused
on show and on history for every kind, recovery by the cleared sender)
and `show_and_history_without_any_matched_admission_keep_legacy_bare_
read` (bare-session continuity without admission, both frontiers);
web/CLI suites (`service`, `confidentiality_http`, `swarm-cli`) re-run.
Scope remains: presentation only; execution, adoption and other channels
stay outside this verdict. Nothing is declared activated in production
or accepted.
