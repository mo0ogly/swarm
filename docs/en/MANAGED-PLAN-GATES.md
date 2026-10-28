# Persisted plan phases in managed publication

On October 10, 2026, T1 of mission `w-ca937edf4e0abd85f8d3d67b` had passed its
checks and independent review, yet T2 could not start. Managed publication had
assigned `delivery` to every check, contradicting the imported plan's `entry`
and `validation` phases. The task was `accepted` but failed `acceptedFresh`.

Publication now uses each persisted `Task.PlanChecks` phase and verifies all
required checks before committing acceptance. Legacy tasks without a phase map
retain their delivery phase. No timeout or budget policy is introduced.

The public `planning requalify` operation also supports an existing accepted
task whose gate contradicts its required plan checks. An exact passed review,
`confirm_recovery`, the current revision and retained producer, attempt, result
and candidate identities are required. Existing process, planning reservation,
reviewer availability and budget guards remain enforced. Fresh acceptance,
missing confirmation or a different review is rejected without mutation.

Requalification archives the previous gate, checks and review, withdraws current
validity and allocates a new proof key. The conductor reruns checks and independent
review on the retained result. Attempts remain consumed; the new review counts
against the existing budget. Replaying the request is idempotent. This explicit
repair does not demonstrate autonomous recovery by the planner.

The temporary-fixture tests `TestManagedImportedPlanPhasesProduceFreshAcceptance`
and `TestRequalifyExplicitManagedPlanPhaseMismatch` reproduce the stale acceptance,
verify the repair, reject unsafe or unnecessary recovery, and preserve historical
receipts, result identity and counters. A deterministic review fixture does not
establish real model review quality.

See the [architecture and flow diagram](../architecture/MANAGED-PLAN-GATES.md).
