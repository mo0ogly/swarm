# PAEA — increment 03: routing refusals with GLM

[Français](../../../../plans/actions-confiance/paea/INCREMENT-03.md) · [Plan](../PAEA-APEX.md)

The user authorized “next avec glm” on October 25, 2026. No compilation or restart
remains allowed. We extend the existing-binary public recipe rather than produce
Rust changes that cannot be qualified here. No Rust/Cargo, service, mission or
active settings change.

The [recipe](../../../../plans/actions-confiance/paea/evidence/20261025-glm-routing-99e363e5/public-recipe/result.json)
passes **6 named tests, 248 invocations: 179 successes and 69 expected refusals**.
Two tests extend the previous four; 160 invocations extend the historical 88.
Binary SHA256 `addcc5eb6b32dd7d2ed963a01a80062155ccd9613985fb518c079f5a461f794f`
is unchanged before/after. It is the historical test candidate, not the installed server.

Both domains and FR/EN exercise commit/approve/config-apply/mandate-create and
incompatible root/server/output/task/phase options: exit2, empty stdout,
`failure.code=command_failed`, retryable false, no output/project file created.
Explicit `--root . --phase delivery` preserves the exact canonical oracle.
Missing file gives `input-document-io`, directory gives `input-regular-file-required`,
empty stdin/file gives `protection-document-invalid`. Every fault has an exact
valid control and recovery in the same temporary root. No private canary echoes.
No OS confinement, effect-service mediation, compiler graph, served interface,
installation or E2/E3 qualification is established.

One GLM HTTP call: requested/reported model glm-5.2, requested low effort, actual
backend version unestablished. Reported usage 4218 input + 1682 output = 5900 tokens,
stop after 9.147 seconds. No automatic retry, global watchdog or inferred price.
The proposed patch was inconsistent: one method but a six-test gate, unsupported
accepted(root), undefined input override, empty-file/no-files conflict, and missing
active-operation cases. The [replan](../../../../plans/actions-confiance/paea/evidence/20261025-glm-routing-99e363e5/replan.json)
retains this before execution. Codex reused server/phase/missing-input cases,
implemented two complete tests and a shared refusal assertion, preserved the
single document.json fixture slot, and read actual input producer codes. GLM had
no repository/tools and supplies no independent E3 verdict.

The [byte-bound self-review](../../../../plans/actions-confiance/paea/evidence/20261025-glm-routing-99e363e5/source-review.json)
covers policies (existing protocol/transport representations and test corpus,
no operator setting), functions (invoke/accepted/refused/two tests/main with
contracts and limits), and errors through stderr/exit/recovery. Existing exclusive
atomic publication preserves originals; subprocess/IO errors propagate. No Rustdoc
build under the user constraint. Contract/plan/queue FR/EN are updated together;
no runtime release note because no shipped code changes or installation. Saved
documents do not update embedded contexts or running sessions.

Applied lesson: stop/nonempty GLM transport does not establish patch compatibility;
check call signatures, test count and fixture invariants before execution.
Historical evidence is preserved. The full program remains open: PA-00/01 partial,
PA-02/03 await IA3 authority producers, a qualifiable Rust candidate and explicit
lifting of the no-compilation instruction. Identity/AC-02 owners retain their
contracts; Codex owns integration. Concurrent work is preserved. Sponsor must
still appoint the independent evaluator and third-party team; PA-11…13 and final
RETEX remain open. This test increment closes no program lot.
