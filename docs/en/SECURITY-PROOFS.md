# Security and evidence

[Plan and remaining work](plans/actions-confiance/SECURITY-PROOFS-APEX.md).

The first integration is readonly: current BLP configuration and embedded declared
inventory. `/security-proofs.html` is linked from the Rust cockpit and Trust actions.
It requires native session admission and a current Trust I3/admin bearer; an
Authority session creates no Trust identity. Credentials remain in page memory and
are cleared on navigation or the explicit clear action. Input edits invalidate results.

CLI: `swarm-rust trust security-show --root ROOT --input FILE --json`, with
`SWARM_TRUST_TOKEN` in the environment and `FILE` containing `{"schema_version":1}`.
API: POST `/api/v1/trust-actions?action=security-show`, same body, native session,
CSRF and Bearer authentication. Unknown fields, supplied results or paths refuse.
Shared Trust reservations, state validation, Root binding and close failures retain
their existing contracts. No publication or mutation.

`security-observation-v1` contains domain `blp`, existing BLP capability, compiled
build declarations, SHA256 of embedded inventory/catalogue, boundary names/owners/
open work packages and recipe references. References are data, never commands.
The observation digest is the canonical report digest before adding that field;
it is not a signature, binary hash or profile qualification. No grants, attributions,
reasons, private paths, project content or credentials are disclosed. Existing
administrator rights govern access.

Configuration, verification and qualification remain independent:
`runtime_enforced:false`, `verification.result:NOT_TESTED`, freshness `unknown`,
`evidence_loaded:false`, qualification `not_qualified`, independent review and
external certification `false`. These describe the absence of loaded evidence in
this observation, not an assertion that historical recipes were never run.
Absent/current/renewal-required policy states are distinct. The 25 OPEN boundaries
come from the compiled inventory, not an exhaustive dynamic measurement.

Policy administration retains the existing read/preview/apply form in Trust actions.
Recipe execution, evidence import/freshness, separate read permissions, export and
qualification decisions remain L04–06. Biba and composition require integration.
No button declares certification or enables global enforcement.

Maintained recipe `security-proofs-engine.json` checks metadata. Public verification
must also cover CLI/HTTP/UX and actual delivered bytes. Component verification is
neither installation nor independent acceptance.


## First integration delivery

Version dev.336 is active locally. Engine and CLI/HTTP/UX recipes are verified on
owned fixtures using the same normal image: 77 browser and 22 CLI/HTTP assertions,
plus 56 on the existing Version dialog. Real Authority-session coverage remains
open pending human login; no technical key became a human session. See the plan
for evidence and limits. Maintainers must update recipe inputs/counts when the
inventory changes and explicitly select the installed browser with the shared
collector, rather than assuming a Puppeteer cache.
