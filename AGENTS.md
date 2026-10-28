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

## Contrôle continu : politiques, documentation et erreurs

À chaque incrément de migration ou de développement, conserver une revue explicite
liée aux octets des fichiers réellement modifiés, sur trois axes :
- Politiques codées en dur : examiner constantes, valeurs littérales, defaults,
  fallbacks, booléens, tentatives, délais et conditions d'arrêt. Distinguer entrée
  de test, invariant de protocole, limite technique et choix opérateur ; justifier
  la catégorie. Pour un choix opérateur, tracer source persistée → validation →
  Admin/API/CLI → consommateur réel, avec témoin, modification et reprise.
- Documentation des fonctions : inventorier les fonctions publiques/non triviales
  créées ou modifiées, vérifier le contrat dans leurs docstrings/Rustdoc/commentaires
  et les documents concernés ; compiler la documentation quand pertinent. La simple
  présence d'un commentaire ou une compilation verte ne prouve pas sa justesse.
- Erreurs : suivre chaque issue d'échec nouvelle/modifiée jusqu'au consommateur,
  vérifier cause conservée, diagnostic sûr, état avant/après, sortie/flush,
  annulation et reprise pertinentes. Chercher aussi erreurs ignorées et fallbacks
  qui pourraient transformer un refus en succès vide.

Utiliser des recherches automatiques pour produire des candidats, puis une revue
sémantique avec preuves ; aucun scanner ne garantit l'absence de défaut. Un élément
non examiné, une exception non justifiée ou un contrôle requis en échec reste ouvert
et interdit de déclarer ce contrat conforme. Refaire la revue lorsqu'un fichier ou
son contrat consommé change ; conserver la portée exacte des preuves réutilisées.
Vérifier aussi les lots antérieurs concernés par une découverte, sans réécrire leur
historique. Cette règle s'applique à toute la migration, pas seulement au dernier lot.

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
  A service retaining project authority must bind operations to the opened
  datastore instance, with an owned descriptor where inode reuse matters;
  a pathname or common UID alone is insufficient. This is not OS confinement.
- Handle errors explicitly, preserving their cause and safe actionable context.
  Keep secrets, session tokens and private runtime data out of logs and reports.
  Never disable checks, modify a live mission database or reset consumption to
  manufacture success. Use supported public operations for runtime changes.
  Carry observed effects through every release/render/close/output layer:
  distinguish known recorded/replayed state, visible uncertain publication and
  unknown outcomes. An Err alone never proves no change. Verify each consumer.
  Reuse and test the established wrapper matrix when adding error traversal;
  source() alone can skip an IO custom error, and secondary cleanup causes matter.
  See Rust migration90's retained counterexamples and current error contracts.
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

- Be cautious with timeouts and other test guards while the code's limits are
  not understood. Identify their origin, scope and protected risk; observe
  progress and measure behavior before adding or tightening a bound. This also
  covers attempt, iteration, volume and concurrency caps or other test stop
  conditions. Distinguish slow progress, a real stall, functional limits and
  limits of the verification method. Do not substitute an arbitrary threshold
  for diagnosis or bypass existing protections; preserve evidence and authority.
- Justify verification deadlines from an observable contract or measurements;
  never inherit an arbitrary whole-test limit from an earlier recipe. Without
  a justified bound, explicitly disable the test runner's global watchdog,
  follow progress and diagnose stalls. A timeout is incomplete evidence, and
  splitting a batch does not justify its original bound. This does not authorize
  removing contractual deadlines, budgets or operator policies.
- A behavioral correction needs a reproducer that fails for the observed defect
  before the fix and passes afterwards. For public interface changes, specify
  expected behavior in targeted tests first. Existing checks can cover an internal
  refactor; a wording-only edit does not require invented application tests.
- Assert meaningful content and state changes, not merely file existence, HTTP
  200, process exit or absence of exceptions. Cover the no-op/empty/wrong-key case,
  rejection without mutation, recovery and persistence where relevant.
- Bind results to actual build profiles, feature unification, dependencies,
  environment, arguments and exercised binaries. An all-features workspace may
  contain fixture code and never establishes the delivered profile. Keep fixture
  and production binaries separate; verify matching build hashes and the public
  flow on the intended profile. Before running a derived recipe, inspect its final
  docstring, commands, counts and promised evidence for inherited stale claims.
  For a UI served by an executable, verify actual protected HTTP asset bytes
  against the effective compiler inventory and the current engine catalogue.
  A workspace component test calling a CLI does not prove embedded resources,
  translations or the interface served by that binary.
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

New or modified public/nontrivial functions include current docstrings, Rustdoc
or Go documentation comments in the same increment. Describe purpose, inputs and
preconditions, outputs, errors, side effects and material limits, including any
unverified authority or evidence. Explain the contract without restating the code;
update stale comments and check documentation links/executable examples.

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

## Local delivery version and release notes

Every delivered change must have an identifiable running version and current
French/English evolution notes in `version_history.json` (`local_changes`).
Use `./swarm.sh build` / `./swarm.sh restart` for a source-managed local server:
the launcher numbers changed effective build inputs before stopping the previous
server, and preserves the number for unchanged inputs. Do not leave an old
configured source binary running while claiming the delivery is installed.
After an authorized restart, verify the actual server identity and the **Version
and what's new** dialog: number, evolution notes, actual Git commit link and
uncommitted-change disclosure. Keep local deliveries distinct from published
releases; never invent a commit/tag or publish automatically. An external
prebuilt binary selected with `--binary` stays external and is not rebuilt.
Pour sélectionner exactement le dernier candidat source déjà qualifié et vérifié,
`./swarm.sh restart --prepared-build SHA256 --no-open` conserve le chemin géré et
revérifie image, profil, archive et métadonnées avant arrêt. Fournir l’empreinte
complète du record examiné ; un autre candidat préparé ou une régression de version
refuse. `start` avec cette option reste idempotent seulement pour la même image
active. Les sources plus récentes restent en attente, jamais déclarées installées.
Le redémarrage sans option garde sa reconstruction normale. Cette sélection ne
qualifie aucun build refusé et ne crée aucune autorisation de publication.

## Installation locale par défaut — décision utilisateur du 24 octobre 2026

Pour un développement autorisé, toute avancée vérifiée doit être construite,
installée et activée dans l’instance locale concernée par défaut, sans nouvelle
demande d’accord. Cette autorisation couvre le démarrage ou redémarrage du seul
service local identifié comme appartenant au projet. Utiliser le chemin de
livraison pris en charge ; préserver données, missions, travaux et processus
d’autrui. Vérifier l’identité du binaire réellement exécuté, la version, les
évolutions FR/EN et le parcours affecté avant d’annoncer « installé ».
Un test réussi ou un candidat compilé ne constitue pas une installation.
Si le raccordement de livraison manque ou refuse, appliquer replan et le traiter
dans le périmètre autorisé ; conserver le défaut, sa preuve et sa condition de
reprise sans prétendre que le produit est installé. Cette préférence ne vaut pas
autorisation de commit, push, publication ou déploiement externe/production.

## Methods

Project skills are in `.agents/skills/`. They link to the canonical files under
`.claude/skills/`; edit those canonical files, not a second copy.

| Need | Skill |
| --- | --- |
| Deliver a bounded change, from analysis through verification | `apex` |
| Audit a system and optionally correct confirmed findings | `audit-pdca` |
| Turn a need into requirements and a dependency-aware plan | `spec-builder` |
| Discover sourced model prices for a versioned catalogue | `pricing-discovery` |
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

## Permanent verification recipes — Claude and Codex

Swarm has a permanent verification tool: `tools/verification/recipe.py`.
Keep reusable recipe manifests under `tools/verification/recipes/` and their
regression code/fixtures in repository sources; `.cache/` must never be the sole
copy of a newly maintained recipe. Update the applicable recipe with its changed
contract and documentation. Use explicit named assertions, refusal and recovery
cases; a zero-test run or exit status alone does not establish success.

Run into a fresh evidence directory, preserving original failures, argv, declared
inputs, profiles and stdout/stderr. Verify retained evidence with the tool's
`verify` command; never overwrite past results to manufacture a replay. Its
source inventory covers declared inputs only: compiler graphs, effective features,
production binary identity and independent acceptance need their own checks.
See `tools/verification/README.md` and `docs/plans/migration-rust/REPLAYS.md`.
This rule/tool grants no execution, provider, live-state or deployment authority.
Repository rules reach Claude through `CLAUDE.md`; they do not automatically
update embedded native agent contexts or already running sessions.

When chaining dependent local steps, stop after a failed prerequisite (for example
an edit script) before compiling or testing. Preserve partial edits and diagnose
the failed stage; a shell continuing after an edit error is not a new precondition
for rerunning the same behavioral failure. Read actual source boundaries rather
than assuming punctuation preserved by a formatter.

## Repository checks

- For a targeted Rust unit regression, use `cargo test -p PACKAGE --lib MODULE::TEST`
  (with the actual manifest path). For an integration regression, name its
  `--test TARGET`. A name filter alone also compiles/launches unrelated integration
  targets; their zero-test results are not evidence. Reuse passing unchanged
  results; do not repeat compilation merely to replace an earlier broad command.
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
- Before changing embedded workflow instructions, measure role sizes with
  `python3 tools/agent-workflows/check_workflow_size.py` before Cargo, then run
  the real Rust workflow check. Preserve the configured body policy and full
  obligations; remove repetition rather than raising caps to hide growth.
- Configuration: `python3 tools/agent-workflows/check.py`.
  For retained proof, add --evidence with a fresh project-local directory and
  optionally --evidence-baseline with a verified directory, to capture actual Go
  root/package sources and embeds, raw named tests and before/after stability.
  Default checks do not establish complete Go source retention.
- All changes: `git diff --check` and inspect the final diff.
- Never use a running mission's database as a test fixture. Build/test against an
  isolated temporary root. A build does not prove the running server uses it.

Inspect the current checkout and installed capabilities before running commands.
Do not assume a particular absolute directory, port, provider, model or quota.

### Rust migration qualification reviews

Read the current migration plan before choosing the baseline. Shared qualification
collectors require `SWARM_MIGRATION_REVIEW_BASELINE` pointing to the previous exact
input inventory and a fresh `source-review.json` in the new evidence output before
closure. Review policies, functions, errors and documentation against current
source/proof hashes; missing/stale/open declarations refuse qualification. Read
`tools/rust-migration/review_increment.py` and FR/EN migration architecture for the
format and limits. A changed baseline must not hide an unresolved finding: carry
historical issues in the plan and reexamine affected earlier lots. Increment review
and functional tests do not establish global conformance or independent acceptance.

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
