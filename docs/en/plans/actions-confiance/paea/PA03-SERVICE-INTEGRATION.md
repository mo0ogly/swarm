# PA-03 — effect service integration: resumption decision

[Français](../../../../plans/actions-confiance/paea/PA03-SERVICE-INTEGRATION.md)

October 27, 2026: source analysis only; implementation and qualification pending.
Owner: direct Codex. PA-02 remains partial; no global invariant established.
This decision changes no engine, policy or running service.

## Observed boundary

`AuthenticatedControl::with_trust_action` holds the current personal account,
business reservation and exact signed permission throughout the callback. `Use`
preserves action and authority results separately; `UseError::AfterUse` may retain
that complete result after a personal-account failure. Future service output must
preserve both layers rather than infer no effect from a late error. Existing
`authenticated_startup` tests cover exact grants, revocation and cancellation;
they do not establish an installed PA-03 service.

`RootStore::trust_enrollment_request` binds an opened datastore using a nonce;
its root_digest is not a caller-selected path hash. Its witness must remain alive
until explicit close. `trust_as_enrolled` compares this binding and exact bytes,
reserves the SQL plan and checks its current fingerprint. `trust::authentication`
also checks Trust revision and expiry. A new consumer cannot accept a deserialized
principal or replace this binding with a proposal-supplied digest.

`trust::operations::effect_transition` currently adds simulated entries to Trust
state: local-ledger-simulation and local-access-simulation. These are not PA-03's
separate SQLite ledger and Git repository. Receipts are indexed by transport key;
that observation does not establish PAEA business uniqueness. `canonicalize` in
the protection contract already derives project+invoice or project+repository+
reference+old OID intent independently of a fresh transport key. PAEA contracts
remain data and confer no commitment capability.

## Architecture decision and sequence

Two options were examined: extend Trust's simulated connectors directly, or bind
separate services to retained control. The former couples effect and evidence in
one state and does not provide the requested separation. Choose separate services,
reusing current control and opened storage, without replacing historical Trust.

1. Define binding among canonical PAEA action, current project/plan, storage
   witness and assigned service. Check the permission namespace explicitly: a
   Trust `commit` operation does not implicitly authorize every PAEA effect. No
   new authority or permissive default translation.
2. Implement fictitious settlement over privately held storage: trusted debt
   resolution, comparison of current terms and durable business intent uniqueness.
   Preserve previous effects when new requests, changed versions or authority
   refusals occur.
3. Implement assigned Git promotion: immutable objects, direct ref, expected old
   OID, durable intent before conditional publication, then reconciliation across
   the Git/storage boundary. Unknown outcomes block potentially duplicating commits.
4. Qualify with isolated fixtures: valid control before each fault, revocation
   before first consumption, new transport key, concurrency, failure after effect,
   reopening, refusal without mutation and recovery. Preserve independent effect,
   authority, publication and close failures.
5. Connect protected transport and actual separated execution only after PA-02/04
   prerequisites hold. Use supported local delivery and verify the running binary
   and affected French/English journeys.

Steps 1–4 do not remove the observed missing newuidmap/newgidmap tools or required
human/independent approvals. Step 5 remains conditional. The permission format
must be decided before any effect; this note does not make that decision. No new
operator timeout, budget, retry or limit is introduced. Technical bounds will need
justification in the implementation.

## Evidence and limits

The adjacent `PA03-SERVICE-INTEGRATION.sources.json` binds observations to inspected
source bytes, not a future build. No compilation or functional recipe was run for
this documentation change; no installation or acceptance is claimed. Concurrent
engine work and processes are preserved. Observed free disk space is approximately
2.6 GB; no shared cache or proof was deleted. Recheck capacity and effective build
profile before compiling. Global RETEX remains open until the original need is met.

Follow-up: increment65 implements separate exact PAEA permissions and the shared held gate. The initial source inventory above remains historical; no separate sink is connected yet. See [the current contract](../../../../en/PAEA-EFFECT-AUTHORITY.md).

66: Separate settlement reference kernel is now implemented and locally verified; enabled separation fails closed pending attributed admission. No protected daemon or public settlement transport. See [current contract](../../../PAEA-SETTLEMENT.md). Historical observations above remain dated.
