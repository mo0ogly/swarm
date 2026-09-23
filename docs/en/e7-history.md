# E7 — historical acceptance qualification

The history recipe passed six test cases/subcases on isolated stores (4.352 s).
A historical accepted status remains recorded, while the engine and public CLI
report it as not fresh if independent review is missing. Reading does not rewrite
the stored work. Existing tests also cover preserved result integration retries,
explicit missing historical evidence and new candidate review requirements.

Run `node tests/engine_acceptance.cjs --case history`. Review providers in these
tests are deterministic doubles, not an independent AI assessment of this change.

Use `work show` to distinguish historical status from current freshness. Preserve
receipts, events and candidate identities. `planning retry-integration` applies
only to eligible integration failures (see REVIEW-TIMEOUT.md); it is not a general
requalification command for an old accepted task without a reviewer.

## Explicit requalification

For a managed Git task historically accepted without an independent verdict,
use `planning requalify WORK --input request.json`. Required fields are
`schema_version`, `event_id`, `expected_revision`, `task_id`, `agent_id`,
`attempt_id`, `result_commit`, `expected_candidate` and `reason`.

The engine requires retained integrated output, a completed producer, configured
checks and an available reviewer. It preserves old evidence in `requalifications`,
withdraws current validity and schedules fresh controls and review, without a new
producer or reset counters. The same request is idempotent after restart and
publication. Old receipt files are preserved in their original proof directory.
Candidate/contract drift, existing reviews, repaired results, active work and
exhausted budgets are rejected. Unmanaged repositories are outside this command.

Behavioral tests use deterministic providers; independent assessment of this
change remains distinct. No live mission was requalified by these tests.

## Web access
In the mission control view, historical managed acceptances without review show **Request review of this result**. The dialog fetches retained identities from the engine and asks for a reason. Submission uses the same contract as the CLI; requesting verification does not grant acceptance.
