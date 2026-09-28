# Reducing an oversized delivery

Swarm holds a new review when the diff from the last accepted candidate exceeds
100 files. This is a Swarm delivery bound, not a Cursor rule or a model context
limit. Raising the review budget or splitting review calls does not bypass it.
Existing reviews are not retroactively invalidated.

`planning scope-preview WORK --input task.json` returns the accepted base,
candidate, revision and changed paths. Use `{"task_id":"TASK"}` as input.

`planning scope-patch WORK --input selection.json` returns an exact patch and
explicit selected/deferred path lists. The JSON input contains `task_id`,
`expected_revision`, `expected_candidate` and a nonempty `scope_files` array of
at most100 distinct changed paths. Select dependencies, tests and delivery reports
explicitly. Paths are literal; renames are represented as deletion plus addition.
Apply the patch to a separate checkout of the returned `accepted_base`.

API parity: GET `/api/v1/planning?work=WORK&task=TASK&action=scope-preview`;
POST `/api/v1/planning?work=WORK&action=scope-patch` with the selection JSON.
Local authentication applies. No new web button is included.

Export does not change the checkout, index, history, budget or acceptance.
The original result remains preserved. The engine does not guess semantic
file dependencies. Rebuild and test the reduced delivery, then submit it through
`planning revise-recovered-result` with explicit external-repair confirmation,
current revision and examined Git tree. A complete oversized delivery with no
review can be corrected this way; normal checks and fresh review still apply.

Deferred changes need separate deliveries. Being under100 files does not prove
correctness or transport/budget compatibility. Local controls may already have
run before the review scope check. An exported patch is never an acceptance.

## Decomposition diagnosis

`scope-preview` now includes `decomposition`: signals, five possible split types
(requirement, component, dependency, specialty, volume), and deterministic path
inventories. A large single-file diff and mixed responsibilities are detected too.
Suggestions have `state=proposal_only`; dependencies are explicitly unexamined.
This does not launch a subplanner or authorize semantic review lots. The existing
guard remains active; an executable semantic plan needs further runtime support.
