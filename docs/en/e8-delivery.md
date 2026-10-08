# E8 — delivery evidence and direct takeover

## Status, 26 September 2026

**Technical delivery prepared; the main mission remains open at 5/8.** The user
requested direct Codex work after repeated review interruptions. The polling
automation was removed. This is an external intervention, not demonstrated autonomy.

The public `delivery` test runner had no matching test. The new
`TestEngineContractDeliveryEvidenceAndPublication` runs six existing behavioral
guards (twenty tests/subtests): incomplete delivery, mandatory independent review,
failed controls, stale acceptance after candidate changes, child scope closure,
and exported branch contents. Fixtures are isolated; no paid provider is invoked.

See [the verification manifest](../e8-verification-20260926.json) for commands,
exit codes, durations and the tested source change. The eight public scenarios are
`launch`, `ownership`, `revision`, `recovery`, `truth`, `real`, `history`, `delivery`.
Run each with `node tests/engine_acceptance.cjs --case CASE`.

## Evidence and limits

C1/C2 are covered by ownership scenarios; C3 by recovery/real; V1/V2 by
launch/delivery; V3/V4 by revision/history/delivery; V5 by launch/revision/delivery;
R1 by recovery; U1 by truth and frontend tests. P1 has historical real-provider
evidence from 23 September on engine `1a93811`, not a new run on the current engine.
All six original artifact hashes were checked again and match the manifest.

Eight delivery records are distinguished: revision, checks, report/limitations,
reviewer verdict, decision, freshness, deliverable, and external interventions.
The current independent review ended without a final result. Codex's direct work
is not an independent approval of its own changes. No acceptance is fabricated.

## Lessons learned

Engine improvements were repeatedly confused with completing the mission. A test
checklist claimed an assertion that did not exist. A one-line correction invalidated
all inspections tied to the previous candidate SHA. Large cumulative reviews and
insufficient event diagnostics increased recovery cost. Repeated polling observed
the blocked state without repairing it.

Next improvements should include proven reuse of unchanged inspected artifacts,
explicit cross-artifact checks, complete budget estimates before launch, bounded
recovery with a changed precondition, and a clear separation between execution,
checks and acceptance. These are proposals, not capabilities claimed by this report.

## Cost and closure

At takeover the mission had consumed 38 of 49 authorized review calls. Monetary
cost is unknown. The separate historical trial used five planning activations,
two productions and one review. External corrections, installations and public
retries are documented; an exhaustive intervention total has not been reconstructed.

The local recovery bundle is not an accepted mission candidate. Completion requires
fresh independent review and checks on the same candidate plus closure of all three
scopes. Passing tests and exporting a bundle do not establish 8/8 completion.
