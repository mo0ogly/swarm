# PR40-OBS-01 — installation phase visibility

State at 2026-11-17T04:47:29.874451+00:00: source integrated in the isolated worktree,fake-only checks passed,independent review pending. Not a PR40 rerun or Docker/GitHub qualification.

## Need and scope

Historical install grouped output contract,native installation and Docker into one Actions step. Retained logs locate the wait in runtime-package construction without establishing its network cause. PR40 last observed17Nov01:22UTC remained blocked at f5d1f42f33ac730d1f0b830a172c0ff000bb18df;install cancelled16Nov06:27:30UTC after6h. Prepared change makes phases distinguishable in a future run; actual GitHub effect not observed. Old6+8PASS retained,not replayed.

Owner Swarm00. OneGLM packet proposed the retained16Nov Make/workflow patch plus maintained test. Requested/reported API modelglm-5.2,217.118s,reported48517tokens; no actual service version/cost inferred. Reserved Makefile,.github/workflows/ci.yml,tests/test_install_phase_targets.py,tools/verification/recipes/install-phase-targets.json. Base23df31007846882fe83bfc6ffea722227449c373. Other chats idle except09 bounded review.

## Current change

- Public make test-install remains. Separate recursive recipe lines preserve native-before-Docker ordering under-j; named phases are PHONY. Actual installer commands unchanged.
- Workflow replaces only grouped block with3named steps. Raw reconstruction/YAML comparison confirms all other bytes/fields,including triggers,install job,toolchain,permissions,always retention unchanged. No new trigger,deadline,retry or bypass.
- Test executes only a copied Makefile with inert python3 accepting the two installer arguments; real installer files are never copied/executed. Minimal child environment. Token checks are not a Make security parser or confinement.
- Required fresh --output leaf in caller-admitted existing parent; no implicit/tmp fallback. Permanent runner retains configured fixture placement,no repository/data/service relocation.

## Evidence and limits

Actual preimages Makefile7b7c104c…/workflowe6e34e4e… equal retained proposal. Postimages d271ecfa…/8c8db73c…,test e52cff13…,manifest2d308a31…. Exact inputs/argv at /run/media/fpizzi/SWARM/swarm-evidence/root00-pr40-phase-visibility-20261117.

Before curation: missingmake gave testsRun0/skipped1/wasSuccessfulTrue; inheritedMAKEFILES loaded an include writing an owned fixture marker. Corrected by prerequisite refusal,controlled environment and maintained CLI guards. Valid witness precedes each injection on samefixture. Separate empty-filter-success hypothesis disproved: installedPython exits5; principal diagnostic incorrectly expected0,no source edit manufactured a defect.

Five named methods PASS by directCLI(fake-qualification-01),then five through **new adapter** recipe→TMPDIR→explicit output(recipe-adapter-01); second passage qualifies new consumer,not historical native campaign. recipe.py verify confirms retained integrity/status without replay. Two distinct runs,not ten unique business tests. Sources,channels,argv/fake effects retained. Guards cover valid witness then missingmake,ambient config excluded,ordered success,native failure blocking fakeDocker,and recovery.

```sh
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/install-phase-targets.json --output /ADMITTED_PARENT/FRESH_EVIDENCE
python3 tools/verification/recipe.py verify --evidence /ADMITTED_PARENT/FRESH_EVIDENCE
```

Commands require allocation/admission; execute fakes,not real6+8. Declared sources/stdlib/make are not compiler/Docker-runtime qualification. Actual SWARM ext4/UUID checked,volume descriptor held through children/publication,host/output20GiB reserve preserved. Configured temporary fixtures local,evidence onSWARM.

## Continuous review and resumption

Policy:-j2,fakeexit3/64,names/count assertions are fixture data/protocol oracles,not operator settings. No user limit/deadline changed. Documentation:nontrivial functions/classes and Make targets documented,pairedFR/EN linked. Errors:missing prerequisite fails;argv/channels/status retained before assertion;fake native refusal blocks fakeDocker and restoration is observed.

No main merge,PR creation,workflow_dispatch,GitHub rerun,Docker/Cargo/provider or local installation. Application code unchanged;no version_history/restart appropriate for this CI tooling. Save WIPcommit/push after review;no PR implicitly launching6+8 without required precondition. OriginalAPT issue remains open:new verified Docker precondition still needed before single exceptionally authorized rerun. Native planner3to4/child2to3 human budget request remains pending and untouched.

Retrospective:scope a blocker to its contract; one fixture's pending budget must not hide an attributable independent source artifact. Preserve refusals/authority,no new permission or guaranteed future heartbeat effect. Next contexts receive index REPRISE-SUPERVISION-20261115.json,no general chat wake.

[Français](../../plans/actions-confiance/reprises-veille-x-20261108/PR40-PHASE-VISIBILITY-20261117.md)


## Review received — 2026-11-17T04:50:19.386855+00:00

09 turn01a1e7d2-e6f6-72e0-8ec7-c5e0d56afb73: APPROVE_SOURCE_AND_RETAINED_FAKE_PROOF,PR40-OBS-01 only. Four hashes/base23df3100… checked,11declared recipe artifacts reread/hashed;5namedtests/Ran5tests/OK/exit0/stable inventories correlated. Reviewer reran no test. GitHub/Docker/real6+8/install/budget remain open. Bounded checkpoint next,no merge.
