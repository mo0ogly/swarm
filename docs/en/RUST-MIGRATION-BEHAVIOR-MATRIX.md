# Migration matrix: entry points and first behavior slices

## Current-source correction —27 October2026

The196 tables below are historical inventories. Their absence labels are not
current migration status. Source reinspection confirms these routed entries;
routing alone does not establish Go parity or global acceptance.

| Entry | Current source | Evidence / remaining |
|---|---|---|
| `agent logs` | agent.rs → agent_logs.rs | Historical rows/output implemented; see permanent agent log recipes. |
| `budget show`, `budget preview` | lib.rs → budget_history.rs | Historical producer connected; see budget history recipe. |
| `mission watch` | mission.rs → mission_watch.rs | Native incremental conductor connected; simultaneous role planning remains explicitly deferred in its source. |
| `mission status`, `changes`, `spending` | mission.rs → mission_status.rs | Historical aggregate under its producer guards; no inference from native-only observations. |
| `planning history` | lib.rs → planning_history.rs | Recorded decisions reader; no historical execution authority. |
| `mission seen` | mission.rs → session_visit.rs | Delivered219 through existing visit writer,17public assertions and installed20/Version14/page11; bounded evidence in MISSION-SEEN-20261027. |
| `agent history` | agent.rs → agent_history.rs | Delivered233: inert imported Bundle.Cockpit;102public assertions/45differential cases,3generator tests and installed31/Version14+8specific checks. See AGENT-HISTORY-20261027. |
| `agent list` | agent.rs refuses | Missing composite cockpit snapshot; `list_agents` alone does not establish this contract. |

## Update196 — integrated parallel work

[Matrice intégrée](../plans/migration-rust/evidence/20261022-parallel-196/matrix-integrated.json). **83 CLI operations across10 families are now detailed**, alongside137 public entries; these overlapping levels are not additive.93 HTTP paths have clauses/consumers/gaps and27 retained Go originals, versus7 hashed route sources in194. No global percentage or accepted global parity.

The38 additional operations comprise19 same-name entries,5 renamed/partial equivalents and14 without an identified native operation. Collector hashes were refreshed after only three queue help strings changed; dispatch function bytes are identical.

| Reference | State | Candidate Rust consumer | Remaining |
|---|---|---|---|
| `work create` | ENTRY_PRESENT_PARITY_OPEN | work create | Request fields, rendered view/events, exact JSON/plain output, revision/replay, refusal and postcommit output recovery require behavioral qualification. |
| `work list` | ENTRY_PRESENT_PARITY_OPEN | work list | Request fields, rendered view/events, exact JSON/plain output, revision/replay, refusal and postcommit output recovery require behavioral qualification. |
| `work show` | ENTRY_PRESENT_PARITY_OPEN | work show | Request fields, rendered view/events, exact JSON/plain output, revision/replay, refusal and postcommit output recovery require behavioral qualification. |
| `work update` | ENTRY_PRESENT_PARITY_OPEN | work update | Request fields, rendered view/events, exact JSON/plain output, revision/replay, refusal and postcommit output recovery require behavioral qualification. |
| `task add` | ENTRY_PRESENT_PARITY_OPEN | task add | Task fields/dependencies/evaluations and mutation/replay/refusal/output contracts require behavioral qualification; Root presence is not parity. |
| `task update` | ENTRY_PRESENT_PARITY_OPEN | task update | Task fields/dependencies/evaluations and mutation/replay/refusal/output contracts require behavioral qualification; Root presence is not parity. |
| `agent attach` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent context-preview` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | agent preview | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent history` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent list` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent logs` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent preflight` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | agent preview (not Go preflight DTO) | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent prepared` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent reconcile` | ENTRY_PRESENT_PARITY_OPEN | agent reconcile | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent resume-dialogue` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent resume-dialogue-preview` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent resume-launch` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent show` | ENTRY_PRESENT_PARITY_OPEN | agent show | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent start` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | agent prepare + agent run (two distinct operations) | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `agent stop` | ENTRY_PRESENT_PARITY_OPEN | agent stop | Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification. |
| `providers cooldown clear` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Configuration is renamed; Go cooldown and relay actions are not routed here. Native provider policies are a separate contract, not proven equivalents. |
| `providers cooldown show` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Configuration is renamed; Go cooldown and relay actions are not routed here. Native provider policies are a separate contract, not proven equivalents. |
| `providers init` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | providers config init | Configuration is renamed; Go cooldown and relay actions are not routed here. Native provider policies are a separate contract, not proven equivalents. |
| `providers relay decide` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Configuration is renamed; Go cooldown and relay actions are not routed here. Native provider policies are a separate contract, not proven equivalents. |
| `providers relay show` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Configuration is renamed; Go cooldown and relay actions are not routed here. Native provider policies are a separate contract, not proven equivalents. |
| `providers show` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | providers config show | Configuration is renamed; Go cooldown and relay actions are not routed here. Native provider policies are a separate contract, not proven equivalents. |
| `pricing estimate` | ENTRY_PRESENT_PARITY_OPEN | pricing estimate | Catalogue/save/quote routes exist; exact units, rational/float representations, missing rates, conflicts, output and recovery parity remain open. |
| `pricing list` | ENTRY_PRESENT_PARITY_OPEN | pricing list | Catalogue/save/quote routes exist; exact units, rational/float representations, missing rates, conflicts, output and recovery parity remain open. |
| `pricing save` | ENTRY_PRESENT_PARITY_OPEN | pricing save | Catalogue/save/quote routes exist; exact units, rational/float representations, missing rates, conflicts, output and recovery parity remain open. |
| `quotas apply` | ENTRY_PRESENT_PARITY_OPEN | quotas apply | Show routes to quota projections in both implementations; exact fields, preview/apply effects and error/output/replay contracts remain open. |
| `quotas preview` | ENTRY_PRESENT_PARITY_OPEN | quotas preview | Show routes to quota projections in both implementations; exact fields, preview/apply effects and error/output/replay contracts remain open. |
| `quotas show` | ENTRY_PRESENT_PARITY_OPEN | quotas show | Show routes to quota projections in both implementations; exact fields, preview/apply effects and error/output/replay contracts remain open. |
| `budget apply` | ENTRY_PRESENT_PARITY_OPEN | budget apply | Native CLI exposes only apply. Go show includes revision, budget and coverage; preview reports remaining/unlimited/new-launch advice. Internal cost APIs do not supply these CLI contracts. |
| `budget preview` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native CLI exposes only apply. Go show includes revision, budget and coverage; preview reports remaining/unlimited/new-launch advice. Internal cost APIs do not supply these CLI contracts. |
| `budget show` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Native CLI exposes only apply. Go show includes revision, budget and coverage; preview reports remaining/unlimited/new-launch advice. Internal cost APIs do not supply these CLI contracts. |
| `task-model apply` | ENTRY_PRESENT_PARITY_OPEN | task-model apply | Native show emits Work; Go show adds available_models to flattened Work fields. Preview/apply validation and actual launch consumption require qualification. |
| `task-model preview` | ENTRY_PRESENT_PARITY_OPEN | task-model preview | Native show emits Work; Go show adds available_models to flattened Work fields. Preview/apply validation and actual launch consumption require qualification. |
| `task-model show` | ENTRY_PRESENT_PARITY_OPEN | task-model show | Native show emits Work; Go show adds available_models to flattened Work fields. Preview/apply validation and actual launch consumption require qualification. |

P01 progresses:885 current Go sources and mod/sum retained, including71 modified existing files and53 additions since553593a. This is not an approved baseline or asset/compiler/overlay inventory. Initial source01 is limited; source15 is current after reproducing/fixing added-file drift and binding the imported publisher.

Native candidate196 adds mission queue: coherent turns/blockers observation, exact decimal ordinals, no-mutation refusals and restoration. Six Rust tests, engine/CLI Clippy/Rustdoc and public recipe PASS; no acquire/release/enforced FIFO follows.13 collector tests and the inventory-race guard are separate from business tests. The other45 operations below retain their detail; matrix12 observes them against CLI196.

---

22 October2026, owner Codex. Historical553593a baseline; uncommitted Go reference changes are not frozen. This table replaces narrative lists within this scope, not historical evidence.

[JSON et sources exactes](../plans/migration-rust/evidence/20261022-pilotage-195/matrix04/matrix.json) · [Recette permanente](../plans/migration-rust/evidence/20261022-pilotage-195/permanent05/result.json).

**137 public entries**:44 CLI names and93 HTTP paths. **45 separately examined operations**:11 mission and34 planning. Do not add these overlapping levels to derive a percentage. No behavior is globally parity-accepted here. Other subcommands, HTTP methods, UI/data/error/recovery flows still need decomposition.

## States and evidence

- `ENTRY_PRESENT_PARITY_OPEN`: Rust entry found, full parity unproved.
- `SAME_ENTRY_ABSENT_REPLACEMENT_OPEN`: same entry absent; reusable components and renamed replacements need mapping.
- `PARTIAL_CONTRACT_DIFFERENT`: native operation with a different request/output/contract.
- `PARTIAL_RENAMED_VIEW`: partial renamed view, not the historical aggregate.
- `NO_NATIVE_OPERATION` / `NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN`: public operation not connected.
- `EXPLICITLY_UNPORTED`: explicit CLI refusal.

The27 absent CLI names have native refusal proof in194, not positive parity. CLI/mission consumers match normal193 hashes. Current web dispatcher differs from candidate193: these are source observations, not candidate network tests. Delegated static assets and two nonhistorical OTEL routes are not replacements for93 APIs. Handoffs183–193 remain dated scoped evidence; their existence closes no row in this matrix.

## Mission

| Reference | State | Candidate Rust consumer | Concrete gap |
|---|---|---|---|
| `mission status` | PARTIAL_RENAMED_VIEW | `mission show` | show exposes controls, not the complete Go missionStatus aggregate/guidance/changes/spending/supervision view |
| `mission changes` | NO_NATIVE_OPERATION | — | No same operation; map any reusable internal components before porting public contract |
| `mission spending` | NO_NATIVE_OPERATION | — | No same operation; map any reusable internal components before porting public contract |
| `mission seen` | NO_NATIVE_OPERATION | — | No same operation; map any reusable internal components before porting public contract |
| `mission recovery` | NO_NATIVE_OPERATION | — | No same operation; map any reusable internal components before porting public contract |
| `mission preview` | NO_NATIVE_OPERATION | — | No same operation; map any reusable internal components before porting public contract |
| `mission start` | PARTIAL_CONTRACT_DIFFERENT | `mission start` | Native audited revision/event request and Work output differ from Go LaunchProfile/missionStatus; exact JSON/plain/refusal/effect parity remains open |
| `mission pause` | PARTIAL_CONTRACT_DIFFERENT | `mission pause` | Native audited revision/event request and Work output differ from Go LaunchProfile/missionStatus; exact JSON/plain/refusal/effect parity remains open |
| `mission resume` | PARTIAL_CONTRACT_DIFFERENT | `mission resume` | Native audited revision/event request and Work output differ from Go LaunchProfile/missionStatus; exact JSON/plain/refusal/effect parity remains open |
| `mission stop` | PARTIAL_CONTRACT_DIFFERENT | `mission stop` | Native audited revision/event request and Work output differ from Go LaunchProfile/missionStatus; exact JSON/plain/refusal/effect parity remains open |
| `mission watch` | EXPLICITLY_UNPORTED | — | Complete reconcile/dispatch/watch loop and interruption semantics absent |

## Planning

| Reference | State | Candidate Rust consumer | Concrete gap |
|---|---|---|---|
| `planning authorize-recovery` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning bundle` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning claim` | ENTRY_PRESENT_PARITY_OPEN | `planning claim` | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning cleanup` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning cleanup-preview` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning configure-reviewer` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning decide` | ENTRY_PRESENT_PARITY_OPEN | `planning decide` | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning diagnose-review` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning enable` | ENTRY_PRESENT_PARITY_OPEN | `planning enable` | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning extend-attempt` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning fragment-preview` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning handoff` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning history` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning judge` | ENTRY_PRESENT_PARITY_OPEN | `planning judge` | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning pause` | ENTRY_PRESENT_PARITY_OPEN | `planning pause` | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning post-delivery` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning recovery-preview` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning reject-instructions` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning renew` | ENTRY_PRESENT_PARITY_OPEN | `planning renew` | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning requalify` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning restart-task` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning resume` | ENTRY_PRESENT_PARITY_OPEN | `planning resume` | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning retry-integration` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning retry-review` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning review-cost` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning review-dossier` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning review-step` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning review-timeout` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning revise-recovered-result` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning scope-patch` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning scope-preview` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning show` | ENTRY_PRESENT_PARITY_OPEN | `planning show` | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning step` | ENTRY_PRESENT_PARITY_OPEN | `planning step` | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |
| `planning submit-recovered-result` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence |

Four present mission operations (start/pause/resume/stop) use audited event/revision requests and return Work; historical Go uses its own inputs then missionStatus output. This is not output parity. show does not restore the complete status aggregate. order, priority and unpause are additional Rust operations, not ports of the missing five.

## Reference public entries

| Stable ID | State | Inspected Rust source |
|---|---|---|
| `cli:agent` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:budget` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:checkpoint` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:evaluate` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:gate` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:init` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:mission` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:ooda` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:planning` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:pricing` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:providers` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:quotas` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:resume` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:task` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:task-model` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:web` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:work` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:aide` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:automation` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:autonomy` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:connections` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:console` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:control` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:dispatch` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:doctor` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:evidence` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:exchange` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:export` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:help` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:import` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:laboratory` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:lifecycle` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:memory` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:plan` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:prepare` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:process` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:profile` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:project-profile` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:role-model` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:run-limits` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:skills` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:trust` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:validation` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:version` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `http:/api/v1/action` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/planning` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/quotas` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/works` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/activity` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/agent-detail` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/ask` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/cancel` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/context` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/export` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/meta` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/turns` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/cancel` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/config` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/create` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/external` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/programs` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/state` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/control-diagnostic` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/dialogue-resume` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/dialogue-resume-preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/events` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/exchanges` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-drafts` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-drafts/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-drafts/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-performance` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-performance/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-performance/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/lifecycle/missions` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/logs` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/planning-bundle` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/planning-cleanup` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/planning-history` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparation-timeout` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparation-timeout/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparation-timeout/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/budget` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/command` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/conversion` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/dialogue` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/files` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/history` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/methods` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/preflight` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/providers` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/send` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/show` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/source` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/stop` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/template-check` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/templates` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/pricing` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/pricing/estimate` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/processes` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/project-memory` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/project-profile` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/project-profile/catalog` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/project-profile/select` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/provider-wait` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/provider-wait/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/provider-wait/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/admin` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/connections` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/connections/test` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/policies` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/resolve` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/recovery-preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/report` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/retex-evolution` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits/effective` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits/rollback` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/runtime-health` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/skills` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/snapshot` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/task` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/terminal` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/terminal/control` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-actions` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-actions/oidc-backchannel-logout` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-evidence` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-laboratory` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-secrets` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/visit` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/web-requests` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/web-requests/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/web-requests/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/web-requests/script.js` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |

## Replay and limits

```sh
python3 tools/rust-migration/build_behavior_matrix.py \
  --inventory docs/plans/migration-rust/evidence/20261022-pilotage-194/surface.json \
  --normal-profile-directory docs/plans/migration-rust/evidence/20261022-193/normal04 \
  --output NOUVEAU_REPERTOIRE
```

The migration-behavior-matrix recipe consumes SWARM_MATRIX_INVENTORY and SWARM_MATRIX_NORMAL_DIRECTORY. Stale source, duplicate entries and empty inventory must refuse before valid collection; no PASS matrix is published for those invalid inputs. Three named tests and two recipe steps PASS, stable source and integrity VERIFIED; no watchdog/build/Go/provider/application mutation. Collector is bounded to reviewed planning usage/branches and mission whitelist, not a general parser. Syntax changes require semantic review, not zero-count success.

Next: freeze full P01 scope, deepen P02 other subcommands and P03 HTTP methods/bodies/effects, bind per-behavior proofs, then continue P05/P04/P06. Diagnostic improvements remain separate from functional migration.


## Update197: applied native FIFO, bounded local proof

Two authorized agents delivered queue transitions, candidate selection and scoped
reviews; root integrated the real lease/consent/readiness/Agent lifecycle. Normal39
has292 effective default inputs, no fixture features, not installed. final43–46 (four recipes/six steps) pass; quality21 has17 named Rust tests, strict Clippy and
private Rustdoc. Final integrity is verified; SDK40/41 match normal39.
P05 now includes standing registrations, Binding3 acquisition, safe terminal release
and journaled generations. The pause cleanup finding was reproduced before its fix.
P05 remains partial: all permission variants, uncertain/crash recovery and complete
fairness acceptance are not established. P04/P06 dispatch/watch remain unconnected;
no CLI operation or HTTP parity count increases from these SDK APIs. The137/83/93
inventories remain overlapping reference levels, never a completion percentage.
Extended cleanup/cancellation faults, exact owner/reservation retention, same-workspace
progress and historical v2 execution pass. Unknown/crash/completion fault, terminal
replay after a new same-task generation and four permission variants remain open. Sources and evidence: 20261022-fifo-197 in the plan.

Final197: normal65 SHA8e5af7aa /292 effective default inputs, SDK66/67/68,
quality69 18 named Rust tests/strict Clippy/private Rustdoc. Five final recipes
(completion-after70 and final71–74) pass7 steps, integrity verified. Both pause
cleanup and missing terminal release proof have retained before/after witnesses;
autocommit regression60 was diagnosed and corrected, not waived. Nested outputs
are independently archived/verified in retained76. Production retry61 passes on
normal55 for unchanged observation code. No full P05/global acceptance, runtime
installation, HTTP parity increment or complete dispatch/watch is inferred.

### Qualification198 — next selection/admission

P04 partial: lease-bound Rust dispatch API, original before sync, shared ordering/disjoint candidate, mission capacity stopping the pass, explicit permission refusal. Normal24 ab349179/293inputs; SDK26 matches200engine inputs, twenty tests10 reused on identical executable bodies, Clippy/Rustdoc27, SDK27 Clippy. Public28 checks4scenarios +3original conflicts (manual/foreign Work/revision), without additional SQL on replay/refusal. One recipe/one step PASS/integrity verified,116outputs retained29. Counts do not add to the global matrix. Before04/13 negative namespace guard:1expectedFAIL then fix and passing guard among20; no actual BUSY. P04/P05/P06 and all18deliverables remain open; no watch/installation/unported-branch parity.

### Increment199 — independence prerequisite verified, product watch open

Public23: two PASS steps/VERIFIED integrity, twelve guards and two actual inert
flows (graceful conductor exit then SIGTERM). The same worker survives, actual
CLI waitpid and completion originals match public output, reservation settles,
FIFO releases and replay neither reruns a provider nor accepts a task. Default
SDK10 matches all 200 engine inputs from normal198/24; three Rust tests08 and
strict Clippy11 PASS. Retained24 preserves 115 outputs without Root databases.
This B1 prerequisite advances P06; none of the 18 global deliverables is closed.
B2 wiring remains, including a safe session route compatible with forbid unsafe.
P07 analysis retains four separate missing axes: chain budget, temporal
eligibility/multiplier, report clock tolerance and relay count/cadence; the cost
threshold is already ported. No new operator policy, installation or combined
Live qualification is inferred.

### Increment200 — safe product bootstrap verified, conductor remains open

Normal05 c734f0c4: 294 default inputs without fixtures; SDK10 shares all 200 engine
sources. Five CLI06 tests, three SDK11 tests and strict Clippy/Rustdoc12 PASS.
Public14: two PASS steps/VERIFIED integrity, fourteen guards, four public
pre-Root refusals then actual graceful/SIGTERM via the unsafe-free product
helper. Retained15 preserves 129 outputs including originals and kernel statuses.
This prerequisite does not deliver the watch driver or descriptor-bound Root
channels; no P01–P18 closure or inferred percentage. Constant-path .swarm
replacement needs verification before wiring; not yet reproduced here. No
installation, combined Live qualification or overall acceptance.


## Increment201 — measured Root identity refusal and recovery

Normal65 `8bd7ebc0` (301 effective default inputs, no fixture feature) and SDK77
share 201 engine inputs plus Cargo configuration. The 67 named Rust tests,
Clippy/Rustdoc85, directory88/DB-only89 public injections, graceful/SIGTERM98
and genuine v1/103–v2/104 compatibility pass within their declared scopes.
Recipe integrity and nested retention90/99/105 are verified. No user datastore
or paid model is involved; these counts close none of the18 global deliverables.

Under an exclusive lock, CLI92 reports Prepare/SQLite5 in4440712ns, then reopens
with unchanged SQL/DB/settings; before61 measured5017838602ns. There is no latency
threshold or SLA. Tests74 establish zero-handler ordering and preservation of
persisted policy. Initializers and historical v1/v2 producers are explicit.
Their new execution refuses without mutation; reading, cancellation/reconcile/
replay, owner close and reopening of the cancelled terminal are verified.
Binding3 and old completed recovery remain separate requirements.

Review96 covers62 files with106 open axes. It does not accept concurrent Live
changes, TelemetryStore's lasting zero timeout policy, or planning/reviewer
Root continuity. The latter and the product driver remain concrete next work
(plans100/101). These binaries are not installed; the binding transport under
development is not qualified by normal65. The199 baseline remains unchanged.

Evidence: [functional subset94](../plans/migration-rust/evidence/20261022-root-instance-201/functional-handoff94/result.json), [review96](../plans/migration-rust/evidence/20261022-root-instance-201/review96/README.md).


Update201, evidence118–125: historical v3 compatibility now passes
`legacy-v3-118` (12 guards and the actual flow), with 169 outputs in
`legacy-v3-retained119`. This complements v1/v2 proof; it establishes neither old
completed recovery, public v3 `mission.start` replay nor fresh admission. The
descriptor identity component passes six tests116 but remains Linux test-only,
without role integration. Four test lints remain recorded as FAIL121; corrections
122/123 on frozen124 pass strict engine lib/tests Clippy125, without running the
reviewer reproducer. That scenario remains unexecuted under tool block115.
`continuation-plan120.md` is adopted and being implemented to observe retained
intentions before dispatch; it is not yet qualified. These results neither advance
baseline199 nor close global work from test counts. Source profiles65/124 remain
distinct.

### Preparation budgets, 26 October2026

Native envelope CLI/API/modal installed125; historical read CLI without input and GET `/api/v1/preparations/budget` implemented in the following increment, qualification retained in [report](../plans/migration-rust/PREPARATION-BUDGET-READ-20261026.md). This supersedes the old absent-entry row only within this exact scope: no preparation AI call or global parity claim.
