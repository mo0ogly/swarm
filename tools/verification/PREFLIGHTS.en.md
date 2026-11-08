# Local archive and Git preflights

[Français](PREFLIGHTS.md) · [Recipes](README.md)

These explicit JSON CLIs are local tooling, not engine permissions. They do not
change missions/services or automatically run in sessions. PASS describes a local
observation and grants no later mutation authority.

## Archive: check actual original members

`archive_preflight.py` requires Python 3.13+ and Unix O_NOFOLLOW/O_NONBLOCK. Supports
plain uncompressed tar, optionally explicitly ordered parts, without concatenating
large files on disk or extracting anything.

```sh
python3 tools/verification/archive_preflight.py --part /PATH/archive.tar --manifest /PATH/expected.json
python3 tools/verification/archive_preflight.py --part /PATH/archive.part000 --part /PATH/archive.part001 --manifest /PATH/expected.json
```

Strict JSON manifest: exactly `schema_version` (integer 1) and nonempty `members`
(canonical relative POSIX path → 64 lowercase hex SHA-256). Expectations come from
operator-selected originals/inventories, not a name index alone. Duplicate JSON
keys, ambiguous/absolute/traversing paths and invalid hashes refuse.

Selected members must be unique regular files with exact hashes. Directories,
symlinks and hard links are not originals. Scan continues to logical end, checking
late duplicates. This contract requires a two-zero-block tar terminator and
zero-only padding, so truncated/corrupt later headers cannot become success.
The one-MiB buffer is a technical allocation, not a time/volume policy.

Descriptors remain held read-only; inode aliases, final symlinks, nonregular and
unavailable parts refuse. Descriptor and path identity/size/timestamps are compared
after reading. This catches some changes but is not writer exclusion or ancestor
confinement. Do not modify archives during checks. IO/format failures publish safe
causes, never member payloads. Structured result has schema_version/status/members
(size, expected/actual hash)/errors/scope. Early failures may retain no member result;
never infer partial success. Codes: manifest_invalid, archive_source, archive_invalid,
archive_changed, member_missing, member_non_regular, member_duplicate, member_digest.
Exit 0 PASS, 1 controlled failure/refusal, 2 argparse syntax error.

No gzip/zip, automatic recovery/extraction, author authentication, whole-stream
hash or unselected-payload validation. A large header inventory can take time;
no arbitrary watchdog/retry/cutoff. Preserve refusals, correct provenance/parts/
expectations from evidence, then run a fresh check without overwriting originals.

## Git: observe the baseline before announcing it

```sh
python3 tools/verification/git_preflight.py --root /PATH/REPO --expected-branch main --tracking-ref refs/remotes/origin/main --source-ref SOURCE_COMMIT
```

All inputs explicit. Physical root must match the worktree. Source SHA works even
if its branch reference was removed. Observe HEAD/branch/tracking/source, ancestry
and ahead/behind counts. Wrong/detached branch, missing refs, unrelated source,
behind/diverged baselines refuse. Local-ahead is reported/allowed when the source
is included, not proof of publication. Tracking is locally recorded only; no fetch
or remote-freshness claim.

Dirty status/paths include rename endpoints and embedded newlines. Diff hash
covers tracked HEAD→index/worktree data; status hash does not hash untracked
contents. No dirty content is backed up. `checkpoint_required_before_mutation`
reminds a separate mutation path to preserve work first. Dirty work can coexist
with PASS if requested baseline is observed; no checkout occurs.

Optional Git locks and fsmonitor disabled; external diff/textconv avoided;
inherited GIT_* repository-selector overrides removed. Git still reads local
configuration: not a sandbox against hostile repositories/configuration. Two
local samples are compared and differences refuse, without locking writers or
untracked contents. JSON includes observed OIDs, equal/ahead/behind/diverged,
source_in_head, dirty and limits. Codes: git_command, git_io, git_root_mismatch,
head_missing, tracking_missing, source_missing, branch_mismatch, tracking_behind,
tracking_diverged, source_not_in_head, git_observation_changed, git_status_format,
git_count_format. Safe exit/cause types only, not private Git stderr. Never
silently substitute an empty success. Exit semantics match archive CLI. Correct
the baseline decision/precondition before rechecking. No switch/merge/reset/
commit/fetch/push is performed.

## Maintained checks and limits

`test_preflights.py` exercises actual CLIs on retained governed fixtures with raw
outputs, valid controls before faults/refusals and recovery. Fixture Git commits
and branches are fictional; no working-project refs changed. A modified second
Git observation is a deterministic test injection, not a concurrent writer lock.
Permanent recipe retains input and output evidence:

```sh
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/evidence-git-preflights.json --output /NEW/PATH
python3 tools/verification/recipe.py verify --evidence /NEW/PATH
```

No enrolled-human approval, model-injection immunity or universal tool coverage
established. Method tooling complements but does not replace separate R1 Trust
simulated-ledger evidence. Origin: QW3/QW4 in the
[Peter plan](../../docs/en/plans/AGENT-CONTROL-POCS.md).
