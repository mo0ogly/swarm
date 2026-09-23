# A different model for each task

Choose **Task model** in the task list, graph inspector or agent list. Select
**Choose for this task**, a provider and a level. Inspect the resolved model and
effort, then **Preview model** and **Save model**. With the current Claude policy,
Standard maps to Sonnet and Demanding maps to Opus. Provider policy is authoritative.

This persistent choice applies to future manual and automatic launches. Conflicting
manual provider/level choices are rejected. **Inherit the launch profile** removes
only this override: the task profile, otherwise the mission profile, applies again.
Workspace, instructions and execution limits remain unchanged. A usable launch
profile is still required.

Cards distinguish Planned, Inherited and Used. Used means the selection recorded
at launch, not an independently confirmed internal provider version. An `opus`
or `sonnet` alias does not establish a specific version number.

```sh
swarm task-model show WORK --json
swarm task-model preview WORK --input model.json --json
swarm task-model apply WORK --input model.json --json
```

`show` includes the revision and `available_models` with model, effort and policy
hash for each available level, without calling an AI provider. Example request:

```json
{"schema_version":1,"event_id":"model-choice-001","expected_revision":12,"task_id":"t1","provider":"claude","level":"exigeant","model_policy_hash":"HASH_FROM_SHOW","inherit":false}
```

Set `inherit:true` to remove the override. Stale revisions and changed policies
are rejected. Exact event replays are idempotent. Active attempts and running
reviews prevent editing. A policy change after saving blocks future launches
until the choice is examined and saved again.

No additional attempts, budget increases or result acceptance are granted.
Planner, subplanner and reviewer models remain separate and unchanged. This screen
does not provide individual editing of those roles or automatic planner proposals.
Exact model and effort are configured under **AI and connections → Configure levels**;
the task selects a level from that policy.
