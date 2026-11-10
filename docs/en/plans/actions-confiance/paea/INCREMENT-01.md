# PAEA — increment 01, review and resumption

[Français](../../../../plans/actions-confiance/paea/INCREMENT-01.md) ·
[Plan](../PAEA-APEX.md) · [Contract](../../../PAEA-CONTRACT.md)

On October 25, 2026, Codex alone implemented the first PA-00/PA-01 foundation.
This is an integration self-review, without E2/E3 independence. The program and
its final retrospective remain open. The latest user instruction prohibits new
compilation while their other session is active; the executable checks below had
already finished. No other session's process is stopped.

## Baseline and bounded result

The [retained baseline](../../../../plans/actions-confiance/paea/evidence/20261025-contract-qekt2uwo/baseline.json)
identifies checkout, branch, commit, selected sources and the observed
`v0.1.1-dev.65` instance. Its archive selects analysis inputs after initial edits;
it is not an atomic pre-edit repository snapshot. Unrelated changes and historical
Go evidence remain preserved. Original PAEA DOCX/PDF files are unchanged.

The new contract has no engine dependency. It checks representations and computes
an exact-content/declared-context digest. Business intent stays stable across new
transport keys. The pure lifecycle prevents UNKNOWN from becoming denial without
effect or another commitment attempt. It reserves neither intents nor effects and
authenticates none of its inputs.

Public `protection canonicalize` reuses this contract without opening a project.
Both domains and FR/EN were exercised in temporary roots. API, UI, authority
storage, executable mandates and effect services are absent from this slice.
`ApprovalBinding` is not a complete approval; `EffectReceipt` is not an authenticated
attestation.

## Retained checks

| Check | Result and scope |
|---|---|
| [Permanent recipe](../../../../../tools/verification/recipes/paea-contract.json), [result](../../../../plans/actions-confiance/paea/evidence/20261025-contract-qekt2uwo/recipe-01/result.json) | PASS: 5 Python tests, 17 Rust contract tests, 4 CLI tests. Fourteen vectors are inputs, not fourteen additional tests. No global watchdog. |
| Recipe integrity | `recipe.py verify` succeeded on original evidence. Declared inputs, argv and raw output are retained, without claiming a complete transitive compiler inventory. |
| [Clippy](../../../../plans/actions-confiance/paea/evidence/20261025-contract-qekt2uwo/clippy-01.stderr) | `-p swarm-protection-contract --all-targets -- -D warnings` succeeded; this library and its tests only. |
| [Rustdoc](../../../../plans/actions-confiance/paea/evidence/20261025-contract-qekt2uwo/rustdoc-01.stderr) | `-p swarm-protection-contract --no-deps`, `RUSTDOCFLAGS=-D warnings` succeeded before the no-compilation instruction. |
| Final static review | [Byte-bound inventory](../../../../plans/actions-confiance/paea/evidence/20261025-contract-qekt2uwo/increment-review.json), local links and diff; no recompilation or Cargo recipe rerun. |

SCALING-S2 engine sources are changing in the other session. The four CLI tests
validate the offline path, not a complete native candidate. Engine warnings from
concurrent work remain in original output. No PAEA installation or activation is
claimed. Documentation corrected after the recipe is inventoried separately;
original archives remain untouched.

## Hardcoded-policy review

| Value or rule | Category, origin and consumer |
|---|---|
| Schema1, `paea-json-v1`, intent prefix, operation names and transitions | New protocol invariants shared by library/schema/docs. Changes require versioning, establish no operator policy or permission. |
| 1–100 ASCII identifiers, 64-character SHA256, integrity I0–I3 | Reused Identity/existing-label representations. A label's name grants no rights. |
| Positive decimal i64 amount, positive i64 revisions | Technical consumer representation bounds. Strings preserve browser precision; no monetary limit/budget configured here. |
| Three-letter currency, nonzero 40/64-character OID, direct branch | Fixture representations. Existence, actual support, physical repository binding and symbolic-ref refusal remain sink duties. |
| JSON depth and bounded CLI reader | Existing technical safeguards, not increased. No new operator size, deadline, budget, retry or ceiling. |
| CLI root `.`, phase `delivery`, offline command only | Existing parser defaults, without opening Root. Explicit `--root .` is equivalent to the default; different effective values refuse. |
| Fixture amounts, identifiers and digests | Explicit test inputs, never installed settings or attributed identities. |

No new persisted operator choice is introduced. The storage → validation →
Admin/API/CLI → actual consumer chain remains mandatory for PA-02/04/05 policies;
this offline computation does not claim that chain.

## Function and error review

The exact inventory is in `increment-review.json`. Reviewed functions include five
decoders, four structure validators, `canonicalize`, its getters and
`matches_approval`, `State::transition`, `oid`/`branch`, strict visitor/decoder,
recursive canonical writer, offline CLI handler and three Python-oracle functions.
Comments explain purity, unauthenticated inputs, refusals and future consumer duties.
`CliError`, dispatch and help connections preserve existing contracts; the dedicated
module documents the offline path.

- JSON/duplicate/unknown/invalid input retains a typed cause and static public
  diagnostic. The CLI test checks that private content is not echoed, then recovers
  with valid input in the same root.
- Invalid transitions return `Err` without mutating input or granting retry/success.
  Reconciliation observations remain distinct.
- Reading reuses `CommandInput`, without a datastore or credential. Encoding errors
  use `CliError::Protection`; internal UTF-8 errors retain their IO cause. None can
  have committed a project effect in this offline path.
- Write/flush errors propagate as `CliError::Output` with nonzero exit. `/dev/full`
  exercises public output failure without creating `.swarm`. Partial output remains
  possible and is not an effect observation.
- Vector publication reuses the existing atomic exclusive publisher, preserving
  previous files/evidence.

Tests do not establish all IO failures or distributed service closure. Future
storage, transport and effect consumers must preserve the same contracts; `Err`
will no longer establish absence of an effect.

## Current state of all 17 invariants

“Local data test” means only the contribution below. No global invariant is
declared satisfied. Reuse candidates are not attested protected producers.

| Invariant | This increment / reuse candidate | Protected storage and consumer still to verify | State / owner |
|---|---|---|---|
| I01 | Strict decoder rejects invented authority fields; CLI output explicitly grants nothing. | Attributed admission and actual consumer under injection. | Local data test; Identity/integration. |
| I02 | Principal/project/instance/mandate shape; Identity contract reuse. | IA3 authority, delegation, audience, opened-project binding. | No execution proof; Identity. |
| I03 | Validated source/version/labels; actual source order preserved. | `memory_files.rs`, reads/summaries/transfers and real recipients. | Local data test; Memory/AC-02. |
| I04 | Exact binding changes with content/policy. | Actual ledger/Git consumption and authorized approver. | Local data test; Trust/repository/UX. |
| I05 | Context bound into digest, no freshness check. | Current mandates/policy/revocation at commitment. | Specified; Identity/sinks. |
| I06 | No confinement mechanism added. | `agent_launch.rs`, descendants, files, IPC/network; closed inventory. | Not demonstrated; AC-02/launch. |
| I07 | CLI reads no credential/secret. | Keys outside agent domain, separate accounts, bounded secret use. | OS boundary not examined; Identity/AC-02. |
| I08 | Derived intent stable across new keys/alternative content. | Durable ledger/Git reservation, conflicts and accumulation. | Local data test; sinks. |
| I09 | UNKNOWN only accepts reconciliation; terminal states cannot mutate. | Durable intent, crash/lost response and independent observation. | Local data test; sinks/verification. |
| I10 | Approver shape only. | Authentication, entitlement, separation of duties and vigilance study. | Not demonstrated; UX/sponsor/evaluator. |
| I11 | Receipt shape distinguishes observations/authorization. | Reuse `trust/proof.rs`, `root_store/trust_store.rs` with separate observer/trust anchors. | Authenticity/independence not demonstrated; verification. |
| I12 | Invalid input and unavailable active commands refuse without opening Root. | Actual launch/sink behavior when required controls are unavailable. | Limited local test; AC-02/sinks. |
| I13 | Labels represented, no flow enforcement. | Broker and actually received output, including models/transports. | Execution not examined; AC-02/Memory. |
| I14 | No A2A channel added. | Peer authentication, audience/mandate/provenance at recipient. | Not demonstrated; Identity/Memory. |
| I15 | Manifest digest bound to action, no binary measurement. | Actual catalogue/tool and readmission after substitution. | Shape implemented; catalogue/launch. |
| I16 | Positive revisions, no agent-supplied clock consumed. | Epochs, monotonic leases, revocation/commit order and restart. | Not demonstrated; Identity/sinks. |
| I17 | Single-use-token erratum retained. | First redemption of pre-issued token fails after effective revocation. | Specified, missing proof; sink/evaluator. |

Existing Trust storage and Git references are neither extracted nor migrated here.
Historical Go and earlier Rust tests retain their own scope; no verdict is inherited
automatically by PAEA.

## Findings, resources and resumption

| Origin / value | Decision and actual state | Owner and observable resumption condition |
|---|---|---|
| PAEA §6.2: token uniqueness does not establish revocation before first use. | PAEA-ERR-01 specified, original preserved. | Sink; valid control then revocation and refused first redemption without effect. |
| Session recovery: concurrent work changes effective engine inputs in the same checkout. | Preserve work; do not deliver an unreviewed cumulative candidate. | Local integration; available checkout and complete candidate review/qualification. |
| Explicit user instruction: avoid compiling. | No new Cargo build/test or restart; subsequent documentation/static review only. | User/integration; instruction lifted before compilation. |
| User has no independent evaluator or third-party team. | PA-11 protocol remains draft; no PA-12 campaign started. | Sponsor; resources and independent-role acceptance before preregistration. |
| Digests/schemas may be mistaken for authority. | Explicit `not-established` output and contract limits verified on offline CLI. | Future producers; no active API may accept supplied context as authority. |
| CLI option review: explicit `--root .` is accepted as default. | FR/EN text corrected to describe actual effective-value refusal. | Codex; predicate and documentation statically compared. |

Separate accounts, necessary privileges, authenticated channel and actual Linux
restrictions remain to qualify with IA3/AC-02. Installed tools or a recent kernel
are no proof. Scripted providers suffice for the foundation; no real campaign
account/model/budget is invented. Lessons reach later readers through bilingual
plans and the APEX queue, without changing embedded contexts or running processes.

Next implementation: finalize PA-01 with IA3 producers, then both PA-03 services
after PA-02, subject to retained migration/confinement dependencies. The program
remains open.
