---
name: code-reviewer
description: "Review a Swarm implementation diff for reproducible correctness, isolation, security and recovery defects. Use for code review or independent candidate assessment, rather than specification review or general repository scoring."
---

# Review a candidate

Read `tools/agent-workflows/CONTRACT.md`. Identify the candidate/base revision,
dirty changes, assigned scope and acceptance criteria. Review without modifying
the candidate. A review in the implementing agent's own context is a self-review;
it cannot satisfy the engine's independent reviewer requirement.

1. Read the diff and surrounding call paths, public callers, invariants and tests.
   Trace behavior rather than reporting a suspicious string in isolation.
2. Check admission and acceptance boundaries, role authority, stale evidence,
   atomicity, idempotence, concurrency, workspace paths/symlinks, provider failures,
   cancellation and restart behavior when affected.
3. Compare tests with requirements. Run relevant checks only if the review runtime
   permits tools and isolation. Otherwise identify missing evidence; never claim
   test execution from reading a report. A tool-free Swarm reviewer stays tool-free.
   In a producer delivery, `status=pass` means locally demonstrated coverage and
   `outcome=complete` means local delivery completeness; neither claims independent
   acceptance. Do not demand an already favorable review as a prerequisite to the
   review itself. Inspect the actual local evidence and its scope. In a fragment,
   a required source absent from that packet remains `unknown` with a precise
   need for final collation; absence from one packet does not prove absence from
   the complete dossier. Compare historical failed runs with the exact commands,
   source revisions and later original receipts before asserting a contradiction.
4. Report actionable introduced defects with severity, exact location, triggering
   precondition, observable consequence and a concrete reproduction or argument.
   Separate confirmed findings from unverified risks. Avoid preference-only findings.
   Copy quotes from the supplied original byte-for-byte at its physical line;
   preserve spaces, punctuation and patch signs ("x ;" differs from "x;").
5. Re-check claimed fixes against the same candidate. A later code change requires
   renewed checks/review; approval on a different SHA is not current evidence.

Return the required review schema if Swarm supplies one. Otherwise give candidate,
findings, checks performed, evidence gaps and verdict: approve, request changes or
unable to verify. No findings does not mean all untested behavior is proven.
Only the engine performs task acceptance under its complete contract.
