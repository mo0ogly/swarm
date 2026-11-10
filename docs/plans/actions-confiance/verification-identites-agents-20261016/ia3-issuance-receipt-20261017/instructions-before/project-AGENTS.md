# Working on Swarm

Read `tools/agent-workflows/CONTRACT.md` before planning or changing this repository.
It defines the scope, role boundaries, evidence and recovery rules shared by Codex,
Claude and Swarm preparation. User instructions and runtime permissions take precedence.

Continue the authorized objective after status requests and intermediate milestones;
apply the continuity and closure rules in section 3 of the shared contract.

The user explicitly prohibits prolonged silent waits, hours of unmanaged blockage
and repeated claims of a fix without verified results. The user also prohibits
long response delays: acknowledge briefly on receipt, use bounded work steps and
concise tool output, and communicate before an analysis or check finishes. During
active work, provide
a meaningful update at least every 60 seconds: observed state, established cause
or uncertainty, current action and pending evidence. Abnormal slowness triggers
`replan` immediately; do not wait for the user to reactivate supervision. Changed
wording or a raised budget alone never justifies repeating the same failure.
Claim a fix only after the expected outcome is verified through the affected
flow; distinguish local tests, runtime effect and overall acceptance. Instructions
and scheduled wakeups alone do not establish repair or effective supervision.

The user considers every discovered improvement highly valuable. Apply the
recursive improvement loop in section 3 of the shared contract to opportunities
as well as anomalies, including our methods, context transmission and engine.
Retain each useful discovery in the existing plan or follow-up with evidence,
owner and an observable outcome; act within the authorized scope and preserve
concrete resumption conditions for anything deferred. A documented improvement
is not a verified effect and creates no extra authority or budget.

## Common quality standard — Codex and Claude

These rules apply to both assistants. They adapt the development discipline in
Wattson's `.claude/CLAUDE.md` and its rules to Swarm's Go engine and web interface.
This file is the canonical project instruction set; `CLAUDE.md` points here.
Wattson-specific infrastructure, deployment permissions, frameworks and numeric
thresholds are not Swarm requirements. User instructions and assigned runtime
roles take precedence; a method never grants tools, delegation or deployment rights.

### Verify the real contract before writing

- On resume, recover the agreed need, existing plan, checkout, branch, dirty work,
  relevant evidence and running version. Preserve previous work; do not repeat
  completed actions merely because the conversation was compacted.
- Search actual paths and symbols with `rg --files` and `rg`. Before consuming an
  existing interface, read its producer, validation and affected callers: actual
  JSON keys, types, optional values, errors and side effects. Check the persisted
  representation and UX consumer when the change crosses those boundaries.
- Specifications state the required behavior; source and observations establish
  the current behavior. Diagnose discrepancies instead of treating a design,
  comment or conversation summary as proof of implementation.
- Resolve uncertainty through inspection and the smallest discriminating check.
  Ask only for a genuinely missing decision or authority. Do not invent paths,
  fields, commands, versions, metrics or a percentage of confidence.
- For a nontrivial change, choose the applicable method before editing and retain
  a short plan with observable criteria, affected contracts and checks. Keep one
  owner of the objective. Parallel agents or native execution require authorization.

### Correct the cause and preserve safety

- Apply `replan` on every recovery, error, refusal, abnormal slowness or missing
  proof, including failures of our own method. Logging alone is insufficient:
  diagnose, correct within rights or route, verify the effect, then resume the need.
- Fix the shared contract for all affected callers. No exception keyed to a
  mission, identifier, date or error wording. Keep a reproducible counterexample,
  relevant invalid inputs and recovery variants as regression guards. A temporary
  mitigation stays explicitly temporary until the cause and effect are verified.
- Before deferring a serious security, integrity or availability finding, compare
  feasible general, bounded and progressive solutions. Record the chosen action,
  residual risk, owner and resumption condition; escalate only a missing decision
  or authority. Do not silently omit a failed requirement or retry unchanged.
- Validate untrusted inputs at the authoritative boundary: schemas, ownership,
  permissions, freshness, paths and output rendering. Use parameterized SQL and
  existing escaping/path-containment helpers; cover malicious inputs when relevant.
  Never turn a denied or malformed operation into a successful empty result.
- Handle errors explicitly, preserving their cause and safe actionable context.
  Keep secrets, session tokens and private runtime data out of logs and reports.
  Never disable checks, modify a live mission database or reset consumption to
  manufacture success. Use supported public operations for runtime changes.
- Keep modules cohesive and changes reviewable. Plan a structural extraction by
  responsibility and dependency before splitting; preserve its public behavior.
  Do not expand the task into unrelated cleanup or create a forest of source copies.

### Configuration is a product capability

Execution policies, timeouts, retries, budgets, concurrency and retention settings
must have a documented source of truth, persist across restart and be editable in
the Admin settings modal. API, CLI, Go and UX use the same authoritative validation
and effective values. Explain units, defaults, unlimited semantics and consequences;
show consumption separately. Preview must not mutate; application must preserve
history, current reservations and protection against stale updates.

No hidden policy in a local constant, frontend `max` attribute or fallback value.
Paths, endpoints and credentials use the existing configuration mechanisms.
Technical representation or transport bounds are distinct from operator policy:
justify and expose them where they affect behavior; never use them as a disguised
business cutoff. Verify saving, reopening and use by the actual consumer.

### Verify outcomes, including silent failures

- A behavioral correction needs a reproducer that fails for the observed defect
  before the fix and passes afterwards. For public interface changes, specify
  expected behavior in targeted tests first. Existing checks can cover an internal
  refactor; a wording-only edit does not require invented application tests.
- Assert meaningful content and state changes, not merely file existence, HTTP
  200, process exit or absence of exceptions. Cover the no-op/empty/wrong-key case,
  rejection without mutation, recovery and persistence where relevant.
- Verify that the selected tests actually ran: named tests and counts from verbose
  or structured output. Zero collected tests, `[no tests to run]`, interrupted runs
  and failed prerequisites never constitute PASS. Do not edit tested sources during
  compilation or execution; bind results to the exact source and relevant inputs.
- Use isolated fixture roots and the real supported CLI/API/browser path for the
  changed contract. Distinguish local doubles, real providers, independent review,
  public acceptance and the binary actually serving the application. A local green
  test does not establish deployment, independent approval or autonomous behavior.
- A required failing check blocks that claim of completion, regardless of a green
  average or other passing checks. Keep original failures; broaden tests only for
  a concrete affected dependency, new failure or unresolved concern.

### Documentation and definition of done

Every modification includes its relevant documentation in the same increment,
without waiting for a reminder: code, configuration, methods and instructions.
If no documentation change is needed, retain the concrete reason in the existing
tracking record. Preserve actual behavior, evidence, limits and remaining work.

A feature or behavioral fix includes implementation, targeted regression checks,
current contract documentation and a recipe verifying the observable result,
without another request from the user. Scale this to the change; use existing
recipes rather than duplicating them. Continue through the agreed final RETEX.

Update relevant `docs/` and `docs/en/` pages together when a shared contract changes.
Explain purpose, examples, inputs/outputs, constraints, role ownership, errors and
recovery. Use Mermaid for significant architecture, event flow and concurrency;
label target or unimplemented behavior explicitly. Verify every named component
and every implementation/runtime claim against actual source or observations.
Keep operational documentation about current behavior; put incident chronology,
original proofs and interim status in the existing plan/research/report area.
Regenerate affected embedded/generated artifacts through their existing tools.

For a distributed correction, trace incident reception → supervisor decision →
authorized engine application → observed effect. Instructions or logs alone do
not prove this function. The supervisor owns business decisions and completion;
the engine verifies the contract and required evidence and applies authorized
decisions. Deterministic fixture coverage and native runtime evidence stay distinct.

Report what changed, what was actually checked, where the evidence is and any
unverified criterion. When showing progress, give the denominator and distinguish
implemented, verified and accepted work; never infer a percentage from elapsed time
or test volume. Keep updates short and understandable during sustained work.

After a verified correction, capture the reusable lesson in the relevant shared
rule or existing architecture/research note: context, cause, proof, prevention and
limits. Consolidate duplicates and replace obsolete advice without altering
historical evidence. Confirm that intended readers/roles receive it; a repository
instruction update alone does not change the engine's embedded agent contexts.

### Git and running environments

Changes go through repository sources and the project's supported delivery path.
Never patch a deployed binary/container or private runtime state to bypass that
path. Check the actual toolchain and environment rather than copying Wattson
Python, Docker, MkDocs or CI commands into Swarm.

Commit, push, deployment, process interruption and destructive cleanup require
applicable session authority; do not ask again when it is already granted. Preserve
unrelated dirty work and others' processes. When committing, keep each independently
reversible critical fix reviewable; group only genuinely dependent changes and state
why. Use the configured Git identity, never invent attribution or add
`Co-Authored-By` lines, including inherited messages. Do not rewrite published
history or force-push without explicit authority.

## Methods

Project skills are in `.agents/skills/`. They link to the canonical files under
`.claude/skills/`; edit those canonical files, not a second copy.

| Need | Skill |
| --- | --- |
| Deliver a bounded change, from analysis through verification | `apex` |
| Audit a system and optionally correct confirmed findings | `audit-pdca` |
| Turn a need into requirements and a dependency-aware plan | `spec-builder` |
| Find contradictions or omissions before executing a plan | `spec-audit` |
| Review an implementation for actionable defects | `code-reviewer` |
| Diagnose a failure before a scoped correction | `debug` |
| Reproduce a fix through the actual CLI, API or web flow | `verify-fix` |
| Recover a blocked plan without erasing history or limits | `replan` |
| Explain outcomes and prioritize evidence-based improvements | `retex-analyzer` |
| Frame a complete application with journeys, stories and design prerequisites | `product-planning` |
| Research, design and implement an assigned product slice | `product-delivery` |
| Review product coverage, design and candidate evidence | `product-review` |

Choose the smallest relevant method. Do not chain every method for a trivial edit.
See `docs/AGENT-METHODS.md` (French) or `docs/en/AGENT-METHODS.md` (English).

## Modèles Swarm

Toute création ou modification d’un modèle Swarm doit utiliser la skill
[`swarm-model-design`](.claude/skills/swarm-model-design/SKILL.md), disponible
comme `$swarm-model-design` pour Codex et `/swarm-model-design` pour Claude.
Lire et appliquer cette skill avant de concevoir le modèle ou modifier le catalogue.
Distinguer le modèle de cadrage du modèle de travail exécutable. Pour ce dernier,
vérifier les actions, dépendances, orientations de décision, validations et reprises
avec les opérations publiques réellement prises en charge par le moteur.
La création d’un modèle ne vaut ni adoption du plan ni autorisation d’exécution.

## Web interface

Read `docs/PRODUCT-ARCHITECTURE.md` before planning product navigation or frontend
architecture. The Projects and Business Processes UX share the Swarm engine but
must remain independently buildable and deployable. Wattson is outside this scope.

Read `docs/UI-DESIGN.md` before changing web layout, navigation, themes or reading accordions.
Keep the logo palette, shared semantic tokens, FR/EN labels and progressive disclosure consistent.

Use Swarm's shared components and semantic background/text pairs in both `etat`
and `sombre`; add new palette values only to the shared design tokens. Keep page,
card, subsection, editable field and focus visibly distinct. Reserve a strong
action style for the primary action; convey statuses through text as well as color.
Target WCAG AA contrast (4.5:1 for normal text, 3:1 for large text and applicable
controls), visible keyboard focus and readable disabled/read-only states.

Use local versioned assets and existing styles/listeners, with no CDN, remote font,
inline event handler or ad hoc colored inline style. Do not import Wattson's
Bootstrap framework or require a skill absent from this repository. Put long help
behind an explicit labeled control using the existing dialog/disclosure patterns;
keep essential errors and blocking reasons visible in the main flow.

Settings and help dialogs require a visible title, accessible labeling, a clear
close action, keyboard navigation, Escape handling and focus restoration. Verify
rendered surfaces, loading/empty/error/success states, FR/EN, both themes and the
relevant mobile widths. Inspect console errors, failed requests, overflow and
actual interaction results. A token scan or screenshot alone is insufficient;
state explicitly when visual or browser verification is unavailable.

## Repository checks

- Select verification from the changed contract and its affected dependencies.
  For Go engine changes, run the relevant targeted `go test` cases, including
  failure and recovery variants, plus appropriate static checks and compilation.
  Do not rerun the entire package or `go test ./...` after each local correction.
  Broaden coverage only when a changed shared contract, a new failure or an
  unresolved risk provides a concrete reason; record that reason first.
  Reuse passing results while their relevant source and execution inputs remain
  unchanged. Use targeted `go test -race` when synchronization or shared-state
  behavior changes. Do not treat an interrupted or failed full suite as passing.
- Frontend: use the scripts in `package.json` and the applicable browser recipes
  under `tests/`. `go test` does not execute those browser recipes.
- Configuration: `python3 tools/agent-workflows/check.py`.
- All changes: `git diff --check` and inspect the final diff.
- Never use a running mission's database as a test fixture. Build/test against an
  isolated temporary root. A build does not prove the running server uses it.

Inspect the current checkout and installed capabilities before running commands.
Do not assume a particular absolute directory, port, provider, model or quota.

## Direct development workspace

For the current Cursor engine evolution, the user selected direct development
by Codex alone, without Claude or native Swarm orchestration. The canonical
checkout is `/home/fpizzi/Documents/ChatGPT/swarm`; `/home/fpizzi/workspace/swarm`
is a compatibility link for retained history and managed copies. Preserve the
paused native mission and its evidence. Work directly in this checkout; do not
create another full source copy for each local correction. Use isolated fixture
roots for tests, and retain original failure evidence in one verification area.
Do not delete a suffixed checkout, registered worktree or retained proof because
of its name; identify its ownership and unique changes first.

Every new execution policy must be persisted and editable through an Admin
settings modal, with the same validation and values in API, CLI and Go. Do not
hide a second policy ceiling in Go constants or frontend input attributes.
Keep policy, current consumption and actual transport limitations distinct.

## Requested completion through RETEX

When the agreed project includes a final retrospective, continue the authorized
plan through the RETEX task. Status, documentation, a passing check or an
intermediate delivery does not close that objective. Diagnose and correct or
route failures within existing rights and budgets, then return to the plan.
Preserve acceptance evidence and user choice of subsequent project evolutions;
a retrospective is not a substitute for unfinished original requirements.
If no useful authorized action remains, retain the concrete cause, evidence,
owner and resumption condition rather than inventing completion or extra rights.

For the current Cursor engine evolution, the user explicitly authorized increasing
Swarm rights and budgets when insufficiency prevents progress. Diagnose the actual
shortfall and use supported public configuration/operations, preserving consumption
and history. This authority does not fabricate acceptance or override provider
account limits; it is scoped to this authorized project.
