# Delivery recovery: native launcher and on-premise setup

Base: main be8e05c. Scope authorized: publish the missing native launcher and
its dependencies, audit local delivery gaps, document generic on-premise Docker
overrides, and keep organization-specific installation documents private.

## Findings and plan

R1: `swarm.sh`, `tools/swarm_local.py` and lifecycle tests were untracked locally
and absent from GitHub. Publish the coherent set, verify with the main binary.
R2: local launcher expected an unauthenticated 200 supplied by an unpublished
sign-in UI. Authenticate readiness probes with the project's session cookie;
do not ship that unrelated UI or its cookie-policy changes.
R3: document explicit Compose file selection, validation/recreation, internal
CAs, DNS/proxy alternatives, persistence and project/method paths.
R4: ignore local overrides, certificates and private installation artifacts.
Add a distribution guard requiring the launcher dependency set to be tracked,
and execute real lifecycle tests in CI after building the candidate.

Other untracked/dirty work in the primary checkout includes web access UI,
preparation recovery, benchmark analysis and publication edits. It is preserved
and excluded from this delivery. None is automatically considered approved or
necessary to the launcher. Historical qualification manifests are unchanged.

## Verification

Lifecycle tests use temporary roots, the binary built from this main-based
candidate, and dynamic loopback ports. They cover restart/data persistence,
private browser opening, unrelated listener and stale PID protection, exact
legacy adoption and atomic binary replacement. No live mission DB is used.

Self-review only in this conversation; not an independent review verdict.
Remote organization's network and certificates cannot be validated locally.

Local results: PASS, 7 real lifecycle tests (11.809 s); PASS canonical build,
launcher shell syntax/help, workflow-contract check, distribution check,
staged diff whitespace check and Docker Compose CA override validation.
Git ignore probes match private DOCX/PDF, private document directory,
local override and certificates. Public additions contain no organization URL,
internal address, key or certificate. GitHub CI remains the final merge gate.
