> KS document structure adapted for Swarm. Role permissions and engine gates apply.

# Stories Review — <product name>

> Fresh-context review of `docs/stories.md` against `docs/prd.md`. Each issue classified: critical / major / minor.

## Perimeter coverage
| Validated capability | Covered by | OK? |
|---|---|---|
| <feature> | <story id> | PASS / FAIL |

- [ ] Every capability in the validated perimeter is delivered by at least one story

## Scope
- [ ] No story reintroduces an explicitly excluded item
- [ ] No story goes beyond the perimeter

## Story quality
- [ ] Each story is an end-to-end shippable slice, not a technical layer
- [ ] Every acceptance criterion can become a test
- [ ] Agentic notes present and useful (files, constraints, traps)
- [ ] Complexity scored; no unsplit 5; every 4 states its risk

## The list as a whole
- [ ] Dependency order executable: no cycle, no forward reference
- [ ] Ids well-formed (`s<number>-<slug>`), unique and stable
- [ ] No overlap or duplication between stories

## Findings
<one line per issue: severity — story id (or "coverage") — what's wrong>

## Verdict
Max severity: <critical | major | minor | none>
Stories ready: <yes | no>
