# A0 oracle — properties over retained evidence

`a0_property_oracle.py` evaluates each historical A0 row offline. It neither replays the receiver nor opens SQLite nor calls a provider. Historical result labels are never oracle verdicts.

```sh
python3 tools/research/a0_property_oracle.py --index docs/recherche/reunion-limos-evalia-20261117/A0-PREUVES.json --output /path/new-directory
python3 tests/test_a0_property_oracle.py -v
```

## Inputs, integrity and output

The retained index supplies `location`, `inventory_sha256` and `journey`; `A0-INVENTORY.json` maps relative paths to SHA-256. Explicit `inventory_path`, `inventory`, `journey_path` and `result_path` variants are validated too. Traversal and symbolic components are refused. Every inventoried file is hashed before and after evaluation; consumed hashes must match. These checks detect observed drift, but do not provide an atomic snapshot against a concurrent hostile writer.

Numbered requests, results, process records and persisted snapshots produce the events. The archived driver establishes that `initial.observation.json` follows initialization; successful `present` is read-only. These are explicit adapter assumptions. The archived driver matches the current source read byte for byte. A response claiming commitment never suffices: a new persisted effect row must corroborate it.

The output directory must not exist and is reserved exclusively. Complete files are synchronized and linked without replacement; `COMPLETE.json` binds only the published artifact hashes; its presence alone never proves that the process returned success. Verifying a run also requires retained successful execution evidence (a terminal receipt) in addition to the hashes. IO failure retains the original OSError cause and partial output; when the failure follows marker publication (final directory fsync), the marker may exist and publication is reported uncertain, with no simulated rollback or unlink. Existing output is preserved. Exit 2 means global refusal; exit 0 means a report was produced, including any `fail`, `error` or `inconc` verdicts.

## Verdict scope

- PT compares complete action terms with the persisted approval.
- PA checks the available authorization projection, prior use and revocation history; PR checks no earlier effective revocation. Observed logical channels are assumed enabled; OS/network identity is not established.
- PV1 checks existence of a preceding effect for the identifier; receipt digest equality belongs to PT, not PV1's stated formula.
- PV1/PV2 have explicit intention (`intent_id` within the case) and attempt (numbered command) projections. Unprojected atoms remain inconclusive until the article resolves i.
- Unknown responses trigger neither PV1 nor PV2. Annotated vacuity establishes no recovery quality.
- PL is inconclusive with withdrawal because Delta and request/acknowledgment times are unavailable; without withdrawal it is vacuous only. Real-time expiry is not modeled.
- Trace gaps remain visible. No order is invented among simultaneous observed effects, although two uses of one grant establish at least one reuse.
- Every snapshot of a case must bind one case-global run (`case_run`); a divergence between snapshots is an evidence error and no effect diff is derived across runs.
- An effect row that disappears, or is rewritten under the same identifier, between two snapshots is a persistence inconsistency: affected properties become `error`/`inconc`, never a merely annotated pass; a sound earlier fail witness is retained.
- PT concludes failure (absence of approval) only when the prefix before the commit is completely observed; otherwise `inconc`. A found exact approval witness is retained even if an unrelated later hole appears.

Aggregation preserves the strongest witness: fail, error, inconclusive, pass. Vacuity is not exercised compliance. JSON retains traces, witnesses, notes and hashes; the table includes every historical row, including an assertion with no attributable trace.

## First real evaluation and limits

The private first evaluation is retained at `/run/media/fpizzi/SWARM/swarm-evidence/article-v6-oracle-kjevtgz4/evaluation-r1`: 83 rows, 2324 inventoried files, no campaign replay. It finds PT failure in `E1-W_sensitivity` and PV2_intention failure in `E1-V06` (effect followed by refusal for the same intention). V06 does not fail PV2_attempt. At the historical `evaluation-r1` stage, before the corrective rounds described below, the effects of the new rules on the real corpus were not yet established and the table still needed to be re-derived. That limitation belongs to r1; the retained r3 and r4 evaluations below subsequently establish a new 83-row table with no observed verdict changes, within their documented scope.

Sparse snapshots in `E1-V05_revoke` and `E1-V05_policy`, and non-numbered transport snapshots in `E1-V07`, are not fully resolved. Affected verdicts stay inconclusive. `E1-Q_evidence_refusals` has no separately attributed trace. Historical PASS labels never fill these gaps. Real evaluation, synthetic checks, independent review and scientific acceptance remain distinct. No universal Swarm, H2, expiry or PV2 claim follows.

## Corrective rounds and evidence scope

The first repair candidate passed 32 tests and preserved the 83 verdicts in
`article-v6-oracle-repair-bpscx4x9/evaluation-r3`. The following review closed PT,
uncertain publication and index binding; two variants remained: terminal evidence
ignored beside an explicit final after-state, and uniformly foreign snapshots
not bound to the requested initialization run. Both were reproduced after healthy
controls and then corrected.

Every snapshot now binds the corroborated successful initialization request's
case_run. Initial snapshots must match its configuration and the receiver's
fresh-store empty effects. Terminal evidence is checked despite an explicit last
after-state. Unattributed extra effects create a trace gap, never a fabricated
commit; disappearance or rewrite creates an evidence error.

Thirteen targeted tests pass on this correction, covering the two new guards and
affected prefix/snapshot/PT/PV2/unknown/shape consumers. Earlier 32 PASS results
remain bound to earlier bytes. The permanent manifest now selects 34 maintained
tests; this complete manifest version has not yet been rerun. Corrective review
and acceptance remain open. Successful terminal exit and marker hashes are both
required to verify an evaluation.

Corrected evaluation r4 is retained in
`article-v6-oracle-repair-bpscx4x9/round2/evaluation-r4`: 83 rows, zero error
verdicts and no verdict changes versus r3. Retained exit 0 and marker hashes were
checked together. This establishes the candidate evaluation, not independent
approval or universal compliance.
