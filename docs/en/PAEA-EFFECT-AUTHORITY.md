# Exact authority for PAEA effect services

[Français](../architecture/PAEA-EFFECT-AUTHORITY.md)

The Linux `swarm_enrollment_authority::paea_action` module separates PAEA effect
permissions from historical Trust commands. It provides a borrowed capability
while current personal-account and business authority reservations are held.
It does not yet provide a daemon, payment/Git service, authenticated transport or
actual Linux account separation. Qualification and delivery are tracked in PAEA
increment 65.

## Permission and binding

`Request::new(root, service, operation, action)` validates two canonical SHA-256
digests. The closed service variants are `FictitiousPayment` and `GitPromotion`;
operations are `Commit` and `Reconcile`. These constants name the protocol and
select no executable, URL or operator policy. The requested signed permission is:

```text
operation: effect.execute
resource: paea-action-v1/<root>/<payment.settle|repository.promote>/<commit|reconcile>/<action>
```

A request is data, never a grant. Its constructor checks no storage, actual
action or identity. The future service must derive `root` from a retained opened
storage witness and `action` from control-attributed canonical PAEA bytes. At use,
it must compare them with current service, project, plan, revision, policy,
approval and business terms. A `trust-action/...` permission, another service,
operation or nearby digest never matches. No wildcard or fallback is accepted.
`Reconcile` grants no new commitment.

## Use and failures

`AuthenticatedControl::with_paea_action` checks the configured account password.
The private shared gate checks principal and installation, retains the business
reservation, reads the current signed active grant, checks exact permission and
expiry, invokes the consumer once and rechecks time afterwards. Trust now uses
the same gate with its public API retained. Locks and reservations are not OS
confinement guarantees.

Callers cannot construct or deserialize `Identity`, or turn a copied principal
into a lasting grant. Getters expose the retained enrollment, signed project,
plan, revision, expiry and exact request; consumers retain their own validation
obligations. A fixture password establishes no actual human consent.

`Use` preserves `authority` and `action` independently. `action=None` means the
callback was not invoked. `Some(Err(...))` preserves the consumer refusal, which
may itself retain a known or uncertain effect. `Some(Ok(...))` survives a later
authority failure. The personal layer may additionally return `UseError::AfterUse`
containing that complete outcome. Inspect every branch: failure establishes no
rollback or absence of effect. Refusal and journal failures retain independent
causes. No automatic retry, timer, restart, initialization or reconciliation.

`tools/verification/recipes/paea-exact-effect-authority.json` uses isolated private
journals and explicit signed fixture grants. It checks exact permissions and
neighbors, Trust/PAEA namespaces, passwords, callback refusal, revocation,
after-use cancellation and CAS. It also reruns the three Trust regressions affected
by the shared gate. Local evidence does not qualify separated effects, protected
installation, global PAEA invariants or E2/E3.

## Connected approval reply

The two connected web surfaces (`web/paea-approval-form.js` and
`web/paea-consent.js`) interpret a success of the approval POST consumption
only on an exact `state:"approved"`: the ordinary full object and the redacted
`trust-status-ack-v1` acknowledgement (`content_redacted:true`) both carry
that state and are accepted without imposing the redacted shape. `denied`
stays a visible refusal with its reason; `admitted`, `pending_approval`,
`committed`, `unknown`, `reconciled` and any missing, null or empty state
render fail-closed and are never a success. This is a browser-side
presentation interpretation, with no producer or server change; it qualifies
no installation or global acceptance.

Local qualification: permanent recipe `tools/verification/recipes/paea-approval-state-fail-closed.json` — `python3 tools/verification/recipe.py run --recipe tools/verification/recipes/paea-approval-state-fail-closed.json --output FRESH`, then `python3 tools/verification/recipe.py verify --evidence FRESH` on that same FRESH directory produced by the recipe run (run manifest, inputs and raw outputs retained in FRESH); 36 named FR/EN checks expected, 0 diagnostics. The browser separately creates its own private, fresh TMPDIR, printed in the step stdout; `verify` does not consume that TMPDIR. Evidence scope: synthetic API and actual Chrome scripts driven through DOM gestures; no verification of the normal ELF's resource serving, the physical pointer or the UI layout is claimed, and this evidence qualifies neither installation nor global acceptance.
# Git promotion failures and effect observations

The Rust promotion path retains a typed effect observation independently of its
error cause: a known durable outcome, or possible publication without an
established durable outcome. The latter exposes the safe code
`paea-git-promote-unknown`; it remains a failure and grants no retry authority.
Expiry or a different ref observed after the Git transaction cannot establish
absence of publication. Original SQL and independent cleanup causes remain
traversable without rendering their private data. A recorded outcome must survive
release, close and CLI output failures. A result line may already have been
written when a later close fails; the CLI must still return a nonzero exit code.

The CLI encodes JSON before writing, then retains original typed causes for
write, newline and flush under separate safe stage codes. Authority close keeps
the original personal/business envelope beside the known result and other close
causes. CLI and HTTP JSON failures separately expose `paea_git_effect.state` as
`recorded` or `unknown`. A pre-effect authority refusal does not establish absence
of historical effects for the intention; reconciliation without an effect row also
keeps history unknown. CLI text includes `effect_state` beside the safe failure code.
Projection guards exercise the real in-memory response producers, not network
delivery, a Git effect or local installation. Text normalization runs in the
renderer after traversing cleanup wrappers, keeping safe cause codes visible in
both CLI languages without private nested messages.

The [article v6 workstream](plans/actions-confiance/ARTICLE-V6-ADAPTATION.md)
retains the candidate, reproducer and checks. Source changes establish neither
installation nor universal PV2. Reference observation during Reconcile is integrated as a source candidate,
without native validation yet; an admission lease is also integrated as a source candidate without effect-point guarantees; a missing journal row still proves no
absence of Git effects. Oracle attribution must distinguish intention and attempt.


## Git reconciliation: history and current reference

The Rust candidate's `Reconciled` result carries `intent_id`, `target_ref`,
`observed_oid` and optional `recorded_effect` (historical action digest, ref and
OID). A validated journal row remains historical if the ref moves or disappears.
A current candidate-valued ref never attributes its publication to this intention.
Without an effect row, history stays unknown regardless of OLD/candidate values.

Reconcile opens the physically bound repository, compares its identity with the
prepared action, and reads the exact ref using the existing hardened Git runner
(a quiet, non-recursive `symbolic-ref` kind probe, then `for-each-ref`).
The documented exit 1 means non-symbolic and permits the listing; other errors
retain their typed causes. Dangling symbolic refs are rejected too. The journal
and Git observations are separate reads, not an atomic cross-store snapshot. Null means a successful listing without that ref. Symbolic refs,
malformed output and process failures remain typed errors. A failure after the
journal read retains any known historical effect and original causes, without a
fabricated Git observation. Cleanup preserves independent failures and observations.

The operation publishes nothing, reserves no intention and never writes the journal.
The web captured-agreement gate applies to Commit; Reconcile retains permission,
plan, policy and physical witness checks without requiring a new Commit agreement.
CLI/HTTP expose `outcome: reconciled`, optional historical effect and observed ref.
Consumers of the previous negative result must adapt to this changed shape.
No renewed publication authority or installation follows. Component checks and
the native public recipe remain separate evidence.

## Git promotion admission lease — source candidate

The matched agreement now carries optional RFC3339 `expires_at` from the approved
action's persisted active mandate, captured under the same shared reservation.
It is never supplied by a browser/agent field. No mandate means null metadata
and the signed identity expiry alone bounds the lease. Validated delegation
already requires a child to expire no later than its parent.

Commit retains the match and derives one fixed monotonic deadline from the
earlier signed/mandate expiry. Monotonic time is sampled before UTC; mapping is
relative to observed UTC, assumed trustworthy during derivation. Once derived,
UTC rollback cannot extend this deadline. Exact/past expiry rejects admission;
both clocks are checked before the transaction and after its prepared-lock read.
Preflights retain their own limits; no total callback/FS/SQL bound is added.
Slow preflight cannot mint a new lifetime.

The Git helper reuses hardened process/output/cause handling with the earlier
operator or supplied lease deadline. The synchronous nested read also respects
the already consumed parent transaction deadline rather than restarting its
operator budget. Post-publication reads retain the same lease boundary. Read
errors never become absence, including zero OLD; unknown remains explicit where
no durable outcome is established. Lease/process/cleanup causes remain typed.

Existing `git_command_timeout_seconds` is persisted in `.swarm/engine-settings.json`,
validated by `EngineSettingsValues::git_command_timeout` and consumed by the runner.
Units are seconds: zero expires immediately; null disables this operator deadline
only; legacy missing fields refuse. The native catalogue, generic Admin component,
show/describe/preview/apply routes and engine-settings CLI share authoritative
validation. This is source evidence, not a newly executed UI flow. No lease knob,
margin, retry or default is added. Disabling the operator deadline does not disable
authority expiry.

Client deadlines bound controlled admission/waiting, not Git's physical publication
after COMMIT: scheduling/storage may exceed expiry. This protocol carries no
receiver-enforced authenticated lease at that point. Late outcomes remain unknown
or historically known according to evidence, never renewed authority. No universal
PA/H2/PV2 claim follows. Injected clocks and inert Git repositories are component
evidence; signed native flow, concurrent withdrawal and delivered profile remain
separate.

Permanent recipe: `tools/verification/recipes/paea-git-promote-lease-rust.json`.
It declares twelve named component checks, consumer typing and Rustdoc compilation.
Its inventory is declarative, not a qualified native graph. An external Cargo
target requires the existing held-mount admission context before Cargo starts;
the recipe grants no rights and does not itself admit removable storage. Use
fresh evidence and preserve original refusals. This whole recipe has not yet
run in this increment; targeted commands and frozen composition are retained
separately.


## Native concurrent withdrawal observation (fixture)

PAEA synchronization hooks require Linux+test-fixtures and sit inside the real
shared section and before the revocation exclusive lock. They add no production
operator policy or admission path. Default image f429 was independently qualified
with empty engine/web/CLI/bin features and no fixture module among effective
inputs; this evidence does not cover arbitrary profiles.

Signed Q3B-R1 observes EAGAIN on the expected FD/inode, unchanged Trust during
SH, then recorded Commit, effective public revocation, renewed-use refusal and
Reconcile separating history/current ref. One named test PASS, two-step recipe
and integrity verification exit0, source/image bindings stable before/after.
See ARTICLE-V6-CONCURRENT-WITHDRAWAL under actions-confiance plans. This bounded
cooperative withdrawal observation proves no global H2, physical Git lease,
global PV2, real LLM or installation. Historical incidents remain in the plan.
