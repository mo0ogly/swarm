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
Planner, subplanner and reviewer models remain separate and unchanged. Those roles have separate controls described below. Automatic planner proposals
are not included.
Exact model and effort are configured under **AI and connections → Configure levels**;
the task selects a level from that policy.

For Sonnet and Opus, Admin offers explicit Claude effort choices: `low`, `medium`,
`high`, `xhigh` and `max`. This is a saved, previewed and versioned policy;
the engine does not automatically replace a blank choice.
**Inherited Claude Code setting** means the client resolves its own setting,
possibly a personal one, rather than a verified effort level. An explicit choice
is passed as `--effort` on subsequent calls. Claude may lower it for the resolved
model or a managed cap; the saved route records requested effort, not evidence
of applied effort. See [Claude documentation](https://code.claude.com/docs/en/model-config#adjust-effort-level).
The legacy Haiku alias and custom models without verified capabilities retain
a blank effort. Changing effort refunds no calls and prevents reusing opinions
produced under another configuration.


## Planner, subplanners and reviewer

Pause the mission. In the team panel, use **Planner model** for each scope or
**Reviewer model** for the reviewer. Preview and save, then resume only when
mission execution is authorized. Saving never resumes the mission or clears a failure.

Each scope has its own override. Without one, it inherits the planning model
configured when the mission was created, not its parent's override. Changing the
root therefore does not silently change its children. Reviewers require an explicit
selection. Role cards show configured models. Scope `last_model` records the last
claim's selection; new reviews record `model_route`. Missing historical values
remain unknown.

```sh
swarm role-model show WORK --json
swarm role-model preview WORK --input role-model.json --json
swarm role-model apply WORK --input role-model.json --json
```

```json
{"schema_version":1,"event_id":"role-model-001","expected_revision":12,"scope":"validation","reviewer":false,"provider":"claude","level":"exigeant","model_policy_hash":"HASH_FROM_SHOW","inherit":false}
```

Use `scope:""`, `reviewer:true` for the reviewer. For a scope to inherit again,
set its `scope`, `reviewer:false`, `inherit:true`. Use the revision observed after
pausing. Stale forms are rejected. Active planner sessions, running reviews and
unfinished batch reviews prevent editing. No missing role is created; usage,
quotas, failures and previous verdicts remain unchanged.

## Reusable profiles in the Rust candidate — lot188

Admission retains the requested level in the task profile and, for a manual
launch, the reusable mission profile. An explicit task-model choice is bound to
the launch request first, then its level is retained. An absent level stays
absent: it means inheriting provider policy on a future launch, not freezing its
current default. Preview writes nothing; identical replay rewrites neither profile
nor reservation. Saved levels do not authenticate the model actually executed.
Older profiles that already lost this level are not reconstructed by guessing;
explicitly choose and save the desired level for future launches.

Lot188 is locally verified on a frozen candidate, not installed. Permanent launch-profile-level covers inheritance,
explicit levels and task-specific model choice through CLI without a provider
process. It does not yet port full dispatch or task.launch_profile precedence in
the automatic factory; the latter needs a versioned provenance contract.

Embedded resource check: embedded-settings-locale consumes SWARM_EMBEDDED_LOCALE_BINARY and SWARM_EMBEDDED_LOCALE_PROFILE (qualified profile result.json). After a valid protected API witness it reads the actual served English script, compares its hash with after.json and requires cost-setting label/help translations. Current workspace files alone are insufficient. The earlier188/normal03 candidate fails this check, with original evidence retained; no installation is claimed.

Current native limit: swarm-rust task-model show returns the Work alone; the available_models field described above is not yet ported there. Native recipes obtain a policy hash from an actual valid agent preview. Full CLI option discovery remains open in the migration plan.
