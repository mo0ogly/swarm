# Rust migration: user-adopted delivery method

For the Go → Rust migration, apply the user-adopted
[method prompt](../../../../docs/plans/migration-rust/PROMPT-METHODE-CODEX.md)
(22 October 2026). These decisions are scoped to this migration; they do not
impose Go parity or a cutover strategy on unrelated APEX work.

1. Classify each G1–G12 surface in CHEMIN-CRITIQUE-SPEC.md before its lot.
   Strict parity covers persisted data, agent/script-consumed CLI JSON,
   security/identity/budget/rights refusals and agent launch/supervision.
   Rewrite other surfaces from user journeys. Mixed groups retain their contract
   boundaries: presentation may differ when documented; meaning, rights and data
   may not drift.
2. Use a differential bench for contracts: identical scenarios on isolated,
   equivalent Go/Rust test Roots; identify both binaries and compare JSON types,
   refusals/exit codes, logical database state and observable process effects.
   Keep an explicit, justified per-scenario allowlist; an unlisted difference
   fails. Extend the permanent corpus with each covered command. Retain regression
   guards for reproduced defects; the bench replaces manual parity demonstrations.
3. Deliver vertical user journeys covering applicable route, CLI, UI and data
   together, with a scoped internal guard. Avoid standalone internal-guard lots
   unless correcting a reproduced defect. Order prerequisites before dependants,
   then preparations, automation and cockpit by remaining scope.
4. Proportion evidence to risk: A (security, identity, budget, data, agent
   processes) requires a frozen snapshot, valid controls before refusals and
   independent review; B (other engine logic) uses targeted unit/integration
   checks and a final group recipe; C (UI, reading, help) uses journey tests and
   visual inspection where applicable. A read surface handling protected data
   still carries its A-level contract. Run one complete recipe at lot end;
   between edits run targeted checks. Apply replan immediately on anomalies;
   three consecutive failures at one point additionally require scope reduction,
   never a fourth unchanged recipe. Preserve required evidence and original failures.
5. Defer hardening beyond Go to an “after cutover” register unless it repairs a
   reproduced defect or is required by a contract surface. Retain origin, expected
   value, scope, owner, observable outcome, actual status and resumption condition.
6. Keep PLAN.md short: about fifteen lines per lot for objective, journey, risk,
   status, evidence link and blockers. Preserve historical details in evidence
   files. Track G1–G12 routes/commands as differential-parity verified or
   journey-accepted, with explicit inventory denominators; do not infer percentages.
7. When delegation is authorized, assign non-overlapping owned files. Announce
   additive shared-file changes in PLAN.md before writing; freeze shared inputs
   during another agent's recipe. Coordinate Claude's LIVE-UX files separately.
   This method itself grants no delegation or execution authority.
8. Prepare R6/R7 alongside the groups: data migration with verified restoration,
   rollback procedure and installation. Maintain a single final writer cutover;
   no dual writer. Earlier read-only Rust service on a test Root remains optional
   and requires explicit user confirmation before activation.

Documentation, function contracts, error handling and operator configuration
remain required in each increment. Evidence reuse is limited to unchanged relevant
inputs. A method update neither closes lot 201 nor establishes product acceptance,
installation, independent review or adoption by already running agent contexts.

## Cost discipline adopted on23 October2026

Apply the existing method as one public delivery cycle, not repeated standalone
component cycles. The active unit has a named user command/journey and observable
end condition. Assemble the existing pieces first; implement missing producers
inside that unit, without filling unavailable data with invented values. An SDK
or internal guard does not close a public journey.

During edits, run only checks discriminating the changed behavior or a diagnosed
failure. Reuse unchanged passing proof. Consolidate the complete journey recipe,
required independent review and final profile/binary qualification at the end;
reopen only for changed inputs, a new failure or an unresolved affected contract.
Preserve risk-A controls, original failures and error/rights/budget contracts.

Keep one short status and one final handoff per unit; retain raw execution proof
automatically instead of repeatedly rewriting narrative reports. Function docs
and relevant FR/EN product documentation remain part of the same increment.
Give each authorized agent a small file-owned assignment with only the necessary
interfaces and proof references. Do not add agents by default or resend the full
migration history; measure useful public outcomes rather than proof-stage counts.
Record wall/build/test time separately and tokens only when actually available.
Do not claim a speedup or estimate days from unweighted file/test/sub-lot counts.
This source reference does not update already embedded/running agent contexts;
the next assignment must explicitly convey this adopted cost discipline.
