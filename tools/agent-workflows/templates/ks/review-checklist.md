> KS document structure adapted for Swarm. Role permissions and engine gates apply.

# Review — Story <id>

> Fresh-context review. Each issue classified: critical / major / minor.
> Diff reviewed: `git diff <default-branch>...feature/<id>`

## Plan compliance
- [ ] The code does what the plan specifies, nothing more

## Anti-hallucination
- [ ] No invented API/function/import (each one opened and verified)
- [ ] No plausible-but-wrong value or logic
- [ ] The code matches what it claims to do

## Rules compliance
- [ ] Repo conventions followed (AGENTS.md)
- [ ] No accepted ADR contradicted (docs/decisions/)
- [ ] Design system respected — components/tokens from docs/design-system.md, screen matches the intent of docs/designs/<id>.md (UI stories)

## Tests
- [ ] Test evidence bound to the candidate; independent rerun only when the reviewer has execution tools
- [ ] Assertions pin the acceptance criteria (no assertion-free tests)
- [ ] Exact commands and tested revision/image recorded
- [ ] Tests not run are reported as not run, never inferred green

## Regressions
- [ ] No impact on existing code paths

## Findings
<one line per issue: severity — file/location — evidence — what's wrong>

## Verdict
Max severity: <critical | major | minor | none>
Ship allowed: <yes | no>
