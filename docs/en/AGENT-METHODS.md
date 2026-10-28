# Working methods for Codex, Claude and Swarm

[Français](../AGENT-METHODS.md) · [User guide](USER-GUIDE.md)

This repository includes **thirteen methods** for specifying, implementing, reviewing
and improving Swarm: guided change workflow, examination and improvement,
specification writing and examination, code review, diagnosis, verification,
replanning and lessons learned. They use explicit criteria, traceable decisions
and observable evidence without requiring another installation.

The user's standing preference is recursive improvement: every discovered
improvement is valuable and remains traceable, even without a failure. Establish
its origin and expected benefit, decide within the role's authority, act or route,
verify the effect, then reassess the need and retain verified lessons. Any anomaly
triggers `replan`. Deferred work keeps its owner and resumption condition. This
method contract proves no autonomous improvement of the installed engine and
grants no additional authority.

Go build evidence covers package code, including module-root files, go:embed
resources and the bytes actually substituted through overlays. An assumed list
of directories is insufficient. Establish a valid control with the same
prerequisites before each fault injection: one common rejection does not prove
coverage of every defect. These rules are also embedded in the shared contract;
rebuild for future attempts without changing historical evidence or already
running processes.

For the Rust migration, [APEX](../../.claude/skills/apex/SKILL.md) includes the method adopted on 22 October 2026: strict contracts and a differential bench, journey-based rewrites, vertical lots, risk-based evidence, deferred hardening, a short plan, owned files and early R6/R7 preparation. Writes retain a single cutover; an earlier read-only service awaits explicit confirmation. This persistent instruction does not update already running contexts. All eight points remain in the conditional APEX reference `references/rust-migration.md`: load it for migration work without enlarging every native role body. Tool-free preparation must receive this reference explicitly.

## Plain-language catalogue

User-facing names follow the [README catalogue](../../README.en.md#working-methods). Specification methods cover both requirements writing and plan examination. Historical identifiers below are native commands and compatibility identifiers, not interface labels.

<details>
<summary>Native commands and technical identifiers</summary>

## Start here

Open Codex or Claude **in this repository**, then invoke a method:

| Need | Codex | Claude |
| --- | --- | --- |
| Frame PRD, journeys, stories, architecture and plan | `$product-planning` | `/product-planning` |
| Deliver authorized research, design and tasks | `$product-delivery` | `/product-delivery` |
| Review coverage, design and evidence | `$product-review` | `/product-review` |
| Analyze, plan and deliver a bounded change | `$apex` | `/apex` |
| Audit with reproducible evidence | `$audit-pdca` | `/audit-pdca` |
| Specify a need and testable requirements | `$spec-builder` | `/spec-builder` |
| Discover official model prices and applicability | `$pricing-discovery` | `/pricing-discovery` |
| Find gaps in a plan | `$spec-audit` | `/spec-audit` |
| Review code and implementation risks | `$code-reviewer` | `/code-reviewer` |
| Diagnose a defect before a scoped fix | `$debug` | `/debug` |
| Verify the affected flow after a fix | `$verify-fix` | `/verify-fix` |
| Recover an existing blocked plan | `$replan` | `/replan` |
| Explain outcomes and prioritize improvements | `$retex-analyzer` | `/retex-analyzer` |

Examples to enter in the conversation:

```text
$apex --plan-only Specify quota recovery with the same web and CLI rules.
$audit-pdca Audit result acceptance; report only, without making corrections.
$audit-pdca --fix Repair confirmed defects within this scope and rerun the checks.
$verify-fix Verify that a refused launch creates neither an agent nor a reservation.
$replan Recover the existing plan from the observed failure while preserving its limits.
```

For Claude, replace `$` with `/`. `--plan-only`, `--resume`, `--fix` and
`--score-only` are instructions interpreted by the method, not additional `swarm`
binary flags. Audits are read-only by default. Methods answer in the user's language.

</details>

## In Swarm preparation

The existing **Prepare with AI** method selector uses:

| Preparation method | Material provided to the model |
| --- | --- |
| Analysis and planning | Need analysis and planning |
| Application creation workflow | Product framing, journeys/stories, architecture, design and plan; no execution in preparation |
| Guided workflow — prepare a change | Scope, criteria and task decomposition |
| Diagnose and fix a problem | Known facts, diagnosis and a scoped verification plan |
| Examiner et améliorer — préparer l’examen | Audit scope, risks and verification plan |

Swarm sends the entire method files and shared contract to the model. Execution
phases remain prohibited during preparation: the model proposes a brief or plan;
it cannot launch or accept agents. Other methods are available in native sessions or role framing, rather than extra preparation buttons. The [product guide](PRODUCT-WORKFLOW.md) explains the application creation workflow stages.

The CLI exposes the same catalogue:

```sh
swarm --root "$PWD" --json prepare methods
```

Each entry includes `available` and a `sha256` fingerprint. `audit_pdca` remains
an alias for `audit-pdca` in Swarm, and `/audit_pdca` in Claude. Historical aliases remain accepted for older integrations; use the specification methods for new workflows.

All five preparation methods are embedded in the binary and work in an empty
project, in native and Docker installations. Swarm creates no `.claude` directory
in your project. For each file, an existing project-root override replaces the
bundled version; an absent file uses the embedded version. Local files must be
regular UTF-8 text, nonempty and at most 128 KiB. Symbolic links, invalid files
and denied access make the method unavailable rather than silently replacing it.
Changing or removing an override changes the context fingerprint. Preparation
remains restricted to analysis and planning.

## Automatic engine framing

Every change includes relevant documentation in the same increment, including
methods and configuration. When no update applies, retain the reason in existing
tracking. Keep FR/EN versions and diagrams aligned; their presence proves neither
effect nor acceptance.

The shared personal rules imported by `~/.claude/CLAUDE.md` and repository rules
also require current docstrings/Rustdoc/Go comments for new or modified public or
nontrivial functions: purpose, inputs/preconditions, outputs, errors, effects and
limits. Check links and executable examples. Persistent files frame later contexts
that read them; this update does not replace an already running Claude process's
context or the engine's embedded instructions. The embedded contract is unchanged.

Throughout the migration, each increment also retains a review of hardcoded
policies, documented contracts and error paths, bound to the reviewed file hashes.
Automated searches produce candidates; semantic review separates test data,
invariants, technical bounds and operator policies. Trace each policy from its
persisted configuration to the real consumer and Admin setting. Check affected
FR/EN pages, diagrams, evolution notes and examples, or explicitly justify why
no change is needed. Unverified findings stay open; a scanner or comment alone
cannot establish compliance. Revisit affected earlier increments after a finding
without modifying their historical evidence.

The migration qualification collector also requires an explicit previous baseline
and a fresh four-axis review; missing coverage or open findings refuse increment
qualification. Earlier findings remain in the plan until resolved before global
acceptance.

For new attempts, the engine includes role-specific methods in the context sent
to the provider. This does not depend on native skill discovery in Codex or Claude.

| Role | Included methods | Boundaries |
| --- | --- | --- |
| Planner and subplanner | Guided workflow, examination and improvement, specification writing and examination, replanning, diagnosis, product planning | PLAN and ACT; no tools or code changes |
| Worker | Guided workflow, examination and improvement, verification, diagnosis, handoff and tracking templates, product delivery | DO and CHECK within authorized scope; read-only for audit-only tasks |
| Independent reviewer | Examination and improvement, code review, product review | CHECK on supplied evidence; no tools or candidate changes |

The `workflow` field records version, role, method names and SHA-256 digest. Unknown
roles and oversized framing are rejected. Methods are **embedded at build time**:
a worker cannot replace engine instructions by editing its checkout. Rebuild the
binary after changing the pack to apply it to future attempts.

Preparations still read project files dynamically. Historical attempts do not gain
these metadata retroactively. The field records constructed guidance, not proven
model obedience. Executable checks remain necessary.

The [communication protocol](AGENT-COMMUNICATION.md) explains complete handoff
delivery, context receipts, linked decisions and their limitations.
Check that you are using the intended checkout, not an older copy.

## One source, two discovery paths

```text
AGENTS.md                         Codex project instructions
CLAUDE.md                         Claude project entry point
.claude/skills/<name>/SKILL.md     canonical sources for all twelve methods
.agents/skills/<name>             relative link to the same method
.claude/skills/<name>/agents/     Codex selector labels and sample prompts
.claude/commands/                 native commands and compatibility aliases
tools/agent-workflows/CONTRACT.md shared rules also loaded by Swarm
```

Links stay inside the repository and travel with Git. Copies that do not preserve
symbolic links must restore them before use. Edit the canonical source, then run:

```sh
python3 tools/agent-workflows/check.py
git diff --check
```

The checker validates links/sources then runs Rust `agent_workflow` integrations.
Nine named tests are required, including planner/subplanner/worker/reviewer role
witnesses over actual engine-built contexts. Empty collection, missing roles,
ignored/failed tests and incoherent counts refuse. Original failure statuses and
both channels survive; no global watchdog is added. Cargo uses the caller's
cache/environment under offline/locked dependencies. Python guards run with
`python3 -m unittest discover -s tools/agent-workflows -p test_check.py -v`
(controlled Rust-output doubles, no model); these doubles establish neither
actual native execution nor independent acceptance.

Changing a method or the shared contract changes its fingerprint. Existing
preparations must apply the new version and repeat validation; earlier evidence
does not become current simply because a file changed. If a native session does
not show a skill, check the root, symbolic links and same-named personal skills,
then reopen the session.

## Guarantees and limits

Methods require separate responsibilities: planners organize, workers implement,
independent reviewers assess, and the engine applies acceptance rules. Reviewing
within the implementing agent's conversation remains a self-review. The contract
prohibits bypassing attempt limits, quotas or missing evidence to report success.

These instructions guide the model; **they do not replace engine enforcement**.
Tests with provider doubles do not demonstrate a real autonomous mission. A method
does not grant additional delegation, publication or deployment authority. Model
selection and credentials remain those of the configured provider/session; this
pack imposes no model or permission mode.

Planner/worker organization is inspired by Cursor's work. Independent review and
acceptance conditions are Swarm's own contract; this pack is not Cursor certification.

## Formats and sources

- Locally adapted from LIA's APEX, audit-pdca, spec-builder, spec-audit, code-reviewer,
  verify-fix, replan and retex-analyzer methods. Machine-specific paths, product
  assumptions and automatic publication workflows have been excluded.
- [Codex skill discovery documentation](https://learn.chatgpt.com/docs/build-skills).
- [Claude project skills and commands documentation](https://code.claude.com/docs/en/skills).
- [Swarm shared contract](../../tools/agent-workflows/CONTRACT.md).

### Local budgets and progress in other scopes

When a subplanner reaches its activation ceiling, its pending input and history
are preserved. The conductor skips it and continues scopes with capacity in both
their own budget and their ancestors' budgets. A local ceiling must not create a
global planning failure. The transactional claim rechecks limits before each call.
When no scope can act, polling consumes no calls and does not mutate the mission.
An exhausted global budget, an unavailable provider and an actual planning failure
remain separate conditions; this rule does not clear them or grant extra attempts.

### Pending handoff order

The conductor selects the owner of the oldest unhandled event in durable inbox
order. A recent coordinator resume message no longer overtakes results already
received by a subplanner. Closed, leased, exhausted or integration-pending scopes
remain ineligible; their events are retained. The claim rechecks limits before
inference. Scheduling does not close scopes on the planner’s behalf or refund
previous attempts.

### Returning a closed child scope to its coordinator

The coordinator context distinguishes owned and delegated requirements.
`task_capacity_remaining` means available creation slots, not unfinished tasks.
`descendant_validation` carries descendant task identities, status, acceptance
freshness checked by the engine, and available review identity and candidate
revision. An accepted label without current evidence yields `accepted_fresh: false`.
This summary does not replace independent review. It lets the coordinator propose
closure, while the engine checks all evidence again before applying it.

### Exceptional recovery after incomplete delivery

After three consumed attempts, `planning authorize-recovery` supports one explicit
operator-confirmed recovery. For a result rejected before paid review, supply
`result_commit` (immutable result SHA) instead of `review_id`, plus `attempt_id`,
`confirm_recovery: true`, a reason and a new corrective instruction. The engine
checks Git attribution, the stopped latest attempt, actual delivery rejection and
reviewer availability. It retains all three attempts and grants at most one more.
Authorization starts no agent and grants no acceptance; checks and independent
review remain required. Repeated grants, replaced results and already-reviewed
results are refused by this route. Available through the CLI and public API; the
existing review-rejection form is unchanged.

### Prepared launch budget

Explicit `agent resume-launch` retains the prepared identity and workspace. When the requested tool-call budget exceeds the unchanged provider ceiling, the engine lowers that budget to the allowed ceiling. Smaller budgets remain unchanged. The original manifest retains the requested budget; the attempt records the effective budget. All other settings, work revision, evidence and provider identity remain checked. Replaying a resume returns the same attempt.

### Inspecting storage created by an older engine

Common inspection commands (`work show/list`, `planning show`, `mission status/preview`, `agent show/list/logs/prepared`, preparation lists/history, workspace and exchange status) refuse to implicitly upgrade older storage. They return `storage_upgrade_required` without changing its version. Use the CLI matching the active server to inspect that mission. For an explicit upgrade, stop older processes first, then run `swarm init` with the newer CLI. Mutating commands and server startup retain their existing migration behavior: do not mix their versions against the same storage.

## Separate planning and review limits

The `planning enable` JSON accepts `max_activations` for planners and
`max_review_calls` for the independent reviewer. For example,
`"max_activations": 12, "max_review_calls": 4` sets separate limits.
`max_review_calls` requires a provider and a value from 1 to 100. Invalid values
or use with another action are rejected before mutation. Omitting it (or using
zero) retains the legacy default: the activation limit, capped at 100.
Configuration does not invoke an AI provider. This does not change existing
mission budgets or refund calls.

## Engine acceptance

See [the six E1–E6 recipes](ENGINE-ACCEPTANCE.md), their requirements and the distinction between deterministic fixtures, browser checks and live-agent trials.

## Structured diagnosis

“Fix a reproducible defect” selects “Diagnose and fix a problem”. The engine embeds
this method for new planners, subplanners and workers. Apply it to anomalies:
reproduce, isolate, test a cause, fix if authorised, verify. Preparation remains
tool-free; the reviewer retains its independent review method. Inclusion and its
fingerprint prove the supplied context, not adherence or successful diagnosis.

The portable version adapts the locally installed Debug skill, without mandatory
connectors, machine paths or dependencies on another installation.

### Verifying the engine during concurrent web work

Embedded web assets and methods currently share the root `resources.go` package:
a web change can invalidate engine-test compilation inputs. To freeze evidence,
a compiler overlay can retain observed assets and their explicit file set without
changing the shared checkout. Declare that view and its hashes; it does not verify
the web interface being edited. Retaining a compiled test binary allows targeted
checks without relinking the package for each command. Distinguish compilation,
collected tests, partial results and final results. See the
[IA3b diagnosis](../plans/actions-confiance/verification-identites-agents-20261016/ia3-issuer-20261017/RAPPORT-IA3b.md).


### Justify a verification deadline

While the code’s limits remain unclear, be cautious with timeouts and other
added test guards: attempts, iterations, volume, concurrency or stop conditions.
Identify their origin, scope and protected risk; observe progress and measure
behavior before adding or tightening a bound. Distinguish slow progress, real
stalls, functional limits and limits of the method. An arbitrary threshold does
not replace diagnosis; existing security, authority and budget protections are
not bypassed.

A test watchdog needs an observable requirement and a justified cost; do not
reuse a number merely because an earlier recipe used it. Distinguish it from
engine policy and performance objectives. Without a justified bound, explicitly
use `-test.timeout=0` to disable Go’s global deadline, follow named events and
diagnose a lack of progress; individual operation contracts still apply. A
timeout is an incomplete execution, not evidence that a business assertion is
false. Splitting a batch may help diagnosis but does not justify the bound.
Preserve original failures and their source/binary binding. The
[IA3c.1 replan](../plans/actions-confiance/verification-identites-agents-20261016/ia3-issuance-receipt-20261017/RAPPORT-IA3c1.md)
retains the user objection, unsupported deadline and its removal.

### Native context evidence

`python3 tools/agent-workflows/check.py --evidence NEW_DIRECTORY` uses permanent
recipe `tools/verification/recipes/agent-workflows-native.json`. It retains
declared Rust sources/embedded texts, commands, original channels and hashes in
a fresh archive. This alone establishes neither a hermetic dependency graph nor
a production profile, identity, model compliance, installation or acceptance.
Inputs are compared before/after and existing proofs are never replaced.

Old Go/delta proofs remain historical. `--evidence-baseline` explicitly refuses:
a Go baseline cannot qualify Rust. Omit it for a full native archive. Historical
Go retention tools are no longer executed by this checker. Without --evidence,
actual tests are displayed but no source archive is retained.

### Historical Go107 evidence

107 locally qualified:4Python retention/replay tests and29Rust consumers
(24admission+5workflow cases) PASS. After an actual body-size regression, exact
sizes are31970/31973/31970/18605bytes with saved32000cap unchanged.5real Go tests/
subtests PASS on1163project sources plus1generated test-main, retained archives/
original JSON and stable before/after inputs. A second run reuses the base with
zero new files.274previously missing inputs are now covered without rewriting
historical evidence. Isolated normal-dev binary has no fixture/test-util;
2actual HTTP flows PASS, no installation. Unsupported GOFLAGS/overlays refuse
this profile; external C/system hermeticity and independent review are unproven.
Original parent-only false coverage and failures retained in107/result.json.
This evidence correction does not complete migration. Next: hierarchical
organization/reviewer guard, configuration/slots/consent and managed actor/gate
before automatic conduction. R6/R7 and RETEX remain open.

Native verification138: the current Root integration profile requires Linux and
the project's Rust toolchain. Evidence mode also runs eight Python guards since141,
including an owned Python child producing an actual SIGTERM status; this does
not prove Cargo cancellation. The signed original status remains diagnostic and
the CLI follows POSIX128+signal. Compilation failures retain their original code.
No temporal ceiling is introduced.

141: the broad native recipe no longer requires the tiny Go module used by the
historical archive tool. Its three tests remain in legacy-go-source-archive.json
(Go required; not executed by the native target). Public role-checker routing
migrated to an owned Cargo shim/Go trap, separate from nine actual Rust tests.
Embedded guidance now points to native evidence; rebuild future contexts while
preserving already issued prompts/processes. The change adds8bytes to the
31970-byte worker body under the unchanged saved32000cap; actual post-build
measurement is still required.

## Local installation by default — user decision, 24 October 2026

Verified authorized development is built, installed and activated in the relevant
local instance by default, without another permission request. This includes
starting/restarting only the identified project service through the supported
delivery path, preserving data, missions and others' processes. Verify the actual
running executable, version, French/English notes and affected flow before
claiming installation. A passing test or compiled candidate is not installed.
Missing/refused delivery wiring triggers replan and corrective work within scope;
retain the evidence and concrete resumption condition. This grants no commit,
push, publication or external/production deployment authority.

Before changing embedded instructions, measure per-role body bytes with `python3 tools/agent-workflows/check_workflow_size.py` before compilation, then verify actual Rust contexts with `python3 tools/agent-workflows/check.py`. Fitting the limit proves neither receipt nor compliance. Preserve the operator policy: remove repetition without losing obligations; insufficient capacity needs diagnosis. Increment1438 showed that an instruction change can break roles while the web service still responds.
