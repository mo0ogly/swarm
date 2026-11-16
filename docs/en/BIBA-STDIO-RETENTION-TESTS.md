# Inert Git/stdio retention checks — unexecuted proposal

This harness exercises the public `run_git_stdio` consumer and its real Python
`stdio_probe`, using explicitly supplied, hash-pinned source. `subprocess.run`
is replaced before loading the candidate; no command, Biba, Git, driver,
provider, mission or installed service is executed. Fixture/publisher modules
are explicit doubles, not qualification of Admin configuration, atomic writing
or OS confinement. Candidate PASS markers remain captured in private evidence.

## Proposed matrix

Eight declared tests: healthy control; nonzero plus drift or posthash-read
refusal; malformed JSON plus either posthash issue; first post-completion
publication plus either issue; second publication plus drift. Each fault has
fresh healthy controls before and afterwards, including when fault assertions
fail. Every inert call must execute once. `owned-completion.json` precedes the
fault. Exact raw channel bytes are compared. Separate subtests distinguish
missing postcheck, lost channel, lost primary cause, masked publication refusal
and missing structured receipts.

The same file/assertions apply to old/new sources. No version branch, fix-only
symbol or expected-failure decorator converts RED to PASS. First-publication
fault targets old stdout versus the new process receipt; second targets old
stderr versus the new postcheck receipt. This is the same logical boundary
after completion, not a claimed replay of the same low-level call: new channels
are written directly instead of being published afterwards.

## Later execution requires separate authorization

Read the private packet's source-closure.json. Supply `--candidate`, its
`--candidate-sha256`, `--outcome-source`, its `--outcome-sha256`, and an existing
`--evidence-parent` admitted by the storage owner. Run the same
`tests/test_biba_stdio_retention.py` for each candidate. The harness and recipe
do not themselves authorize imports or execution.

The proposed permanent recipe consumes canonical source and these variables:
`SWARM_STDIO_RETENTION_CANDIDATE_SHA256`,
`SWARM_STDIO_RETENTION_OUTCOME_SHA256`,
`SWARM_STDIO_RETENTION_EVIDENCE_PARENT`. It requires success without changing a
refusal into success. The harness retains loaded source bytes, before/after
pins, every trial, unittest output and summary. Optimized Python or a test
collection other than eight cases refuses. Eight is the case inventory, not a
product limit or timeout.

## Source-observed gap

The corrected candidate calls `outcomes.execute` before parsing stdout. When
execute raises on posthash, parsing is skipped: JSONDecodeError is not diagnosed
in the group even though malformed bytes remain. Both malformed+posthash tests
require both causes and may therefore still fail on this candidate. This is a
source prediction, not an executed test. Preserve the requirement; see
OPEN-SOURCE-GAP.md. This packet establishes neither a green recipe nor real
Biba behavior.

## Typed assertion revision — not executed

Causes are recognized only through structured `type`, `exceptions`, `cause`
and `context` nodes. Diagnostics are matched on the very node carrying the
expected type. A RuntimeError message naming JSONDecodeError is therefore not
a parser exception. Receipts must contain exact structured fields: integer
returncode 7 for the nonzero process (0 for malformed JSON), execution attempted,
null signal, postcheck attempted and explicitly failed, and its typed error.
Both channel hashes must match the expected raw bytes. Publication refusals must
be recorded at their own boundary: original process receipt or secondary
diagnostic receipt.

A pure guard inside the healthy test adds no named test. It contrasts deceptive
text with real exception objects, checks grouped/causal branches, and rejects
wrong returncode, integer 1 replacing a boolean, text-only receipts and wrongly
typed publication errors. The guard has not been executed. All eight names,
healthy controls, recoveries, doubles and old/new arguments are retained. Both
malformed+postcheck cases remain open; no requirement is weakened and no frozen
consumer candidate is changed by this harness revision.
