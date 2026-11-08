# Rust integration into main

Both histories are preserved. The Go engine removed by migration is not restored. Native persisted engine settings provide SQLite controls; Go guides and recipes are historical. Main training/Docker additions are retained. Historical preimages are not blended: the migration version stays at its path, and the main version remains in its parent commit.

The method checker runs the Rust agent_workflow target, requires nine named contracts and refuses empty collection. CI debug/incremental settings match the qualified local profile without removing checks or changing business policies. Local uncommitted work is preserved. CI checks/install remain required before merge; this document establishes neither independent acceptance nor real identity activation.

The first CI exposed two omitted test fixtures and an installation recipe assuming the evidence parent already existed. Referenced synthetic fixtures are published; both recipes create missing parents and still reject an existing output. Literal include review now covers every published Rust source rather than changed files only. No library unit test ran in that failed compilation.

The second CI ran 632 engine tests successfully and exposed six failures: four historical schema-version assertions and two traversals missing the Linux artifact error wrapper. Tests now preserve newer independently adopted settings groups; classification uses the shared typed error graph, still rejecting unknown and cleanup errors without granting access. Docker failures are retained and exposed in CI rather than reduced to an exit code without their cause.

The Docker context explicitly allows the five embedded configuration JSON files (provider wait, log reads, activity, graph performance, web requests). Distribution checks enumerate their literal includes and reject missing publication or Docker allowlisting; this does not replace an actual build. An incorrect module path in the new test was retained as a compilation failure and corrected.

The remaining historical fixtures now derive from catalogue introduced_schema_version: versions 40 then 39 for the report setting removal, and 40 then 41 for preparation-read settings. Newer group dependencies are no longer mixed into old settings; engine validation stays strict. The fixture builder is compiled for tests only.

All 639 engine tests pass; three identity-binding tests need a fixture in a genuinely different namespace. ubuntu-latest refused uid_map (EPERM). In the previous attempt, checks selected Ubuntu 22.04 and checks that prerequisite before compilation, without sudo, AppArmor/sysctl changes or skipped tests; install stays on ubuntu-latest. This does not qualify every Linux deployment. The CI owner must validate a compatible successor before that runner image retires. Sources: [Ubuntu restrictions](https://documentation.ubuntu.com/security/security-features/security-features-tables/), [GitHub Ubuntu 22.04 image](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2204-Readme.md).

The candidate also integrates both commits published during this integration (2bec348fc, 9ebb8e393): private Git binding/PAEA pack streams and remaining project files. Three README/release-note conflicts retain the new content and main’s Rust declaration. Method checks include the now-published pricing-discovery skill. Python 3.13 is explicitly selected for build tooling using hashlib.file_digest, absent from Ubuntu 22.04’s default Python 3.10. No private runtime files or executable Go source are restored.

The owner explicitly authorized namespace configuration on the disposable GitHub CI VM only. checks uses Ubuntu24.04 (recent kernel), retains kernel.apparmor_restrict_unprivileged_userns then temporarily sets it to0, requires the unshare fixture and restores the initial value in an always step. No local machine setting or test is removed. Python3.13 remains explicit; install retains its separate environment. The earlier Ubuntu22.04 attempt lacked pidfd namespace ioctls despite available unshare; it is no longer the current runner contract.

CI observed ETXTBSY while directly executing a just-created fixture script. The test now reproduces that condition with a retained writable descriptor, then verifies the same body through its explicit interpreter; the descriptor closes before Git verification. No sleep/retry or engine filter execution is added; marker, mutation refusal and recovery assertions remain required. The descriptor holder in the original failure has not been established.

The ETXTBSY witness uses the existing nix errno type, without adding a direct libc dependency. Its initial compilation failure is retained; it established no executed test.

The historical Go acceptance-runner unit test now injects fixture dependencies just as it already injected Go and DOM results. The real Rust checkout is explicitly checked: absent Go inputs => code3 before any launch. Failed Go, zero tests, DOM failure and unknown-case guards remain; this unit test neither qualifies Rust nor executes Go.

CI now retains cockpit recipe step channels and targeted browser errors. Work Roots, databases, token-bearing configuration, private transcripts and screenshots are excluded from that artifact. The initial opaque recipe failure is retained; a FAIL summary alone does not establish its cause.

The cockpit recipe expects the 21 scripts explicitly published by the Rust producer: the 19 native scripts (including the native instruction client) plus the two authority-console clients injected unconditionally by `cockpit_page` (`rust/crates/swarm-web/src/service/native_admin.rs`: `/native-runtime/authority-admin.js` → `web/native-authority-admin.js` and `/cockpit-authority.js`). It rejects missing/duplicate/inline/foreign-origin scripts, then compares protected HTTP bytes to versioned inputs (native quotas route maps to native-quotas.js; authority maps to native-authority-admin.js). The permanent recipe and driver inventory include that client. The original inventory refusal is retained; no product action or script is added. Realignment of 8 November 2026: merge 197d663f6 had combined main's strict 19-entry check with the branch's 21-script page (commit 3b41131dd, 3 November 2026, which added the two authority clients without updating the test's inventory); the CI `fr-etat-native-scripts-only` failure (run 42484108884) and its local reproduction (`.cache/existing-cockpit-razju52o`) are retained as witnesses; the test is realigned to the actual 21-script served inventory with strict guarantees unchanged (no missing, duplicate, inline or foreign script tolerated). Second semantic conflict of the same merge, same cause: f9ff58214 (31 October) had removed the empty-graph early return (the graph of a task-less Work now draws an empty svg, the "Aucune tâche" message and mounts the toolbar with the `#pilot-add-task` create control — behavior verified by `tests/rust_graph_navigation_ui.cjs`: `empty-graph-has-create-control`), without updating the `actual-empty-graph-after-selection` assertion that required no svg; the assertion is realigned to the delivered contract (message present, zero `.graph-noeud`, create control mounted). No CI ran on the branch between 29 October and 8 November: both contract breaks were only observable on the merged tree.

The Version recipe compares the local-change disclosure to the binary’s actual published modified flag instead of assuming a dirty build. After that real witness, explicitly marked client responses check clean/dirty rendering in FR/EN and each theme. Those fixtures never become binary metadata or an observed source comparison. The original clean-build CI failure is retained.

The check targets the Version component’s exact disclosure paragraph, not all historical release-note text, which can legitimately mention uncommitted changes. Clean/dirty variants also cover that vocabulary collision.

Third semantic conflict of the same merge, same root cause: wiring the mandatory
Biba I0 boundary into `PreparedExec::execute` (6706e201b, 30 October, per BT02 in
`docs/plans/actions-confiance/BIBA-TRANSVERSE-APEX.md`, "no unconfined fallback")
now refuses any workspace overlapping the Root (`biba-workspace-overlaps-root`,
`rust/crates/swarm-biba-linux/src/lib.rs`: `root.starts_with(&path)` is true for
equality). The default request of the `tests/rust_agent_model_route.py` fixture
set no workspace: the preflight (`agent_launch_preflight.rs::workspace`) resolves
an empty workspace to the Root itself, which main's boundary-less execution
(9075649c7: chdir + execve without Biba) tolerated. The delivered contract lives
elsewhere: `preparation_call_cli.rs::workspace_path` places workspaces deeper
than the Root precisely to pass both Biba guards, and the sibling tests
`rust_cockpit_biba_compatibility.py::make_i0_workspace` and
`rust_mission_worker_dispatch.py` (`workspace='dispatch-shared'`) mount a Root
child outside `.swarm`. The fixture is realigned to that contract: it now
creates an `i0-workspace` child (0700) and the default request carries it;
`run_and_assert` reads the provider proof (`model-received.json`) from that
workspace, and refusal assertions require no proof anywhere under the Root
(rglob), a faithful contraction of "no execution anywhere". Fixture consumers
that still read the proof at the Root after a successful run (nine files,
including `rust_native_completion_recovery.py`, `rust_scoped_worker.py`,
`rust_status_accepted.py`) remain failing on the merged tree — already-mapped
BT02 debt to realign during their documented requalification (BLP-APEX.md);
their current failure was already the Biba refusal itself, not a green pass.
Failure witnesses retained: `.swarm-fixtures/agent-logs-repro1` (Biba refusal)
and `.swarm-fixtures/agent-logs-repro2` (full green journey after realignment).
