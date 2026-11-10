# PAEA — increment 14: laboratory effects and oracle

[Français](../../../../plans/actions-confiance/paea/INCREMENT-14.md) ·
[Plan](../PAEA-APEX.md) · [Operational dossier](OPERATIONAL-PILOT.md)

## Resume and decision

“go next” resumes increment13's effect/authority/oracle and three-fault priority
in **both** domains. APEX/replan applied, root Codex alone, direct checkout,
without compilation, restart, installation, provider, Root DB, external effect,
delegation, commit or push. Six shared documents matched retained preimages
before edits. Exercised executable: increment12's frozen dev77, SHA256
`7e7563dd5e1bdb848fec42f6ae2e619c31b048889221191517553fe45e56f6d3`.
Its retained profile evidence stays historical; no new build is inferred.

Options compared: write unqualifiable native services ahead of PA-02 authority
outputs, or executable isolated effects and observations. Selected the latter.
**This laboratory does not implement protected PA-02/03.** Effect cards cover
consequence, exact input, intent, required sources, commitment point, oracle,
risks and access graph. Business/risk owner, human approver and independent
evaluator remain unassigned; no invented C3 decision or mandate.

Method diagnostics retained: excessive full git-status output changed to bounded
owned-path reads; assumed `swarm-trust` crate absent, actual ownership found in
`swarm-engine/src/trust`. No dependent edit/execution on the assumed path.

## Realization and observations

`tools/paea/pilot_lab.py` supplies two fixture sinks: unique-intent SQLite entry,
and Git branch CAS plus atomic intent marker. Simulated grant/current epoch are
checked under SQLite's shared lock. This provides deterministic orders, not a
distributed race or immediate-revocation guarantee. The existing public CLI
produces canonical content; no Python replacement canonicalizer.

`pilot_oracle.py` reads actual effect data in a separate process: read-only SQL,
or real Git refs/blobs. It reads no receipt/approval/decision journal. **Separate
observation path, not independent evaluation**: common author, common UID, files
still accessible to participants.

| Executed check | Result and limit |
|---|---|
| T04, both domains | Fresh control, substitution refused without mutation, equivalent reordered JSON subsequently accepted. |
| T06/T21, both | Actual exit86 after effect/before reply; client UNKNOWN, one effect observed then reconciled. Git SQL journal still empty after crash. New attempt key cannot duplicate; changed context refuses. |
| T05/T17, both | Unused preissued grant refuses after persisted revoke, SQL sequence/epochs checked. Reverse order preserves earlier effect; current grant restores valid operation. |
| Unavailable oracle, both | Error/UNKNOWN, no empty DB created; restoration finds the original effect. |
| Stale/symbolic Git ref | Refusal, unchanged snapshot, then valid recovery. |
| `/dev/full`, both | UNKNOWN diagnostic, effect retained/reconciled. CPython exits120 during final buffer flush following the write error; never success or no-effect evidence. |
| PAEA-GUIDE-03 | Two raw IDs hypothetically aliasing one debt cause **two entries** without a registry. Counterexample retained; global debt uniqueness unproven. |

Final recipe `paea-pilot-lab.json`: **7 named tests PASS**, observed **17.125 s**, no
watchdog. **132 subprocesses: 31 public canonicalizations, 39 fixture services,
62 oracles**; 26 isolated sink roots retained. Exits: 116 successes, ten expected
refusals2, two injected crashes86, two output failures120, two unavailable
observations1. Counts describe the final run, not accumulated replays. These
are neither independent E3 trials nor power-loss atomicity
proof. Recipe **PASS**, integrity **VERIFIED**. Native HTTP tests remain unrun.

## Negative witness and improvements

**PAEA-LAB-01, exact post-crash binding.** Initial-design review found that a
candidate-only marker would lose canonical context after exit before SQL
journaling. Selected realization: marker blob contains the entire canonical
content, atomically bound to branch publication. An isolated four-file copy
changes only the replay guard to compare the candidate while ignoring context.
The valid control succeeds; changed-context attempt wrongly exits0 instead of2,
and the Git subcase fails exactly with **`AssertionError: 0 != 2`**. Original
mutant sources/diff/output retained. Selected sources stayed unchanged during
this test. Subsequent review added explicit `laboratory-intent-conflict` cause
to UNKNOWN reconciliation, asserted exactly in both domains. `recipe-01` retains
the first PASS (21.126 s); `recipe-02` verifies the new bytes (17.125 s). The
negative witness still covers the unchanged replay guard, not the later
diagnostic change.

**PAEA-GUIDE-03, business registry prerequisite.** A real laboratory counterexample
now supplements increment13's documented risk. Owner: PA-02/03 business
integration. Resume with a protected project-unique debt registry and
supplier/reference collision, alias and correction tests; otherwise explicit
schema version/migration. The fixture deliberately accepts raw IDs to preserve
this limitation and must not become a protected service before resolution.
No implicit v1 hash change or per-invoice/mission exception.

## Review, retention and transmission

Evidence: `evidence/20261025-pilot14-zswhjf5p/` in the French plan directory.
`plan.json`, `summary-02.json`, `recipe-02/`, `observations-02/`, retained first run,
`mutation-source/`,
`mutation-result.json`, hash-bound review/static checks/owned diff retain
authority, exact bytes, functions, policy and error review. Declared-input
retention cannot replace effective compiler graphs or independent review.

- Policies: amounts/identities/OIDs/epoch1/exit86 are fixture/fault protocol;
  intent grammar/CAS are test invariants. `timeout=0` surfaces fixture SQLite
  contention immediately. No product execution setting/retry/budget/hidden
  ceiling or actual Admin path added. Git excludes inherited config/hooks/routing.
- Documentation: **27 functions/methods** in three Python sources have reviewed
  contracts. FR/EN dossier, plan, queue and guide transmit usage/limits. Persistent
  files do not alter embedded instructions or already running contexts.
- Errors: grant/epoch/intent/CAS/symref refusal occurs before a new effect;
  IO/SQL/Git/output errors may retain UNKNOWN. The observer never converts failed
  reads into empty success. Injected failures/negative witness remain visible
  with recovery tied to changed prerequisites.

A plan edit refused a truncated anchor before mutation; the complete actual
line was read and the patch corrected. `execution-inputs.json` retains this
diagnostic plus Python/SQLite/Git, executable paths/hashes and environment.

No new product release note: laboratory tools/docs only, no UI/service build
input changed and no delivery. Browser checks are not repeated for non-UI files;
increment13's component evidence stays separate.

## Return to the need

Operational preparation verified **only in fixtures**. **0/17 global invariants
demonstrated end to end**. The PAEA menu still checks file correspondence; it
does not launch these campaigns. Next authorized work: use tests/counterexamples
to specify the protected debt producer and PA-02 authority binding, then PA-03
services; integrate campaigns into UX after an actual public contract. Preserve
IA3/Rust/AC-02 prerequisites. Build/native qualification/installation require
explicit lifting of the ban; E2/E3, authorized roles and final RETEX remain open.
