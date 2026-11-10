# PAEA — increment 24: concurrent laboratory revocation and commitment

[Français](../../../../plans/actions-confiance/paea/INCREMENT-24.md) · [Plan](../PAEA-APEX.md)

## Resume and decision

Increment 23 committed as `97933d8b7790fa5ee67956ac96351fd24fca2a72`, 27 files;
108 foreign staged paths preserved. Resume keeps PA-00…13, Codex alone without
GLM, existing authority and budgets. Fresh evidence: `evidence/20261026-contention24-efknxkuf/`
in the French plan tree. Initial collector omitted the launcher's module search
path; adding the actual `tools` path allowed read-only identity inspection, without
service mutation (`replan-resume-import.json`).

Observed PA-03/I05/I17 gap: T05/T17 only exercised sequential orderings. Options:
introduce SQL waiting/retries, or retain immediate contention and explicit recovery.
Choose existing SQLite `timeout=0`, two real overlapping processes and both owners.
A test helper pauses after an actual authority read inside the real transaction;
SQL/Git effects and the separate oracle are unchanged. The native CLI is the
previously qualified normal dev109 pinned by hash/profile from increment 23;
current concurrent sources are not claimed qualified.

## Criteria and limits

1. Matching unopposed worker/gate control before every contention.
2. Commit owns lock: concurrent revoke fails with lock cause without mutation;
exact effect then explicit revoke, with retained durable ordering.
3. Revoke owns lock: concurrent commit is UNKNOWN without effect; after release,
old unused grant denied, new grant permits exactly one effect.
4. Both domains, raw oracles/reconciliation and retained output/PID/IPC.
5. Explicit early-transaction-release substitution must fail the same assertion
after valid controls, never establish the intended property.
6. Existing eight cases/report v1, output guard and recipe integrity verified;
byte-bound policy/function/error review.

Only checkout tests and recipes change: no compiled input or served asset. The
menu's explicit command consumes these checkout files. Rebuilding/restarting would
deliver none of the changed bytes, so no new server installation or embedded
release note is attributed to this qualification. Same UID, simulated authority,
instrumented scheduling and cooperative writers remain limitations. No protected
authority/confinement/distributed race. Protected PA-02/03, IA3/AC-02, humans, E2/E3,
0/17 invariants and final programme RETEX remain open.

## Verification and follow-up

Pending actual execution, owned by Codex for the recipe. Contention never proves
revocation applied: explicit successful recovery plus epoch/sink readback establish
the local order. Historical increment 23 evidence remains unchanged and does not
cover this helper.


## Results, review and slice RETEX

`final-recipe-03`: **three steps PASS, integrity VERIFIED**, no global watchdog.
The named T05/T17 case keeps historical orderings and passes **four contentions**,
each preceded by a matching worker/gate control. The guard requires **four expected
assertion failures**, one per domain/owner. All four controls/peers succeed and the
four suspended owners are aborted/reaped with the original cause. The negative
campaign is never exported as PASS. The public command then runs **eight named
tests** in `campaign-02`, verifies/exports report v1 and refuses wrong hash,
existing output and corrupted proof without overwriting the report. Inner recipe
also PASS/VERIFIED. Scenario counts are not independent human evaluation.

Initial targeted PASS retained. `final-recipe-01` failed because the guard expected
PilotLab's stdout summary on stderr. After fixing that consumer, all commands in
`final-recipe-02` passed but its derived manifest asserted LabCommand's stderr
summary on stdout. Original failures and outputs retained. The final manifest
follows all three real producer streams and is preflighted against originals.
Both reader/component inventories now include the transitive helper; their
inventory/schema checks pass, but browsers are not rerun/requalified in this
slice. No served asset/rendering changed.

`effective-production-inputs.json` matches **ten protection sources** against the
normal dev109 effective inventory/archive: identical bytes, empty core features,
test=false, no fixture feature. Exercised binary SHA256:
`fb1dcac07bd0794a0f0d45c8b1f379c685ac2a1953e830f4bf784e647e1adc6e`.
No current global engine qualification is inferred. Separate protected runtime
observation found dev112/PID1327375, embedded commit97933d8b7, modified=true,
while concurrent delivery continued (`runtime-observation.json`). No restart or
installation attributed to24, source configuration preserved, no foreign signal.

Same-context `source-review.json` binds **14 actual files** to policy/function/error
review. IPC gating/early release are explicit test instruments; no product delay,
retry or concurrency cap. Existing fixture timeout=0 remains unchanged. EOF/abort
at the held phase closes the connection, rolls back, reaps the child and retains
its cause/uncertainty. Assertions check the actual lock effect and raw sink, not
only the in_transaction flag. No independent/global approval claimed.

Lesson: inspect every derived recipe command's actual output streams and carry
new transitive dependencies into all caller inventories. The first stream fix
was too narrow; the final preflight covers all three producers and inventories.
Persistent documents and verification README transmit this lesson; embedded
agent contexts/already running processes are not assumed updated.

The programme remains open: same-UID cooperative locks are not a protected
boundary. IA3/PA-02 service/bootstrap/human principal/confinement, protected PA-03,
browser campaign execution and PA-10…13/E2/E3/final RETEX retain their plan owners
and prerequisites. No commit24/push or manufactured acceptance.
