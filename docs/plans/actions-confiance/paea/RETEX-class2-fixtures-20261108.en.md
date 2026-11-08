# Class-2 RETEX — tempdir migrations under governed areas, 8 November 2026

Evidence-based retrospective of the Claude session of 7–8 November 2026 (the
D04 and class-2/D05 batches). Scope: migrating the 151 bare
`tempfile::tempdir()`/`TempDir::new()` allocation sites under `rust/crates`
to governed areas, extending the `tools/check_tmp_interdiction.py` guard,
FR/EN documentation. It does not establish independent acceptance; no
commit/push (the previous authorization was consumed by `123eeeb0b`).

## 1. Intended vs actual outcome

Intended ("reprends"): consign and complete the class-2 migration (D05) of
Rust fixtures to governed areas, with the guard extension and FR/EN
documentation in the same increment.

Actual: 151 sites migrated to the `tempdir_in(<area>)`/`new_in(<area>)`
contract (RAII `TempDir` kept) — repository area
`swarm_engine::fixture_area()`, fallback area
`swarm_engine::fallback_area()` (`~/.local/state/swarm-fixtures/`, 0700), or
inline replication for crates without a swarm-engine dependency. Guard
extended: 0 violations / 10 review items. Compilation 0 errors on the 7
touched crates. Verification: enrollment+linux-prerequisites 110 green,
authority 207 green, engine `--lib` 750 green / 6 reds qualified out of scope
with dated causes, full cli `--no-fail-fast` 258 green / 9 red / 20 ignored
(the 9 = pre-existing Codex-owner debts), paea +29 green with the canonical
fixture. FR/EN docs current. Work evidence retained under
`rust/.swarm-fixtures/d0{4,5}-*`. **Not achieved**: executing the 20 ignored
paea tests (dedicated recipes, delivered binaries, outside this batch's
scope); resuming the 4 consigned debts (owner Codex).

## 2. Important sequence

1. Resume after two compactions: cartography of the 151 sites per crate,
   area selection by contract (repository vs fallback), first editing pass.
2. E0308 then E0433 in the inline area helper of `authenticated_fixture.rs`
   (cross-crate `#[path]` inclusion from swarm-engine: `super::` does not
   resolve to the including crate) → self-contained helper documenting the
   case.
3. 13 `FixtureFault` plain `--tests` reds: feature-gated, pre-existing at
   HEAD, out of scope — requalified with `--features test-fixtures` (the
   project recipe convention), 0 errors.
4. 7 identity `--lib` reds: my first area choice (repository) under modules
   whose flow traverses `open_private_directory` (it refuses the checkout's
   775 ancestors) → the 4 identity modules migrated to the fallback area,
   18 green; area rule refined and recorded.
5. Two deterministic engine reds: dated to the Biba batch `2496c031b`
   (28 Oct.) by reconstructing the code at the test's origin (`174b89106`)
   and diffing the chain — the `anchored_artifact::with` interposition
   breaks the `LocalPathError` downcast. Out of scope, owner Codex Biba
   wave.
6. trust_enrollment 2 reds then identity_checkpoint 6 reds: same mechanism
   (the CLI child / `swarm_cli::run` reads anchors/declarations through the
   private walk); HEAD-before-blame via the 7 Nov. BF-16b cartography
   (trust_enrollment was green there) → 7 integration suites migrated,
   39 green.
7. The 7 Nov. "full" cli run without `--no-fail-fast`: stopped at the first
   failure, cli.rs/paea_settlement/planning never exercised; the full 8 Nov.
   run → 2 cli.rs reds (same area cause, fixed, 17 green) and 1
   paea_settlement red (same cause, fixed); preventive migration of the 4
   declaration-bearing paea suites; final run 258/9/20.
8. Guard extended to "any bare Rust tempfile allocation" (including the
   `Builder…tempdir()` window); 947 `.tmp*` residues under /tmp all
   predate the batch (latest 08:25 on the 8th), no leak since.

## 3. Available measurements

- Sites: 151 migrated (engine 14, web 6, cli 86, enrollment+linux-prereq
  51); 23 fallback sites under swarm-cli (1 version_no_repository, 4
  identity `--lib` modules, 7 integration suites, cli.rs, 5 paea suites).
- Tests: 110 + 207 + 750 + 258 green; 15 reds qualified (7 identity `--lib`
  + 8 integration + 3 cli/paea introduced then withdrawn by this batch; 9
  residual reds = Codex debts: 8 agent_launch BT02 Biba
  `biba-workspace-overlaps-root` — signature re-read in the retained
  `.tmpxUJ8Sl/active.stdout` evidence — and 1 planning
  `agent-provider-catalogue-read` "never green").
- Guard: 0 violations / 10 review items (documented/pre-existing), 3 runs.
- Cost/time: not measured (no counter) — unknown, not zero.

## 4. What worked

- HEAD-before-blame discipline: every red was attached to a dated state
  (BF-16b cartography, `git show` at the creating commit) before any
  inference; the 18 reds introduced by this batch were distinguished from
  the 9 residual debts without ambiguity.
- The objective area rule derived from the mechanism: "any flow that reads
  an anchor or a declaration through `open_private_directory` requires the
  fallback area, even inside swarm-cli" — applied preventively to the
  remaining paea suites instead of discovering reds one suite at a time.
- The /tmp interdiction held end to end: no project artifact under /tmp
  during conformant runs; governed TMPDIR (`/home/fpizzi/swarm-run-tmp`,
  0700 outside the checkout) for the suites that require one.

## 5. Most costly failure mechanisms (observed)

1. **First area choice not discriminating (18 reds in 3 waves)**: the
   initial distribution opposed "enrollment crates" to "swarm-cli", while
   the real criterion is the flow (private walk), not the crate. Cause: the
   initial cartography classified sites by crate without tracing child read
   flows. The final rule is objective but was reached through three waves
   of reds; tracing the flows first (`open_private_directory` /
   `authenticated_startup::open` / `load_authenticated`) would have produced
   the correct distribution immediately.
2. **"Full" run without `--no-fail-fast` (false coverage)**: the 7 Nov. run
   stopped at agent_launch and left cli.rs, paea_settlement and planning
   never exercised; the cartography's "80 green / 8 red" tally was
   incomplete. Detected on the 8th by the full `--no-fail-fast` run.
   Lesson: a suite tally must be `--no-fail-fast` or carry the explicit
   note "stopped at first failure, remaining suites not exercised".
3. **Ghost evidence reference**: BLP-APEX cited a `d05-guard-red.txt` that
   was never created; corrected to the real files (`d05-guard-final.txt`,
   `d05-cli-final-noff.txt`). Control: cite only files verified by `ls` at
   writing time.

## 6. Prioritized actions

| Action | Impact | Confidence | Effort | Owner | Acceptance criterion |
|---|---|---|---|---|---|
| ~~Trace `open_private_directory`/`authenticated_startup::open` flows in the permanent-recipe cartography~~ **Done 8 Nov.**: the flow-based decision rule was added to the verification README (FR line ~168, EN ~218) — the criterion is the flow, not the crate; applicable to a new suite without a trial run | Medium | High | Low | Done | ✅ README EN/FR |
| Resume the 9 residual cli reds (8 agent_launch BT02 + 1 planning catalogue-read) | High | High | Medium | Codex | Criterion in BIBA-TRANSVERSE-APEX.md; the planning criterion is to be defined by the Codex lot (no verifiable reference file as of 8 Nov) |
| Engine debt: `LocalPathError` downcast through `AnchoredArtifactError` (Biba batch `2496c031b`) + v52 drift | High | High | Medium | Codex | The 2 mission_status reds + 2 settings reds green at HEAD |
| ~~Fold `/home/fpizzi/swarm-run-tmp` into the permanent recipes as the governed TMPDIR~~ **Already done, better**: the `recipe.py` runner pins TMPDIR under the configured area (Admin setting `runtime_temporary_directory`, operator private fallback — no hard-coded path); the criterion is met and proven by the class-2 recipe (PASS 4/4, VERIFIED, `environment.json`: `TMPDIR=~/.local/state/swarm-fixtures/runner-tmp`) | Medium | High | Low | Done | ✅ Evidence `.swarm-fixtures/class2-recipe-20261108T030000Z` |
| Commit/push of this batch (local modifications preserved) | — | — | Low | User | Fresh authorization, then commit without `Co-Authored-By` |

## 7. Limits of this RETEX

- Cost, time and interval not measured (unknown, not zero).
- This batch's review is not independent; no independent acceptance was
  obtained.
- The 20 ignored paea tests and their dedicated recipes are not exercised by
  this batch; their declaration-bearing flow was migrated preventively
  (objective criterion) but execution proof belongs to the recipes.
- Earlier batches (v52, Biba) receive their qualification, not their fix;
  owners recorded in BLP-APEX FR/EN.
- No commit/push; everything remains preserved local modification.
