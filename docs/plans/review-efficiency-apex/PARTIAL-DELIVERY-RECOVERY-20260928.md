# Corrective resubmission of a completed partial delivery

## Confirmed defect and decision
E6's producer exited as completed with a partial delivery. The engine correctly blocked acceptance, but corrective resubmission required an independent refusal which cannot exist before a valid delivery. Restarting production would spend another attempt without repairing this missing transition.

The existing public `planning revise-recovered-result` operation now supports this exact case. It requires a stopped completed producer, delivery version 1, an existing conflict result whose immutable delivery fails validation, no independent review, explicit external-repair confirmation, current revision, exact attempt and examined Git tree. A complete original delivery is excluded. Original commits, producer status, attempt count and budgets remain unchanged. The corrected delivery must be complete and undergo normal controls and independent review.

## Verification
The focused regression covers rejection before review of the partial delivery, missing confirmation, stale revision, unrelated review ID, corrected submission, preserved original report, one consumed attempt and idempotent replay without another review. Full suite and race results are retained in `codex-takeover-20260927/partial-repair-*.log` outside the repository.

## RETEX
A terminal process is not a complete delivery. Recovery must cover the state between a partial delivery and an independent review, rather than asking the user to retry an impossible review. The extension reuses the public corrective operation and keeps all provenance. It does not declare the externally repaired producer autonomous; the separate real trial establishes the bounded autonomy claim.

## Observed installation and submission

- Fix commit: `56b35fb6ca2dc53b1365fdbfdd1b72a99c7b08ff`, pushed and installed.
- Host `go test -timeout 20m ./...`: PASS, 603.590 seconds.
- Host targeted recovery race suite: PASS, 87.272 seconds.
- Candidate `go test -timeout 20m ./...`: PASS, 619.412 seconds.
- `go vet ./...` on host and candidate: PASS. Candidate real behavioral runner: 2 PASS.
- Both first full runs reached the global 10 minute timeout; their logs remain preserved. Increasing the local test timeout did not change mission budgets.
- Installation verified binary SHA, clean source, unchanged mission and eight served asset hashes.
- Public external-repair event was persisted at revision534. Integration encountered SQLITE_BUSY with the web process present. Exact idempotent replay with the web process temporarily stopped progressed; web restarted. No duplicate attempt or paid review.
- Candidate submitted: `5bf7197cd65dee3eb3868b7712b532f0ce0e09a1`.
- Public cost preview: 368 artifacts, 352 diff files, 2,132,230 content bytes; protocol4 fits transport, requires9 inspections plus2 final calls. 73/75 calls already consumed:9 additional calls need authorization (ceiling84). No review was launched.
- Public main mission remains5/8; E6 awaits budgeted review, E7/E8 remain pending. The successful isolated real trial is distinct from main acceptance.

## Remaining reliability issue
The explicit recovery transaction succeeds, but a concurrent integration can still hit SQLITE_BUSY outside the protected review transaction. Temporarily stopping the web process was an external operational recovery, not an engine fix for that remaining contention. Preserve this as a follow-up regression target rather than claiming the engine is fully hardened.
