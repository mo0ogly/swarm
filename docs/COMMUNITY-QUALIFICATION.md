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
| Multiple real AIs in parallel | Not exercised by these recipes | NOT TESTED |
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
- The real parallel scenario, a complete web-only real-provider journey and an
  external-user trial are still required before claiming community-ready autonomy.
- One successful run does not cancel the previous failures or justify a stable
  product claim. Current positioning remains an experimental alpha.

The public correction and qualification report are reviewed through PR #1.
No branch-protection bypass or self-approval is used.
