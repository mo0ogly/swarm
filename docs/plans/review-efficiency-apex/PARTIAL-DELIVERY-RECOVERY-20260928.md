# Corrective resubmission of a completed partial delivery

## Confirmed defect and decision
E6's producer exited as completed with a partial delivery. The engine correctly blocked acceptance, but corrective resubmission required an independent refusal which cannot exist before a valid delivery. Restarting production would spend another attempt without repairing this missing transition.

The existing public `planning revise-recovered-result` operation now supports this exact case. It requires a stopped completed producer, delivery version 1, an existing conflict result whose immutable delivery fails validation, no independent review, explicit external-repair confirmation, current revision, exact attempt and examined Git tree. A complete original delivery is excluded. Original commits, producer status, attempt count and budgets remain unchanged. The corrected delivery must be complete and undergo normal controls and independent review.

## Verification
The focused regression covers rejection before review of the partial delivery, missing confirmation, stale revision, unrelated review ID, corrected submission, preserved original report, one consumed attempt and idempotent replay without another review. Full suite and race results are retained in `codex-takeover-20260927/partial-repair-*.log` outside the repository.

## RETEX
A terminal process is not a complete delivery. Recovery must cover the state between a partial delivery and an independent review, rather than asking the user to retry an impossible review. The extension reuses the public corrective operation and keeps all provenance. It does not declare the externally repaired producer autonomous; the separate real trial establishes the bounded autonomy claim.
