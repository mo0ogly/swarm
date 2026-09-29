# Community qualification — 29 September 2026

## Result and limits

A real Claude mission recovered from an injected business defect and closed
without operator recovery after launch. This establishes one bounded successful
trajectory, not general reliability. The old abandoned E6 mission remains deleted.

| Scenario | Evidence | Result |
| --- | --- | --- |
| Simple sequential mission | `automatic_validation_process.py`: two worker processes, fresh receipts, no intermediate click | PASS, simulated providers |
| Parallel production then synthesis | `organized_coordination_process.py nominal`: overlapping producers, synthesis after both results, three reviews and handoffs | PASS, simulated providers |
| Shared workspace | Same recipe, `shared`: producers serialized | PASS, simulated providers |
| Server crash/restart | Same recipe, `restart`: ownership and progress preserved | PASS, simulated providers |
| Web preparation | `journey_ui.cjs`: browser need-to-departure authorization | PASS, simulated provider; does not prove completed production |
| Error then correction | `controlled_autonomy_campaign.py`: real Claude planner, subplanner, two workers, independent review and engine controls | PASS, real provider |
| Multiple real AIs in parallel | Two real Claude workers, isolated Git copies, sequential candidate reviews and final bundle checked | PASS, real provider |
| Outside-user usability | No external participant in this qualification | NOT TESTED |

## Real trajectory

Work `w-fb2c7c5e2901e5ddef514f02`, disposable Python project. The task is to
implement `is_expected(value)`. After the first correct production, the harness
changes the function to return True for every input. The engine rejects that
candidate using its fixed business check. The planner requests correction, the
second worker fixes it, and the independent review and engine accept the new
candidate. Both planner scopes close.

- 357.4 seconds from start to the closing event.
- 5 planning calls, 2 producer processes, 1 review call.
- 8 then 14 observed worker tool calls, with a ceiling of 20 per worker.
- Zero operator claim, decision, retry, acceptance or budget changes after launch.
- Worker-reported costs: USD 0.2924266 and USD 0.400452, total USD 0.6928786.
  This is **not the total mission cost**: planning/review costs are not included.
- Accepted candidate: `05af14415571a8425a8750f9384f409e4e062533`.
- Oracle: PASS, no missing evidence; checks require the same candidate identity
  for control, review and publication, plus both closed responsibility scopes.
- The harness stops the mission/server afterward and confirms no active agents.

Engine correction is in commit `e5ef435`. The trial binary was built from those
source changes just before that commit, with `vcs.modified=true`; its frozen
binary hash and build metadata remain in the private campaign manifest. Do not
claim it was built from a clean tagged release.

## Failed first attempt and corrective action

An earlier trial used a provider silence limit of 120 seconds but the harness
copied only the tool-call limit into the mission. The mission defaulted to 180
seconds. The engine correctly rejected the mismatch, but it had already created
a managed copy; later dispatch requested manual resumption and obscured the
original cause. No producer ran; two planning calls were consumed. The trial was
stopped and preserved, not marked successful or refunded.

Fix: validate static provider, model, execution limits and executable before
creating a managed copy. Preserve all explicit provider limits in the harness.
The regression test failed before the correction and passed afterward. Five
campaign tests, including the simulated fault/recovery integration test, pass.
The existing prepared-copy recovery guards remain in place.

## Practical RETEX

- Real recovery is possible on a small, explicit scope with a fixed business test.
- Simulated tests alone missed a real configuration combination. Qualification
  must include the actual provider configuration and a fresh disposable project.
- A rejection should happen before workspace side effects whenever possible.
- Even this tiny task required 22 worker tool calls and roughly six minutes.
  Reports, evidence and coordination have measurable overhead; simple work may
  be faster with a single agent.
- A complete web-only real-provider journey and an external-user trial are still
  required before claiming community-ready autonomy. The parallel test below
  preassigns its two tasks through the CLI; it does not test free-form planning.
- One successful run does not cancel the previous failures or justify a stable
  product claim. Current positioning remains an experimental alpha.

The public correction and qualification report are reviewed through PR #1.
No branch-protection bypass or self-approval is used.

## Real parallel qualification and a second engine correction

The first parallel harness stopped too early on the transient blocked state used
while a completed producer awaits review. That interrupted trial is not a success.
After correcting the observer, another run exposed an engine race: both reviews
used the same base; publication of the first revoked the second review. The
planner eventually recovered, but used a third producer unnecessarily. That run
fails the requirement of two productions without redundant work.

Correction: one nonblocking publication lane per work covers candidate checks,
independent review and publication. Workers still execute in parallel. The Git
operation lock is released during inference, so unrelated Git operations remain
possible. A sibling result waits for the lane, then builds and reviews its
candidate from the latest accepted revision. Counters and prior evidence are
preserved. The OS releases the lane on process exit.

Validation: `TestManagedPublicationLaneRetainsSiblingWithoutPaidReview` passes
with `-race` and checks two Store handles, no second paid call while the first
review is pending, Git lock availability, then two accepted results with fresh
review evidence and exactly two calls. `go test -run '^TestManaged' -count=1
-timeout 5m ./...` passes in 224.359 seconds; go vet and diff checks pass.

Final real run `w-a0b2c7c673fd590d6107437a`:

- PASS in 142.2 seconds; two worker execution intervals overlap.
- Distinct workspaces; exactly two producers, 9 and 6 tool calls.
- Two planning activations including the initial operator task-assignment claim,
  and two independent reviews. No operator recovery after launch.
- Worker-reported cost USD 0.4995294; excludes planning and review costs.
- Both tasks accepted, planner closed, source repository unchanged.
- Exported Git bundle cloned and both Python modules executed successfully.
- Candidate `44b039314d0bf9f97084377dc3bd194a567c7b74`.
- The real trial used the corrected source before commit; this is not a
  qualification of the previously published main binary.

Reproduction helper: `tests/parallel_real_campaign.py BINARY NEW_OUTPUT
PROVIDERS_JSON`. It deliberately invokes real Claude processes and is not run by
default CI. It requires a new output directory and retains failed evidence.
The preparatory harness errors and earlier paid trials remain separate and are
not included in the final successful run's cost or duration.
