# A4 — Private reference-fixture candidate

Private candidate: results belong in receipts bound to exact input hashes, not in
a source label. This handoff changes no consumer checkout file. Four new paths
are proposed: `tests/a4_relay_contract.py`, the French and English contract notes,
and `tools/verification/recipes/a4-relay-contract.json`. Shared integration and
its reservations still require allocation.

## Scope and data

The standalone Python driver neither imports nor launches Swarm. It exercises
only its fictional model and oracle: actors A/B, operator-registered originals,
in-memory R1 then R2 doubles, an application point and effect observations.
Known/engaged/unknown states are scenario declarations. No remote effect, process,
OS confinement, cryptographic authority, AI supervision or native integration is
tested. Passing this fixture does not close native A4 qualification.

The immutable original is registered separately from receipts. R1 receives that
order; R2 receives the actual R1 receipt, payload and predecessor. The fixture
guard checks the trusted original, ordered chain, linkage and recorded receipts.
In-memory object identity is not a distributed transport guarantee. Handoff and
receipt apply nothing. Only validated application changes running state. Resume
requires another ID, a new registered decision and a reference to the latest
applicable stop. The same operator may make that new decision; a different human
or authority identity is not imposed as a product prerequisite.

## Proposed scenarios

| Criterion | Assertion | Status |
| --- | --- | --- |
| Healthy A/B control | Both act; separate healthy stop and authorized-resume clones precede their corresponding variants | See bound receipt |
| Two relays | R2 receives R1; receipt alone does not stop A; application does, and B subsequently acts | See bound receipt |
| Altered scope or meaning | At each relay: B/global target, discussion ban, replaced issuer/decision refuse; original remains fixed | See bound receipt |
| Incomplete chain | Missing, reversed, unlinked or altered receipts refuse; valid chain still applies | See bound receipt |
| Replay | Duplicate application adds no decision and cannot replace the original | See bound receipt |
| Effects and recovery | Existing engaged/unknown observations remain; new A actions refuse; a new linked decision restores A/B | See bound receipt |
| Invalid resume | Unknown order, same decision, wrong stop or no applied stop refuse; authorized recovery follows | See bound receipt |
| Oracle sensitivity | Deliberate fixture-only guard omission stops B or creates a new A effect; no engine mutation | See bound receipt |

Refused operations raise a stable `Refused` code before changing complete fixture
state, including logs and receipts. No diagnostic is stored in that state. Tests
compare before/after state rather than only catching an exception. Existing
effects are never erased. No real compensation or reconciliation is simulated;
authentication decisions remain outside this fixture.

## Historical provenance of Go / Rust comparisons

This section describes working sources read on 15 November 2026, not capabilities
of a delivered version. Exact excerpts and their source hashes are retained in
private evidence `a4-private-20261115-9801ea9866/context.json`, SHA-256
60db7e052c73c1118c236c77da267329f9830fc7675b3e4d8d00b30dbd601706.
The local evidence index retains its location and receipts. Neither this context
nor the archives below are claimed to be files delivered by main.

The following two Go identifiers refer to sources retained on disk in the
canonical checkout, with the same bytes supplied to GLM. Neither path is present
in CI tree `019dd19c1f92ec7fe0bd2df14639a25e96f5cf9e`
or the observed local main reference `a7bb657fb8b56c410913c2f6850e74201b4dec52`.
These observations concern those exact revisions, not a future main.

| Historical source identifier, not a delivered link | SHA-256 of retained original |
| --- | --- |
| internal/engine/run_supervision.go | c4b53f49951102f8fe9778ac3b8b8a8f1868ef63ba38b601bd8e07343b84c01f |
| internal/engine/run_supervision_test.go | 93d492c29d71953acbf611a2c3582789170dfe3793e93ab51587fc5220832450 |

Full originals, read back and hash-checked, are retained under
`/run/media/fpizzi/SWARM/a4-evidence/a4-private-20261115-9801ea9866/archives-doc04/`
using the same subpaths; provenance.json retains the Git-tree observations.
This is a private evidence archive, not a dependency or public link. In those
originals only, lines 125–195 of the first describe stop-run binding to task/scope,
attempt and observation; lines 66–150 of the second contain a confirmed-stop
scenario. The Python driver does not call either. The label "historical Go" proves
neither current Go/Rust conformance nor the presence or absence of a capability
in main. This candidate adds no Go file.

The Rust comparisons below are likewise limited to working-source excerpts
retained in that same context.json. They qualify neither installed binary bytes
nor current parity with the Go originals:

- `rust/crates/swarm-cli/src/agent.rs:55–100` and
  `rust/crates/swarm-engine/src/root_store.rs:1790–1812` show recorded intent and a
  CLI receipt with termination_confirmed=false in those excerpts.
- `rust/crates/swarm-engine/src/agent_supervision.rs:414–451` observes desired=stop
  there and may request TERM then KILL under that source policy. No signal or
  actual termination is exercised by the synthetic fixture.
- `rust/crates/swarm-engine/src/planning_decision.rs:325–340,698–715` refuses
  unsupported operations and specialized run_observation inputs there. This source
  observation alone is insufficient to justify porting or repairing the engine.

## Assigned command and delivery limits

In an allocated private fixture with stable retained inputs:
`python3 tests/a4_relay_contract.py -v`.
Seven named methods, with subTest variants, are proposed. The actual count and
outcomes are in the corresponding named output. No global watchdog, timeout,
retry, network, shell, child process, provider or real mission is used. R1/R2,
A/B and literal states are scenario data/invariants, not operator policies.
Only the Python standard library is required.

The proposed permanent recipe declares both contract languages and the driver,
expects the seven names, count and OK, and is distinct from a direct command run.
Its inventory does not qualify a native compiler graph or engine build, which
this fixture does not consume. Keep complete output, exit status, input hashes
and assertion results. Any failure stays open; an oracle defect calls for a
fixture correction, not an alleged Swarm repair. A later native qualification
must select a public path and bind it to actual producers/consumers. No build,
installation, Git integration or delivered product capability is claimed here.
