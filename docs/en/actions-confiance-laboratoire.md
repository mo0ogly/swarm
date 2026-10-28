# Simulation laboratory — R17

The laboratory runs a closed versioned corpus against the Trust engine in a fresh
server-created fixture. No provider, payment or real case receives an action.
This scope does not qualify sensitive providers. Verification status comes from
the evidence receipt rather than this document.

Corpus v1 covers five scenarios: denied direct writes/Biba elevation, exact
approval, a single effect despite repeated commitment, lost response followed by
reconciliation, and original tampering under an anchor. Its manifest exposes
version, digest, identifiers, simulated connector, fixture clock/policy and scope.
The clock belongs to simulation; no input controls real business time. Seeds are
canonical uint64 decimal strings, preserving all bits across Go, CLI and browser.
Arbitrary paths, executables, URLs, destinations and production connectors fail
schema validation before admission.

## Administration and execution

CLI namespace `laboratory`, API `/api/v1/trust-laboratory`: `show`,
`policy-preview`, `policy-apply`, `run`, `result`, `replay`. Normal operator session,
Origin/CSRF and a current I3 business administrator are required. The authenticated
loopback `--server` client never falls back to a local root after remote failure.

Persisted policy `laboratory` contains strict boolean `enabled`, positive integer
`timeout_seconds` (proposed30s) and positive integer `max_concurrent_runs`
(proposed1). Zero is rejected even while disabled; omitting the policy block
preserves it. Shared configuration revision protects updates. Exposed technical
bounds are separate from `active_runs` consumption and historical `attempts`.
Preview does not mutate state.

`run` requires `schema_version:1`, `event_id`, `expected_revision`, reason and
`input`:

```json
{"corpus_version":1,"corpus_digest":"DIGEST_FROM_SHOW", "seed":"42",
 "scenario_ids":["biba-denials","exact-approval"],
 "connector_id":"local-ledger-simulation-v1"}
```

Version, digest and scenario selection are checked before admission. Values in a
candidate file never automatically become trusted authority. A durable attempt
is reserved before launch; an identical request reads its receipt without another
execution. `result` takes `run_id`. `replay` takes event/revision/reason fields and
`run_id`, creating a separate attributed attempt with the same inputs.
Deterministic digests exclude operational time, PID and paths. Divergent output
cannot be accepted as successful replay.

## Isolation, interruption and limits

The parent starts only its fixed helper through pipes with minimal environment.
The helper rejects an occupied directory, closes foreign descriptors and applies
the14 Landlock/seccomp checks, then runs the corpus on the locked operation
thread. Network and direct executable access are denied; Trust calls stay within
its fixture roots. No user code, callback or extension goroutine is accepted.
This confines the fixed helper rather than arbitrary providers.

The configured deadline cancels and terminates its process group. Execution stays
outside the business lock, and publication rechecks current identity, policy and
revision. Invalidation closes the attempt without publishing success. Owner PID
and start identity are retained; owner loss yields an interrupted attempt without
resetting consumption. New attempts are explicit. Only the temporary roots owned
by the current execution are cleaned. A hard server crash may leave an orphaned
simulation fixture; no global scan automatically cleans historical roots.
The interrupted attempt remains visible and consumed.

Reports retain corpus, inputs, policy, observations and limitations, without raw
tokens, fixture keys or mission paths. Corpus and engine evolve separately: old
results remain readable even if their corpus is no longer executable; replay
never silently adopts another current corpus. eBPF collection and real providers
remain separate qualifications.

Process source: [Go1.24.3 SysProcAttr](https://pkg.go.dev/syscall@go1.24.3#SysProcAttr). The parent stays on its creating thread for the child lifetime; cancellation and parent death are tested separately.

Isolated recipes:

```sh
go test -race -count=1 -run '^TestActionsConfianceT6Laboratory' -v ./internal/engine
python3 tests/actions_confiance_laboratory.py --binary BIN --evidence-dir DIR
node tests/actions_confiance_laboratory_ui.cjs BIN DIR
```

The UI shows a verdict, scenario/OS check counts and five readable case labels.
The JSON report stays collapsed until explicitly opened.
