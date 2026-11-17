# Offline observation of a declared tree

`trace_process_tree.parse_declared_tree(texts, nodes, edges)` is pure: no CLI,
file IO, children, hashing or admission. Caller supplies full per-PID strace
texts, custody/hashes and an independently declared tree. Python dictionaries
cannot recover duplicate keys already collapsed upstream; decoding is caller's
responsibility.

Exact node keys: executable, argv, environment, exit_code. Strict types,
positive integer PIDs (bool refused), normal exit 0..255 (protocol representation,
not quota). Env names nonempty without '=' or NUL; values without NUL so
NAME=VALUE preserves mapping meaning. Declared UTF-8 strings compare with -xx
bytes; this is not general non-UTF-8 Unix filename support. Require one root,
no cycles/duplicate edges/multiple parents/disconnection. One-node trees allowed.

Selected grammar: successful -xx/-v execve using existing EXEC/hex_array helpers,
successful sample-flagged clone3, WIFEXITED/WEXITSTATUS wait4, exit_group and normal
terminal. Stack addresses/sizes remain observed data, not invented kernel
constants. Compare executable separately from declared argv[0]; declared spoof
is valid. Each clone is followed by its wait in the parent; status matches child
terminal. Arbitrary concurrent creation is outside scope: each clone must be
waited before the next. Unsupported, abbreviated/unfinished/resumed, signal
terminal, legacy fork/clone/vfork/execveat/waitid or malformed selected-family
lines raise short-reason ValueError, never empty success.

Output nodes/edges retain raw selected lines and numbers, observation_scope and
per-PID unexamined_lines. Other lines, including SIGCHLD/Python startup, remain
uninterpreted with explicit numbers/counts and caller-retained originals.
complete_build_qualification and behavioral_admission are always false. No
universal graph, per-syscall inode identity or authenticity from supplied text.

Output includes plaintext argv/env and exec lines. Caller must authorize those
data and keep artifacts private. This is no secret filter; current controlled
fixture uses only LC_ALL=C. It qualifies no arbitrary descendant/Cargo env.

`recipes/trace-process-tree-inert.json` declares eight synthetic child-free
guards. First run of the initial seven: five PASS, two FAIL (malformed selected name silently
unexamined; '=' in env name changes mapping meaning). Same two guards PASS after
correction with valid control/fresh recovery; five initial PASS reused in scope,
not replayed. Full recipe not run after correction. Original refusals/phases:
root00-cd01-process-tree-0bitlzr_ on SWARM.

Offline original worker564552 traces yield three nodes, two edges and 13 selected
process-family lines; 2617 explicitly unexamined lines remain in hashed originals.
No child replay or reassignment to current worker. OFFLINE-ALLOCATION.json and
OFFLINE-OBSERVATION.json bind declarations, sources/tools and original bytes.
Complete compiler closure, C-D01 admission and 12+3 bodies remain open; no served
product change, installation or application version change.

PT-01 independent review: <... wait4 resumed> does not contain the incorrect
"... resumed>" needle. Eighth selected guard with control/recovery FAIL→PASS
after rejecting continuation-prefixed <... records. Earlier guards and offline
analysis retain older source bindings, no replay/reassignment to PT-01 bytes.
