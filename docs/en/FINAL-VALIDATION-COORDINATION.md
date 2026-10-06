# Final validation coordination

This document describes the consolidated candidate after T2/T3, the expected
coverage of its final validation, and the limits of the current organization.
The associated machine-readable manifest is
[`final-validation-coverage.json`](../final-validation-coverage.json). It checks
that the same requirements, components, controls, and evidence remain tied to
one candidate; its presence is not an engine receipt, independent review, or
acceptance.

## What exists today

| Item | Delivered behavior | Explicit limit |
| --- | --- | --- |
| Common candidate | The manifest fixes the base commit and hashes of the documentation, engine, web, and test inputs covering `req-5` and `req-6`. Every group refers to the same `candidate_id`. | The checkout is dirty: the reliable identity is the commit **plus** content hashes, not the commit alone. Changing an input requires a new manifest and invalidates dependent checks or opinions. |
| T2 recheck | CLI and web use the same preview/apply contract. A correction preserves the attempt and adds no producer; an unchanged precondition is rejected. | T2/T3 evidence remains evidence from their reports and isolated recipes; it does not replace a final check of the consolidated candidate. |
| T3 evidence | CLI and web share the projection for current verdict, history, and measurements. Only available measurements are shown; cost and tokens remain `unknown`. | A control duration is not CPU time, mission cost, or a measurement of provider activity. |
| Deterministic controls | `tests/supervision_final_acceptance.py` runs the complete Go inventory, vet, configuration, diff, canonical build, and isolated browser recipes. `tests/supervision_go_suite.py` partitions the Go inventory without duplication across four processes, rejects empty/incomplete coverage, and requires every group to exit 0. | The four processes are test processes, not four agents. Local success does not replace an engine receipt for the same candidate. |
| Review coordination | `review-plan` records bounded groups covering all files and criteria, tied to the candidate and evidence digest. The engine validates the DAG and budget, then the configured independent reviewer inspects groups sequentially before a global synthesis. | A group or partial inspection accepts nothing. Structural validation alone does not prove that the grouping is relevant and creates neither autonomous subagents nor direct reviewer-to-reviewer discussion. |
| Closure | The engine requires a fresh gate and current independent review for every task before closing a non-delegated root. | The supervisor, producer, and this document cannot self-accept. |

## Coordinating the common candidate

The manifest maps each requirement to its components, controls, and evidence.
Before execution, the validation supervisor checks candidate identity, omission-
free coverage, deterministic commands, and total budget availability. After a
correction it invalidates only controls and analyses that depend on modified
inputs, then reruns those items against the renewed candidate. It gives the
independent reviewer the manifest, original outputs, and limits; only the engine
publishes the final decision.

The repository already provides two distinct mechanisms:

1. the deterministic four-**test-process** recipe, which runs mechanical checks
   without a model call;
2. bounded review groups by domain, run sequentially through the configured
   **independent reviewer**, followed by a final interaction review.

The broader architecture in which a **validation supervisor** delegates
qualitative analysis or anomaly investigation to multiple specialized
**subagents** remains proposed. It is not created by test sharding or by
`review-plan`. If implemented later, subagents should not launch deterministic
commands merely to parallelize them: they would deliver bounded analyses, while
the supervisor retains the manifest and cross-domain coverage and the independent
reviewer retains the verdict.

## Integrated historical retrospective

The facts below come from the historical reports identified in the manifest.
**T4 did not rerun them**; they are repository-corroborated context, not new
checks.

| Historical fact | Attribution and nature | Measurement and permitted conclusion |
| --- | --- | --- |
| Two final controls timed out after 300 s with the same output and no usable exit code. | Observable engine-control failure; no provider failure demonstrated. | Two 300 s wall-clock durations. Aggregate CPU, cost, and tokens: unknown. A timeout does not demonstrate a product defect. |
| The conductor repeated the same control without correcting its precondition and started a second producer attempt. | Engine recovery/coordination defect. The second producer changed only the report; the repetition is not product evidence. | Unnecessary retry; history and both attempts preserved; no limit increase. |
| The recovery guard was corrected to reject automatic correction after an unexecuted control or one without an exit code, and to recognize the same cause despite a different receipt path. | Shipped engine defect followed by a supervised engine correction and targeted tests. It is not a provider correction. | Historical targeted tests passed; T4 does not rerun them here. |
| The corrected final recipe covered 974 tests in four processes, followed by vet, configuration, diff, build, and ten browser checks, in about 234 s. | Supervised deterministic control, followed by a favorable independent-review opinion and engine gate in the historical mission. | About 234 s wall-clock for the complete recipe. No aggregate CPU, cost, or token total is provided. The final 974 count supersedes the interim local inventory of 973 recorded before the fresh receipt. |

This chronology therefore separates a **failed control** (the timeouts), a
**shipped and corrected engine defect** (the guard and identical retry), an
**agent action without a cause correction** (the second producer), and
**supervision consolidation** (diagnosis, deterministic partitioning, and handoff
to the reviewer). None of these facts supports attributing the cause to the
provider.

## Loop-free validation sequence

1. Freeze the candidate manifest once and mechanically validate its paths,
   hashes, and coverage.
2. Run one deterministic final recipe under the existing ceiling. Preserve a
   failure and its cause; retry only after a concrete precondition changes.
3. Run the planned bounded domain analyses against exactly the same candidate.
   No partial analysis is a verdict.
4. Have the independent reviewer produce the synthesis, then let the engine
   enforce freshness, gate, and acceptance.

A correction changes the candidate: renew hashes and only the dependent evidence
before resuming at the appropriate step. Do not start a new producer merely to
complete a documentary packet, and do not confuse provider silence, a live
process, a finished control, available evidence, and an accepted result.
