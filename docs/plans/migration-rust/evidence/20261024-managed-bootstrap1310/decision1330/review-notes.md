# Local source review1330

Scope: exact five effective build inputs changed since normal1279; default
normal1319 has340inputs,110artifacts,4core/no fixture features. No installation,
independent acceptance, full migration or managed worker execution established.

managed_repository.rs: fixed private Git actions run via existing owned process
runner, saved git_command_timeout_seconds, optional monotonic deadline; no literal
90second product/test policy or retry. File-only transport, hooks/fsmonitor/Git
inherited overrides disabled; no checkout/filters or source writes. Typed Git
outputs remain privately retained, public codes static. sha40/64 are Git object
representations, hash64 SHA256, file mode0700 private storage invariant, not operator
choices. Dirty snapshots explicitly unported rather than a hidden byte ceiling.
All nontrivial helpers have contracts; inside/finish checks each descriptor/lock
close even after primary failure. Retained manifest strict shape/request/path/
objects checked; pending without bare/orphan/changed proofs refuse without clone
replacement. SQL rollback does not roll back filesystem preparation. Git core.fsync
and parent barriers do not establish hardware-crash acceptance. No arbitrary cleanup
removes a failed staging tree. Current paths are UTF8 request/model data, not an OS
confinement promise; same-UID and repository config authenticity remain external.

local_storage.rs: child() uses validated single names, mkdir/openat NOFOLLOW and
private owner checks; EEXIST only permits the authoritative open checks. Explicit
create bool is operation semantics, not a policy. process_path refers only to OUR
held descriptor/PID, never arbitrary target PID. rename_from uses actual source/
target descriptors plus NOREPLACE, preserves both sync failures and visible
publication uncertainty. Two actual filesystem tests prove symlink refusal,
retained inode after rename, no-replace conflict and owned recovery. No ownership
or descriptor provenance is inferred from filename or same UID alone.

work_engine.rs: original external intent remains intact; replay precedes IO.
Only Root adapter clones/strips repository for complete pure candidate prevalidation
and provider metadata observation, then attaches source-prepared repository. Pure
API still refuses. Existing managed claim/decision/launch/settlement refusals remain;
actual public claim refusal verified. Provider close failures precede repository
write; preparation/close failure rolls SQL back, retaining filesystem evidence.
Rustdoc now states filesystem effects survive rollback, no process/acceptance.
Nonmanaged provider bootstrap and six planning control tests pass unchanged.

lib.rs: Linux-private module registration only; no new public grant or feature.
version_history.json: FR/EN candidate note, no installed version claim. Other
existing dirty notes preserved. Build includes these exact bytes.

Public Python recipe: source Git commits use checkout's actual configured identity;
source regular files/index/HEAD/config hashes unchanged, committed blob read back,
dirty files excluded, actual protected HTTP/CLI saved Work equal, subproject correct.
Replay works with source hidden. Malformed/divergent/binding/missing object/orphan/
missing-bare cases preserve Work/proofs then restore actual originals. Owned Git
wrapper executes the real Git and creates only an empty conflicting final-manifest
directory after successful setup; this proves filesystem refusal and SQL rollback,
not a SQL failure injector. PATH selected explicitly and restored. Original1320
schema-refused trigger and1323 unselected-wrapper false precondition retained, no
production guard weakened. Final1327 seven named assertions pass; integrity verified.
No model process, positive SQL write, live database or global watchdog. New public
and nontrivial functions document purpose/preconditions/effects/errors/limits.

Native typed error traversal retains LocalStorageError's publication/cleanup and
AgentLaunchError cleanup children; CLI safe cause collection already understands
these types. No new dynamic-error-to-empty fallback introduced. Independent failure
in publication/close refuses even if SQL source outcome is uncertain.
