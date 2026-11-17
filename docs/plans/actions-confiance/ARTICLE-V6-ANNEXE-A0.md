# Annexe A0 — évaluation des 83 rangées conservées

Évaluation hors ligne du corpus historique, sans rejeu ni nouvelle expérience.
Code et limites : [oracle](../../../tools/research/A0-PROPERTY-ORACLE.md).
Source évaluée : SHA-256 `22ce18d5e9230a2d96399ea4ef14c7a615c99369ecd45af34ed6e458c9db2e66`.
Rapport complet : `/run/media/fpizzi/SWARM/swarm-evidence/article-v6-oracle-repair-bpscx4x9/round2/evaluation-r4/a0_oracle_report.json`.
Le code de sortie 0 conservé et les empreintes de `COMPLETE.json` ont été relus ;
le marqueur seul ne constitue pas une preuve de succès de l’exécution.

La seconde revue a clos les corrections PT, publication incertaine et index,
mais demandé deux variantes supplémentaires : contrôle terminal malgré un after
explicite et lien avec le run demandé à l’initialisation. Les gardes reproduisent
ces variantes puis passent après correction. Treize tests ciblés passent sur
ce code ; les 32 PASS antérieurs restent attachés aux anciens octets. Le manifeste
permanent prévoit 34 gardes ; sa nouvelle version complète n’a pas encore été réexécutée.
Les verdicts réels restent identiques aux évaluations antérieures : aucune incidence
de ces deux défauts n’est observée sur les 83 rangées. La nouvelle revue et
l’acceptation scientifique restent ouvertes. Les passages vacuitaires, trous de
trace et projections gardent leur portée. Le récepteur A0 n’est pas le binaire
Swarm ; aucune conformité universelle n’est déduite.

# A0 property oracle - per-case results

source location: `/run/media/fpizzi/SWARM/evalia-a0-20261117T190328-e4c80fd6`
inventory sha256 (validated): `99ee38f241a141b60285d25660f8a142a31e10f15eb6c18c36037a4d6d9d4db2`
historical rows: 83 (statuses recorded, never used as verdicts)

Verdicts: pass / FAIL / inconc / error; `pass (vacuous)` marks properties with no triggering instance; coverage is annotated separately from verdicts. See a0_oracle_report.json for traces, digests and witnesses.

| case | recorded (not a verdict) | PT | PA | PR | PV1_intention | PV1_attempt | PV2_intention | PV2_attempt | PV1 | PV2 | PL | witnesses / notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| E1-C01 | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-C03 | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-C02_strict_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-C02_strict | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | inconc | 4 detailed note(s) in the JSON report |
| E1-C02_weak_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-C02_weak | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | inconc | 4 detailed note(s) in the JSON report |
| E1-C04_strict_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-C04_strict | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-C04_weak_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-C04_weak | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-V01_beneficiary_alternate_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V01_beneficiary_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V01_beneficiary | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V02_amount_alternate_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_amount_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_amount | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V02_source_alternate_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_source_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_source | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V02_ledger_alternate_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_ledger_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_ledger | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V02_intent_alternate_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_intent_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_intent | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V02_requester_alternate_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_requester_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_requester | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V02_scope_alternate_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_scope_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_scope | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V02_run_alternate_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_run_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_run | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V02_schema_currency_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_schema_currency | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V02_schema_operation_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V02_schema_operation | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V03_substituted_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V03_substituted | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-V03_incomplete_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V03_incomplete | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-V04_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V04 | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V05_revoke_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V05_revoke | PASS | inconc | inconc | inconc | pass (vacuous) | pass (vacuous) | inconc | inconc | inconc | inconc | inconc | 1 detailed note(s) in the JSON report |
| E1-V05_policy_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V05_policy | PASS | inconc | inconc | inconc | pass (vacuous) | pass (vacuous) | inconc | inconc | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-V06_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V06 | PASS | pass | pass | pass | pass | pass | FAIL | pass | inconc | inconc | pass (vacuous) | PV2_intention: refusal step 5 intent 'invoice' follows commit step 4 |
| E1-V07_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V07 | PASS | inconc | inconc | inconc | inconc | inconc | inconc | inconc | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-V08_zero_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_zero | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V08_negative_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_negative | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V08_overflow_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_overflow | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V08_float_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_float | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | inconc | inconc | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-V08_leading_zero_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_leading_zero | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V08_boolean_revision_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_boolean_revision | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V08_unknown_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_unknown | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V08_missing_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_missing | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-V08_duplicate_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_duplicate | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | inconc | inconc | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-V08_exponent_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-V08_exponent | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | inconc | inconc | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-Q_before_commit_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-Q_before_commit | PASS | pass | pass | pass | pass | pass | pass | pass | inconc | inconc | pass (vacuous) | - |
| E1-Q_after_commit_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-Q_after_commit | PASS | pass | pass | pass | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | 1 detailed note(s) in the JSON report |
| E1-Q_observer | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-Q_evidence_refusals | PASS | inconc | inconc | inconc | inconc | inconc | inconc | inconc | inconc | inconc | inconc | 1 detailed note(s) in the JSON report |
| E1-Q_approval_identifier_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-Q_approval_identifier | PASS | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | pass (vacuous) | inconc | inconc | inconc | inconc | inconc | 2 detailed note(s) in the JSON report |
| E1-W_beta_authorized | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-W_sensitivity_control | PASS | pass | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | - |
| E1-W_sensitivity | EXPECTED_COUNTEREXAMPLE | FAIL | pass | pass | pass | pass | pass (vacuous) | pass (vacuous) | inconc | inconc | pass (vacuous) | PT: commit step 4 approval 'approval': committed action differs from approved action |

## Counts

| property | pass | inconc | error | FAIL |
| --- | --- | --- | --- | --- |
| PT | 78 | 4 | 0 | 1 |
| PA | 79 | 4 | 0 | 0 |
| PR | 79 | 4 | 0 | 0 |
| PV1_intention | 81 | 2 | 0 | 0 |
| PV1_attempt | 81 | 2 | 0 | 0 |
| PV2_intention | 74 | 8 | 0 | 1 |
| PV2_attempt | 75 | 8 | 0 | 0 |
| PV1 | 0 | 83 | 0 | 0 |
| PV2 | 0 | 83 | 0 | 0 |
| PL | 78 | 5 | 0 | 0 |

## Assumptions and limits

- A0-projection: a channel recorded in the observed configuration is treated as an enabled logical channel (presence in configuration = enabled). No OS/network identity is proven; this is an explicit A0 assumption, not a Swarm proof.
- Expiration and real-time validity of approvals are NOT modeled in A0; no result here speaks about real-time expiry or Swarm behaviour.
- PL: the bound Delta is unspecified and request/ack timing is not observable; any case containing a withdrawal (a revoke request, or an updated response whose request bytes are unparseable) is inconc; cases without withdrawal are vacuous only. No latency is invented.
- The article identifier i is unresolved between business intention (intent_id) and per-attempt identity; unprojected PV1/PV2 are reported inconc and two explicit projections are emitted; no universal compliance is claimed.
- Events are derived only from success responses corroborated by persisted snapshots; commit events come only from new rows in before/after effect diffs. A claimed response alone is never an event.
- Responses with status 'unknown' trigger neither PV1 nor PV2; their coverage is annotated separately from verdicts, and an unknown-response pass never establishes recovery quality.
- 'past' includes the current trace position; each response is evaluated only against its own past prefix (no retroactive reclassification, e.g. a before_commit refusal followed by recovery is not an earlier PV2 fail).
- Every snapshot is bound to the successful initialization request case_run, not only to the other snapshots. Initial captures must match the fresh-store initialization; terminal captures are checked even with an explicit final after-state.
- Historical result.json statuses (PASS / EXPECTED_COUNTEREXAMPLE) are recorded for traceability and are never used as oracle verdicts.
- Digest comparisons use stored bytes only; no digest is recomputed for verdicts.
- No total timestamp order is invented across actors; ordering is the numbered sequential command order inside each case.
- Vacuous passes are annotated via coverage and never establish triggered compliance.
- An unobserved or corrupt snapshot anywhere in a case is a trace hole: affected properties become inconc (an earlier fail is never overwritten). Invalid raw request bytes keep refusals unattributed rather than inventing an identifier.

Evaluation executed on the source bytes recorded above. Independent scientific acceptance is not established by this tool; vacuous and inconclusive verdicts retain their limits.
