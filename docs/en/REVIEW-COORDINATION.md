# Planning review lots

The task's owning planner or subplanner can submit a `review-plan` operation
through the existing versioned `planning decide` contract. The operation's `id`
is the task ID and its `deliverable` is a JSON-encoded proposal containing
`candidate_commit`, `evidence_sha256`, `lots` and `final_review`.
Each lot has `id`, `kind`, `objective`, `files`, `criteria` and `depends`.
Kinds: requirement, component, dependency, specialty, volume.

Relevant pending events receive `review_planning_inputs` with exact candidate,
evidence digest, changed paths, task#N criteria and decomposition suggestions.
The engine requires complete file/criterion coverage, known references, unique
lot IDs, bounded lots, an acyclic dependency graph and a final interaction review.
Overlapping lots are allowed for complementary specialist reviews. The current
bound is100 lots with100 files per lot. Existing lease, revision, generation and
input-event guards remain mandatory.

The persisted `task.review_coordination` record contains the proposal, digest,
responsible scope, decision and dependency order. State `validated_not_executed`
means structural validation only. No paid review, worker launch or task acceptance
occurs. Replay is idempotent. Existing plans cannot be silently replaced.

The executor now runs approved lots in dependency order through the existing
independent reviewer. Each packet carries its lot objective and criterion text.
Large lots may require multiple bounded inspections. Shared reports, controls,
historical changes and supplemental evidence remain covered by global packets.
Intentional overlap is inspected for each applicable lot and increases cost.

Public `retry-review` checks candidate/evidence identity, complete transport and
all inspection calls plus two final calls before starting. `planning review-cost`
provides the estimate. Insufficient budget spends no call. Once started, the
coordination state is `execution_started`; the independent review and its durable
journal hold the outcome. Existing explicit resume preserves completed inspections.
Technical repartition cannot silently discard semantic lot boundaries.

Final review receives the proposal and original evidence. Partial inspection never
accepts a task; publication still requires same-candidate checks and final verdict.
This is sequential execution through the configured reviewer, not one autonomous
agent per lot or direct inter-reviewer messaging. Oversized indivisible evidence
is rejected, never truncated. Changed candidate/evidence invalidates the proposal;
implicit proposal replacement remains forbidden.

Capacity admission includes each lot's instructions and criterion text before
packing. When local client/model capacity is established, token-aware transport
may pack more evidence within a lot, while preserving lot boundaries. Preflight
also checks both final calls; cost estimation and execution select the same
protocol under the same remaining budget.
