# G6 review package — handover to designated third parties

9 November 2026 · [Français](../../../../plans/actions-confiance/paea/REVIEW-PACKAGE-20261109.md)

Lot G6 of the [gap-closure plan](GAP-CLOSURE-PLAN-20261103.md): "Review
package, contracts/vectors, collection and B2 comparator with the same
business protections". With this document, all four local G6 objects are
delivered: contracts and 14 vectors, collection (increments 14-25), the
[B2 comparator](B2-COMPARATOR-20261109.md) and the **review package** — the
closed dossier handed to the designated third parties (gate H3) when they
accept the mission.

## 1. What the package is — and is not

`tools/paea/review_package.py` assembles the closed inventory of the E3
preparation deliverables present in the repository, each entry with its exact
SHA256 fingerprint and byte size, published atomically via
`recipe.py::publish` into `paea-review-package.json` (schema v1). Three
folders, the structure required by [OPERATIONAL-PILOT](OPERATIONAL-PILOT.md)
(conception / realization / qualification):

- **conception** (7 entries): E3 protocol, integration guide, operational
  pilots dossier, gap-closure plan, dashboard, G3a egress-path inventory, B2
  comparator deliverable;
- **realization** (21 entries): the seven `tools/paea/` modules, the pilot
  test driver, the contention helper, the `contract-v1.json` /
  `payment-registry-v1.json` / `payment-request-v1.json` fixtures, and the
  `rust/crates/swarm-protection-contract/src/*.rs` sources;
- **qualification** (5 entries): the maintained recipes `paea-contract`,
  `paea-pilot-lab`, `paea-pilot-contention`, `paea-laboratory-evidence` and
  `paea-b2-comparator`.

The manifest carries explicit honest labels: it **proves** existence and
byte-identity at assembly time; it proves neither quality, nor acceptance,
nor independent evaluation, nor preregistration, nor a reserved corpus. The
handover gates stay H3 (evaluator + second implementation designation) and
H4 (business criteria, human trials). It never stands in for preregistration
or the compared campaign.

## 2. Assembler contract

- Any missing or empty deliverable **refuses** assembly
  (`review-package-deliverable-missing` / `-empty`); a glob matching nothing
  is the same refusal as an absent path — a half inventory is never
  published.
- **Double collection**: two complete passes must produce the same
  fingerprints; any source changed during assembly (shared tree, concurrent
  session) refuses with the divergent paths listed
  (`review-package-source-changed-during-assembly`) and nothing is published.
- Evidence in the `SWARM_PAEA_REVIEW_PACKAGE_EVIDENCE` directory (created by
  the script, must be new) — never under /tmp.
- The package is **reproducible**: two runs without source change produce
  byte-identical manifests.

## 3. Actual result (9 November 2026)

- 33 deliverables (7 + 21 + 5), 324,890 bytes, manifest sha256
  `f981d9b7e2d4d7a85ca65b0e8a4ebbb71b4bc63c564ba91c52af43ce148b378b` — three
  consecutive runs byte-identical, including the public path
  (`review-package-run1/`, `run2/`, recipe campaign `rp-recipe-campaign2/`
  under `.swarm-fixtures/`);
- public execution: recipe `tools/verification/recipes/paea-review-package.json`
  PASS (evidence `rp-recipe-evidence2/`), independent verification
  `{"integrity": "VERIFIED", "recorded_status": "PASS"}`; the first launch
  without `SWARM_PAEA_REVIEW_PACKAGE_EVIDENCE` is retained FAIL
  (`rp-recipe-evidence/`) — the refusal without the variable is part of the
  contract;
- refusal variants verified: missing deliverable → refusal; empty deliverable
  → refusal; missing evidence variable or already-existing directory →
  refusal.

## 4. Handover — concrete resumption condition

The actual handover waits for H3 designation (the user confirmed on 25
October having neither an independent evaluator nor a third-party team; no
roles are invented). When third parties are designated: regenerate the
manifest (today's fingerprints become the offer), attach
`paea-review-package.json` + the listed deliverables + [E3-PROTOCOL](E3-PROTOCOL.md)
as the mission, and keep each party's acceptance proof. Any later change to a
deliverable invalidates the manifest fingerprint — the package regenerates,
it is not hand-updated.

## 5. What remains open in G6

- **Preregistration** (gate H3/H4): evaluator identity and mission
  acceptance, reserved corpus with provenance, volumes/order/seeds/stop
  rules, business criteria. None of these can be manufactured locally.
- The compared campaign (candidate vs B2 vs second implementation): PA-12
  gate, after preregistration.
- A regenerated invariant matrix will be attached to the package at handover
  time (7 November delta note, section "Suivi du 7 novembre (fin)").
