# PAEA — increment 09: loop resumption and verification without compilation

[Français](../../../../plans/actions-confiance/paea/INCREMENT-09.md) · [Plan](../PAEA-APEX.md)

The user requested “next en boucle”. Root Codex continues without a new prompt,
compilation or restart, applying APEX/replan and verify-fix. Another session built
candidate `v0.1.1-dev.75`; an owned frozen copy is used for offline CLI observations,
never as a server. Observed effective inputs match protection/comparison/receipt
sources. This neither installs the binary here nor establishes protected effects.

Rather than accumulate unexecuted extensions, first verify increments 05…08 on
that existing artifact. No provider/agent call. Replan found two increment-08
Rust assertions incorrectly reading root `code`; the actual producer uses
`failure.code`. A real missing-file receipt invocation returned exit2, empty
stdout and `failure.code=input-document-io`, with no root code. Original:
[failure-envelope.stderr](../../../../plans/actions-confiance/paea/evidence/20261025-loop09-ys8pyhtn/failure-envelope.stderr).
Both assertions are corrected; no executed Rust test failure is invented and
their execution stays deferred. Extend the existing public test harness rather
than add a Python product implementation.

Fourteen named Python tests are prepared: six previous canonical/routing cases,
four comparison and four receipt cases through the frozen Rust binary. Complete
reports, unchanged input bytes/inventory, refusal/recovery, FR/EN and both domains,
UNKNOWN/terminals and `/dev/full` write failure. Controls/recovery bracket faults;
no real Root, service, global watchdog or automatic retry. The Rust injected-writer
flush test stays unexecuted and separate.

Sources, originals, binary, observed version/build inventory and recipe outcome:
[evidence directory](../../../../plans/actions-confiance/paea/evidence/20261025-loop09-ys8pyhtn/).
Executed state is retained in `recipe-01/result.json` after launch; absence is not
PASS. Final status below will follow integrity verification. E3 independent means,
protected authority and authenticated observation remain open.

## Verified outcome and second loop

**14 tests passed, 618 invocations**: 451 exit0 and 167 expected refusals. The
14 include six previous tests plus eight new tests. Operations: 248 historical
canonical/routing calls, 163 comparisons, 207 receipt checks. Three public ENOSPC
refusals included. `recipe.py verify` confirms original PASS/integrity. Binary
unchanged, no project mutation. These are not Cargo Rust test executions and do
not inject the Rust writer's flush failure.

Provenance loop: verify every hash in the archive of **432 effective build inputs**;
eight PAEA inputs match current source. Normal core profile has no fixture feature
and is not a test profile. The first check incorrectly assumed empty features for
every dependency: enrollment authority/config enable Identity `serde`. Original
refusal retained, producers inspected, exact features then checked (`protection=[]`,
`identity=[serde]`). No artifact/protection changed to make this pass. See
[provenance-verification.json](../../../../plans/actions-confiance/paea/evidence/20261025-loop09-ys8pyhtn/provenance-verification.json).

Earlier-lot review: the Python verifier actually emits a root `code` in its own
envelope, so its assertions stay unchanged. Only increment-08 CLI assertions
were incorrect. Lesson: inspect the consumed serializer, beyond an error type's
`code()` method. FR/EN contract and Rust helper now convey `failure.code`; historical
results are untouched.

Two schema descriptions wrongly embedded temporal “uncompiled” state in a stable
contract. Only descriptions changed; all shape constraints are identical, and
these files are not effective inputs of the observed build. Original recipe stays
bound to original bytes; observations on unchanged binary/tests/Rust sources are
reused, while metadata is reviewed separately, with no claimed rerun. This
tests/docs-only increment changes no delivered behavior or release number/notes.

Two further retained calls verify that receipts with differing supplied observation
digests can yield identical correspondence reports. Reports claim no authenticity.
Follow-up `PAEA-OBS-01`, PA-09/Codex: bind standalone output to exact input bytes or
require the external evidence bundle. Applied mitigation here: retain complete
input/binary/output transcript. Output evolution waits for its compatibility
contract and an authorized build, granting no rights. Original observation:
[standalone report binding](../../../../plans/actions-confiance/paea/evidence/20261025-loop09-ys8pyhtn/standalone-report-binding-observation.json).

## Reassessing all 17 invariants

**0/17 demonstrated end to end**; six have local structural observations, eleven
lack proof of their real protection. The 14 local tests replace no distributed
T01…T22 campaign.

| Invariant | Local evidence | Owner and resumption condition |
|---|---|---|
| I01 Model non-authority | PARTIAL: forged authority fields refuse, output grants nothing. | PA-02/06: actual model → protected control → admitted/denied use. |
| I02 Identity/delegation | NOT TESTED: fixture identifiers are unauthenticated. | PA-02/06: protected IA3 output and current delegation. |
| I03 Provenance/flow | NOT TESTED: no protected read/revoke/compaction. | PA-05/06 and Memory: actual versions and uses. |
| I04 Exact approval | PARTIAL: exact content/digest/data binding. | PA-02/03/08: current approver and content consumed at the sink. |
| I05 Last use | NOT TESTED: no sink commitment. | PA-02/03: ordered authority check at commitment. |
| I06 Mediation | NOT TESTED: offline tool has no confinement. | PA-04/05, AC-02 prerequisites and bypass inventory. |
| I07 Secrets | NOT TESTED: no critical secret or separate account. | PA-02/05: control/keys outside agent domain. |
| I08 Business uniqueness | PARTIAL: stable intent under new transport keys/content. | PA-03: durable reservation and no duplicated effect. |
| I09 UNKNOWN/recovery | PARTIAL: declarative lifecycle and replay/relabel refusal. | PA-03/09: lost reply, durable observation and reconciliation. |
| I10 Human supervision | NOT TESTED: no approver/human experiment. | PA-08/12: approval flow and authorized human protocol. |
| I11 Independent evidence | PARTIAL: binary/originals/hashes retained, no independent observer. | PA-09/12: external anchors and independent evaluation. |
| I12 Safe failure | PARTIAL: input/routing/output refusals without fixture mutation. | PA-03/04/05: unavailable prerequisite during an actual effect. |
| I13 Egress/confidentiality | NOT TESTED: no controlled network/destination. | PA-05/06: network oracle and effective policy. |
| I14 A2A | NOT TESTED: no authenticated peer/mandate/audience. | PA-06: verified rights handoff and use. |
| I15 Tooling | NOT TESTED: build provenance is not protected tool admission. | PA-07: actual substitution and invalidation. |
| I16 Time/order | NOT TESTED: no protected clock/epoch/lease. | PA-02/03/07: actual commitment/restart ordering. |
| I17 Sink revocation | NOT TESTED: retained erratum, no token consumption. | PA-02/03/12: effective revocation then first consumption denied. |

Next: root Codex runs corrected Rust targets/Rustdoc after the instruction is
lifted. PA-02/Identity integration supplies protected identity/authority before
PA-03 effects. AC-02 keeps prerequisites, without implicit activation. Sponsor
provides E3 third parties. No local restart/install here; PA-00…13 and final RETEX
remain open. Byte-bound review and final checks:
[source-review.json](../../../../plans/actions-confiance/paea/evidence/20261025-loop09-ys8pyhtn/source-review.json).
