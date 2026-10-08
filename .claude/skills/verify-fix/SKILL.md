---
name: verify-fix
description: "Verify that a Swarm fix works through the affected web, CLI or API flow with observable assertions and diagnostics. Use after a repair or when asked to reproduce a regression; a build alone is not a verification."
---

# Verify the actual behavior

Read `tools/agent-workflows/CONTRACT.md`. Identify the reported scenario, expected
effect, candidate revision and actual runtime. Use an isolated fixture root when
an operation changes missions, files or state. Respect the session's tool permissions.
In preparation, specify this recipe without executing it.

1. Before execution, write the trigger, preconditions, inputs, expected observable
   and which assertion would fail if the repair disappeared. Reproduce the original
   failure on a safe baseline when practical; never damage real mission data.
2. Select the boundary: CLI with exit code/stdout/stderr and state readback; HTTP
   with response and persisted effect; browser with the real interaction and DOM.
   Verify the process serves the candidate being tested, not an older installed binary.
3. Attach diagnostics before the first action. For web, collect page exceptions,
   console errors, failed requests and unexpected HTTP errors. For CLI/engine,
   collect stderr, exit status and the relevant structured event/error output.
4. Execute the full affected path, relevant negative input and failure/recovery
   path. Verify persistence after reopening when promised, and no forbidden effects
   after a rejection. Include both themes, languages and keyboard/focus for UI changes.
5. Treat unexpected diagnostics as failures to investigate. Do not add broad
   allowlists, hide errors, disable a guard or replace the behavior under test with
   a mock. Clearly label any external provider double and its proof limitations.
6. Save the precise commands/interactions, revision, environment, assertions,
   outcomes and small evidence artifacts. Do not expose tokens or private data.

Report PASS, FAIL, PARTIAL or NOT TESTED per criterion and list residual limits.
If browser/provider/authentication is unavailable, say which flow was not verified;
do not substitute source inspection for runtime proof. A passing recipe still
does not substitute for required independent review and engine acceptance.
