# PAEA — increment 04: correspondence verifier and corrected GLM calls

[Français](../../../../plans/actions-confiance/paea/INCREMENT-04.md) · [Plan](../PAEA-APEX.md)

The user authorized continuation with GLM; no compilation/restart remains in force.
Rather than only extend tests of the same binary, Codex implemented a usable
read-only export consumer: `tools/paea/verify_canonical.py`, with
[operational contract](../../../PAEA-CANONICAL-VERIFIER.md). Initial PA-01/PA-09
component only: no protected project/effect activation, Rust/Cargo/service changes.

The [recipe](../../../../plans/actions-confiance/paea/evidence/20261025-glm-verifier-fadc9ff1/recipe-02/result.json)
is PASS/VERIFIED: **nine named tests**, 42 CLI observations (14 Rust exports,
27 Python verifier calls and one vector generator call), 33 successes and nine expected refusals. Pure calls and
writer doubles supplement these counts, not additional CLI invocations. Historical
test binary SHA256 remains `addcc5eb6b32dd7d2ed963a01a80062155ccd9613985fb518c079f5a461f794f`;
it is not the installed server.

All 14 actual exports first match frozen expected outputs, then pass correspondence.
Mutated canonical text/digests/intent/authority, missing/unknown fields, bool version,
duplicate names, exponents, invalid Unicode and trailing content refuse with valid
controls/recovery. Input bytes and argv/outputs are retained; consumed byte hashes
are checked and public inputs remain unchanged. Request ID/label-order equivalence
is preserved. A mutually forged matching out-of-i64 amount is deliberately accepted
as correspondence only, preventing an admission claim. No authenticity, authority,
effect, compiler graph or E2/E3 assurance is established; schema/oracle are shared
local dependencies.

The original candidate and IO reproducer are retained under the evidence directory.
Directory fdopen failed before fstat, yielding generic IO instead of type refusal.
`/dev/full` initially emitted exit2 then Python exit-flush changed actual exit to120
and added non-JSON stderr. General fixes inspect the owned descriptor before
transfer, close pre-stream paths and write the complete report without buffering
on standard stdout. Public refusal/recovery and simulated partial/zero/error writes
pass. Partial publication remains possible on failure. GLM suggested writer tests
and avoiding a detached Python stdout wrapper; global redirect/detach for a merely
hypothetical later flush issue was rejected without a reproducer. No model-run tests
or independent verdict are claimed.

The user explicitly corrected our unrequested 4096-token GLM output cap. It was
chosen by Codex and required by the old helper, not requested by the user. The
first call exhausted all4096 in reasoning; the focused call used2048. Original
calls are preserved. A new local HTTP regression first failed because argparse
required the allocation. The helper now omits max_tokens/max_completion_tokens
unless a positive allocation is explicitly supplied. No arbitrary reduced reasoning
effort is imposed; server configuration remains unchanged. **Nine helper tests**
pass with synthetic local HTTP, covering actual request absence, explicit values,
pre-send refusal/recovery, credentials, redirects, original preservation and no retry.
These tests consume no SKYNET call. Personal skill instructions are updated using
skill-creator, and original/current personal files are retained in the final review.
Persistent instructions do not update already-running processes.

The final real review request contains no client cap or reasoning_effort. Provider
limits may still apply; omission is not infinite output or unlimited call authority.
Exact usages/verdicts remain in glm, glm-focused and glm-final evidence directories.

Byte-bound review: evidence/20261025-glm-verifier-fadc9ff1/source-review.json.
Policies are existing protocol/representation/technical FD/test corpus choices;
no new Swarm operator setting, watchdog or retry. Public/nontrivial functions and
safe errors are documented through consumers, including secondary stderr failure
retaining exit2. FR/EN contracts, recipe, plan and queue are updated together.
No runtime release note/build/install: repository tool and personal method only.
Declared recipe inputs stay stable, without claiming a compiler graph.

PA-00/01 remain partial; PA-09 is only started. IA3 authority producers, protected
mandates/approvals, effect services and AC-02 confinement/network remain open.
Codex owns integration; Identity/AC-02 owners retain their contracts. Rust integration
requires a qualifiable shared candidate and explicit lifting of no-compilation.
Concurrent session work is preserved. Sponsor still needs the independent evaluator
and third-party team. PA-11…13 and final RETEX remain open; internal review cannot
replace E3. No complete program lot is closed here.

Final uncapped GLM review completed: 5824 input +33773 output (31009 reasoning,
2764 text) =39597 reported tokens, stop after216.927s. Three calls total49798
reported tokens; no price inferred. Requested/reported glm-5.2 does not establish
backend version. Actual request omits caps and reasoning effort. Nonstreaming
transport exposes no intermediate model progress; no watchdog added.

Final review findings were reproduced before correction: boolean true schema
allowed empty context to acquire fixture defaults; weakened schema allowed an
uncaught KeyError. Schema profile/decode diagnostics and independent mandatory
context fields now close these paths. Reports bind schema ID/hash without claiming
authenticity. External references/nested bases refuse to preserve offline behavior.
Standalone generator cache writing was also reproduced in a minimal fixture,
then disabled; all14 generated vectors still match frozen originals. POSIX and
jsonschema>=4 prerequisites and resource limits are documented. Original
schema/cache reproducers and first seven-test PASS recipe are preserved; final
nine-test recipe is fresh. Root owns integration/tests; no repeated review call
is started. Final RETEX remains open with the program.
