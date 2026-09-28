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

The semantic-lot executor is not connected yet. Capacity/budget admission,
independent lot reviews, correction invalidation and final same-SHA review remain
required before this planning record can drive acceptance. Existing technical
fragment execution must not be misrepresented as semantic coordination.
