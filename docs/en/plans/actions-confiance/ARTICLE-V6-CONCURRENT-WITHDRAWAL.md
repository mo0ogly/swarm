# Concurrent withdrawal — protocol and evidence

Current status: one named concurrent native test PASS, two-step recipe and integrity verification PASS. Bounded observation of cooperative public revocation on the qualified fixture; no global H2 or installation. Earlier component/inert results retain their source scope. The initial protocol and retained failures below are historical.
[Detailed French protocol](../../../plans/actions-confiance/ARTICLE-V6-CONCURRENT-WITHDRAWAL.md).
Dependencies: settled P2 reconciliation and P3 lease contracts, qualified native
composition, and root00 allocation of a fresh physically accepted source fixture.
Historical accepted fixtures must not be copied as inode substitutes or mutated.

The planned barrier belongs inside `call_paea_git_promote`, after the final
`paea_git_agreement_match`, before `effect::commit`, while the actual shared
`reserve_trust(false)` remains held. A second fixture observation immediately
before the public withdrawal's exclusive flock must distinguish an entered
request from a merely spawned thread. These hooks do not exist yet and must
remain exclusive to the separately inventoried `test-fixtures` build.

First establish a healthy commit with the same authority, approval, candidate
transfer and mandate prerequisites. Then observe: shared section reached;
withdrawal request entered; actual exclusive contention on the same inode;
no effective withdrawal before barrier release; Git effect and journal result;
shared section release; persistent effective withdrawal; rejection of renewed
use without another effect; Reconcile separating history from current ref.
Barriers establish order; sleeps and elapsed time are not evidence. A nonblocking
exclusive probe requires a valid outside-section control. Failed prerequisites
must release resources and cannot strand another actor behind an abandoned barrier.

Declare exact effective sources, manifests, features, profile, binary and helpers
before allocation; the P1 2109-file composition is not a complete native graph.
Use the supported `tests/rust_native_first_publication.py` path with a new output,
no historical reuse or direct acceptance-table writes. Retain real root/work,
candidate, independent review, acceptance and physical attribution, beyond PASS.
Read the final promotion fixture helpers and update their obsolete P2 assertions.
Run the named concurrent test alone, retain argv/stdout/stderr/source hashes and
progress events, and apply no arbitrary global watchdog. Publish a permanent
recipe with matching FR/EN documentation alongside the eventual implementation.

This will observe serialization in one specific public path. It cannot establish
global tool mediation, expiry at Git's effect point, or a delivered native binary.
Lease clocks, reference publication and local installation remain separate claims.

## Discriminating fixture design

Two planned hooks must be scoped to one thread/invocation, cleaned on panic,
and compiled only with `test-fixtures`: after Commit's final approval match,
and immediately before the withdrawal's actual exclusive flock. The latter
passes only the actual lock file's device/inode witness to the controller.
No such API exists yet.

The controller must never add its own shared lock, which would mask removal of
the production reservation. A nonblocking exclusive probe on an independent
opening of that same inode must contend solely because of the actual Commit.
Without the production shared reservation, that expected contention cannot hold.
Release barriers and join actors before propagating assertion failures.
A signal merely before calling withdrawal, or an empty channel from a possibly
unscheduled thread, cannot establish entry. Use fresh service/event identities
after the valid signed Commit control, existing public Trust operations, and
public persisted withdrawal/Reconcile observations; never direct database writes.
The barrier does not replace the lease contract.

### Attribute contention to the correct actor

A controller-side probe might contend with withdrawal's own exclusive lock and
produce a false witness. The withdrawal fixture hook must therefore attempt a
nonblocking exclusive flock on the actual file it just opened, before the real
blocking flock, while Commit remains held at its release barrier. Report the
result and inode witness, never the descriptor. If the unexpected attempt
acquires the lock, explicitly release it before reporting failure. Existing
rustix producer vocabulary is `FlockOperation::NonBlockingLockExclusive`.
This distinguishes Commit's shared reservation without elapsed-time or scheduler
assumptions or contention caused by a different actor.

Send progress before every barrier rendezvous; an early failure must emit a
terminal event rather than strand the controller. Release rendezvous and join
actors before asserting, retain cleanup causes and diagnostic fixtures, and
ensure the normal non-fixture profile contains no observation hooks.


## Integrated candidate — 18 November 2026

The earlier design paragraphs retain the initial contract; the final
contention point uses the descriptor actually opened by revocation before its
blocking flock, rather than an extra controller probe.

The candidate adds `paea_git_fixture` under Linux + `test-fixtures`, two
thread-local RAII hook states, one insertion after Commit's final agreement
match, and one insertion exclusively before `reserve_trust`'s EX flock.
Callbacks are consumed once; same-kind re-registration is refused while the
scope lives, including after consumption; return and unwind clear state.
Errors use existing cleanup paths. No new operator policy, timeout, retry or
production default is introduced.

The named test `promote_call_concurrent_withdrawal_waits_for_shared_section`
is appended to integration target `paea_git_promote_call`, retaining `#[ignore]`
and requiring explicit `--ignored --exact` selection. An actual signed Commit
on a fresh service precedes injection; public revocation outside the section
controls successful EX acquisition and explicit unlock. The concurrent scenario
uses a different service and events, the same signed path and the newly admitted
native fixture. The controller holds no additional SH lock. Public revocation
tries EX on its actual checked file, sends device/inode and EAGAIN, and waits
at a barrier before the blocking flock. Commit waits at its own barrier.
Both workers are joined before assertions; typed outcomes and close failures
remain separate. No sleeps, receive deadlines or global watchdog establish order.

These calls retain service and grant fixtures before concurrency. Historical
preparation/transfer helpers retain their existing cleanup contract. After
revocation, assertions cover absent agreement, refused renewal without ref
change, and Reconcile's separate recorded history and current ref observation.

Permanent recipe: `tools/verification/recipes/paea-concurrent-withdrawal.json`.
It requires a new `SWARM_PAEA_SOURCE_FIXTURE` and external Cargo-target admission;
it does not qualify the delivered profile. The six hook guards, integration
compilation, default native profile and concurrent body remain NOT EXECUTED.
Rustfmt syntax validation establishes none of them. The original GLM proposal
and defects (BarrierWaitResult, invalid control, ignored errors) are retained
in P4 evidence. Principal supplementation is limited to this bounded packet.


### Dedicated positive fixture (source, not executed)

Swarm00 reserved `tests/paea_concurrent_withdrawal_source.py`, now integrated
from absence. It replaces the earlier historical-campaign proposal: no run()
from first-publication/reviewer/inspection/validation/auto-worker/planning.
Only public birth, source Git commit, two inert planners, one task and actual
manual inert worker creating its report, public immutable retention, three
controls, one inert reviewer, preview and one public publication. No historical
copy, corruption, injected refusal, refund or SQL write. Budgets are explicit
fixture inputs, not production defaults.

`observed-publication.json` retains the observed effect without qualification.
The final `result.json` PASS follows final binary/profile/helper hashes, actual
candidate/review originals and marker checks, accepted readback and unchanged
device/inode. Retained observation hashes are bound in result. A late failure
may leave a visible result: the outer runner must retain exit=0, reject
failure.json, reread input/original/artifact hashes and publish its own receipt
bound to the result hash before supplying the fixture to the test. PASS presence
alone is insufficient; that receipt consumer remains to integrate and review.


### Review 09: candidate repairs, without execution

Retained REQUEST_CHANGES found authority TempDir deletion on return,
progress assertions hiding typed results, and capture destruction under a
borrow_mut causing panic/abort during unwind. The sole authorized shared-fixture
anchor adds Fixture::retain_directory: directory retention only, neither sync
nor implicit journal close. The controller owns the fixture and retains startup,
use, app.close and both independently closed original journals. Both threads
join before assertion; RetainedFailure combines original panic, typed results
and cleanup. Safe native_error_links projections aid diagnosis without replacing
owned causes or rendering private messages.

ScopeGuard extracts captures while borrowed, destroys outside the borrow while
remaining active, records cleanup panic and clears state. The body is caught
before cleanup: ScopeError::Cleanup retains its exact return; HookUnwind retains
both panics. Three module guards cover reentry, returned-body cleanup failure
and simultaneous panics; two integration component guards cover failure bundles
and actual fixture retention without a native source lifecycle. Planned count:
nine module guards, two component guards, one ignored concurrent body. None have
run; Rustfmt/AST are syntax checks only.

Use the fresh helper with --supervise: one child, no retry. Its terminal receipt
binds actual zero exit, result hash and process record. The test rejects missing
receipt, visible failure, mismatched hashes or changed originals/observations,
and checks actual device/inode. These files establish no H2 verdict, delivered
profile or installation. Original pin manifests remain historical.


### Residual R09-2 — retain the EX control

Review r2 found two unwraps evaluated before storing the EX control's
nested result. retain_ex_witness now saves the complete nested value in an
outer variable before any assertion; the control failure bundle retains it
alongside the call, close failures and original panic. The new
ex_witness_error_is_retained_before_assertion guard starts with a valid control,
then retains an injected errno across assertion. It checks generic result
retention, not kernel contention or public revocation. Planned count: nine
module guards, three components, one ignored concurrent body; none executed.
The r2 pins remain historical.


### Q1 — observed components, native scenario still not executed

Nine module guards and three named components pass on a new local frozen
composition of3498 inputs. The ignored body is typechecked, not executed.
Cargo artifacts/features, test profiles and hashes bind two retained ELF copies.
Actual depfiles identify539 workspace sources,1070 registry sources and three
generated inputs; this does not qualify the entire C/header/system/proc-macro
trust graph or the delivered profile.

The identical API-compatible reentry guard, preceded by its valid control,
fails at runtime against the old module: one test, one RefCell already borrowed
failure after successful compilation. Only that module plus the guard differs
from the candidate copy. No old double-panic test was run. The old ELF is retained
separately. Cargo reuses its executable pathname: after this counterexample the
cache unit ELF is old. Both candidate copies were hash-rechecked; never identify
qualified bytes by executable filename alone.

The first attempt refused before rustc because a declared example was missing
from the3125-input copy; zero artifacts/tests. Recovery retains that failure and
compares actual-checkout manifests/metadata with the new copy:343 targets in17
packages, including52 explicit declarations; names, kinds, source paths and
features match. Five CARGO_MANIFEST_DIR concatenations resolve to the retained
fixture file. Metadata from a truncated copy cannot prove completeness.
211 crate archives are bound to Cargo.lock with11887 extracted files verified.
The absent legacy .cargo-checksum.json assumption is retained as a method failure;
no build preceded its repair. Registry candidates are not all effective inputs.

Evidence: article-v6-withdrawal-f8vbbjjd/q1/recovery-r1 and q1/old-hook-reproducer
on admitted SWARM storage. No fresh helper, default profile, native concurrent
test, service or installation ran. Four inert Python qualification guards are
proposed separately and remain unexecuted.


### Prepared inert qualification guards (not executed)

tests/test_paea_withdrawal_source_qualification.py contains four Python
supervisor cases: intact control, visible PASS with nonzero child, incomplete
result after exit zero, and divergent artifact digest. Every refusal follows
an intact control under the same prerequisites. subprocess.run is mocked with
CompletedProcess: no real child, CLI, public helper, kernel, provider, native
Root or acceptance runs. Synthetic fixture originals are retained. These guards
cover supervise(), not the Rust consumer or genuine native qualification.
The recipe includes them and actual package sources needed by Cargo parsing;
it grants no authority for native steps. No new execution yet. Declared total:
four inert guards, nine module guards, three components and one ignored body;
the complete recipe has not run.


### Q2 — four inert guards PASS and qualified default image

The four named SourceQualificationGuards methods pass with retained names,
exit zero and unchanged before/after sources. subprocess.run is mocked: no
public helper, actual business child/provider or native Root ran. The isolated
copy contains38 Python sources plus the real fixture-area defaults read on
import; synthetic fixture originals are archived.

After that prerequisite, a fresh default Cargo target distinct from the fixture
target was admitted, preserving five prior entries. A new3499-input local copy
binds configuration after registration; actual/frozen metadata targets/features
match (343targets/52explicit). verify_native_profile exits zero:206observed
artifacts, four core artifacts with empty features and test=false; none carry
test-fixtures/test-util.790effective depfile inputs are archived and read back;
paea_git_fixture is absent. This does not qualify every C/header/OS/proc-macro
input, any business execution, or installation.

Frozen LOCAL image SHA-256:
f429fde5cbe566cc1d741d60c15d53f6b4c5db46be2a4c71859f49e126e1f3fc.
q2b/terminal.json binds image identity, actual exit zero, profile hash, stable
sources/registry and installed=false. Profile/result and raw Cargo channels are
under article-v6-withdrawal-f8vbbjjd/q2b. No execution of this image, fresh public
helper, ignored body, service, merge or installation occurred. Q3 still requires
allocation after evidence handoff. Nine guards/three components are not rerun
for Python/docs deltas; previous manifests keep their exact scope.


### Q3A — retained failure and SOURCE producer correction

The single allocated Q3A ended exit1 without qualification. Its manual worker
completed but the automatic relay excludes it: mission_report_relay.rs:143
requires native_mission or conductor origin, then producer() requires
AutomaticWorker. task_report.rs:160–164 checks an already retained result rather
than providing manual bootstrap. Current production retain_result calls lead
to the automatic relay; other references are internal or tests. Engine rights
are not relaxed to make this helper pass.

Only the pidfd/argv/root/hash-bound owned watch received SIGINT. Final events
record children_still_running=0, lease_released=true, root_closed=true; the
public pause returned0. Child/supervisor exit1, failures/process/originals and
actual root remain retained, with no terminal receipt. Eighteen prior commands
and estimated consumption remain historical; no refund/reset or copied accepted
root. Drained events also show scoped-worker-retry-decision-required. Empty
buffered stdout was not proof of no events; the new collector flushes/fsyncs
individual frames.

The source correction uses one CLI mission watch conductor from a fresh TODO,
no agent and paused planning, checked before Popen. dispatch_mission_worker and
_mission_worker_entry supply actual native admission; the helper never writes
origin/native_mission. Validation and review guards return None while planning
is paused (task_validation.rs:490, reviewer_native/runtime.rs:372), so no extra
implicit calls occur. One inert worker, no server. Stop requires exact handoff
AND helper exit0 for that agent; then SIGINT, drain/wait, root/lease/children
receipt, public pause and actual native-producer readback. Primary/cleanup
failures are combined; only verified adopted helpers of this root may be cleaned
under the existing subreaper contract. EOF/terminal refusal/blocked fresh inputs
never become silent waiting or success.

Four additional inert guards cover manual-state preflight before Popen, both
completion events, preserved refusal/nonzero exit, and unpaused planning refusal.
Valid controls precede faults. Eight Python methods are now planned, NOT RUN on
this delta. Four Q2 passes retain their previous byte scope. Q1 Rust sources and
default f429 image are unchanged; no rebuild, new birth or concurrent body ran.
Fresh source review and allocation are required.

Our earlier review defect: static import closure and four supervisor mocks did
not prove relay-producer eligibility. Read producer/admission/consumer/cleanup
before reusing a recipe. The separate >7h supervision incident remains retained:
active chat did not establish active calculation; no quota/tool cause was proven.
New source writes establish resumption, not native repair.


### Source review — transient observations are not terminal

Retained finding09 cursor134: managed-operation-busy and NoAdmission are
engine observations eligible for later reobservation. AutomaticProgress no
longer refuses waiting after exit0 or mission-watch-blocked after departure.
The engine keeps saved cadence and all admission guards; this collector retries
no provider and adds no duration/attempt ceiling. Waiting/blocked frames are
retained without treating them as success or final refusal. Nonzero worker
exit, stop/EOF before handoff, malformed identity and additional departure or
adoption remain terminal verification failures.

Written regressions: started/waiting/exited0/submitted, waiting+blocked after
exit0, stop before handoff refusal and nonzero exit refusal, each with valid
controls. Eleven inert methods are planned, none executed on this delta; the
prior eight had not run. Public birth/concurrent body remain unallocated until
new review. Rust/default f429 image and product rights are unchanged.


### Q3A-R1 — eleven inert guards observed

test_waiting_before_exit_can_later_submit loads the captured old helper,
passes its valid_progress control, then raises AutomaticSourceRefused at the
waiting/exited0 transition. One test, one unittest runtime ERROR, exit1; neither
import failure nor compilation refusal. Separate old source/API bytes retained.

All eleven candidate methods then pass, exit0, on38 Python sources plus the
actual fixture-area defaults JSON. Both input sets remain stable before/after;
raw channels, names and source/inert-fixture archives are under
article-v6-withdrawal-f8vbbjjd/q3a-r1-guards. No actual helper Popen/run, business
CLI, provider or native source birth occurred. These passes do not verify the
public automatic lifecycle. Rust, default f429 image and fixture d499 ELF remain
unchanged; new public allocation is still required. Concurrent body not executed.


### Q3A-R2 — qualified new public lifecycle

One newly allocated attempt after the fix and eleven guards exits zero.
The original new local root remains unmoved at
/home/fpizzi/Documents/ChatGPT/swarm/.cache/article-v6-p4-q3a-r2-lu8sy44x/.swarm-fixtures/swarm-model83-7x03krbe,
.swarm device64513/inode18134681, work w-87c690d138b64e98ac6a3fca,
candidate3b883af5808a51014c73e6a6ec4adb77566a2e50. This composition now holds
physical evidence and must not be treated as disposable cache without owner
review; documentation does not itself enforce cleanup exclusion. No copy may
substitute the physically accepted root.

Actual sequence: watch-started, worker-started, worker-exited0,
report-submitted, watch-stopped(sigint). Final receipt reports children0,
lease_released/root_closed true; public pause exits zero and producer readback
is completed/exit0 with an actual native_mission object. After conductor close,
three controls, one inert reviewer and one public publication pass; accepted
readback retained. This proves neither human review nor a real LLM.

source/terminal.json binds actual child exit zero and result; outer checks
revalidate profile/local f429 image, code, artifacts, actual candidate/review
originals, process marker and inode, without failure markers. Result SHA
b0476dc5ac9cdef174db42d3d248d497a61ab41887b26e92d1551e33c14f58dc;
terminal585b86dd81c0fd5a31fa5c0cbdec487a7bbfc2df0eee42a9671f6b93efc26452.
Primaries: article-v6-withdrawal-f8vbbjjd/q3a-r2. Earlier failures are unchanged.
The concurrent body has not run; separate Q3B allocation still required. The
outer-runner SyntaxWarning and executed bytes are retained; one whitespace
fix for future reuse did not trigger any reexecution.


### Q3B — retained failure, test schema consumer requires correction

The single allocated native test exited101 (1 named, 0 passed, 1 failed), after
the valid signed Commit control and positive exclusive probe. Same-inode EAGAIN,
unchanged Trust while SH was held, recorded Commit and public revoked reply
checks passed. Readback wrongly expected mandates[id].state: State.public
projects raw mandates with revoked_at and a separate mandate_status map. The
retained original contains the concurrent mandate’s revoked_at.

Cause: an invented test consumer contract. This does not establish a withdrawal
failure. Renewed-use refusal and Reconcile were not reached. Correct the test
assertions rather than production schema or persisted state. A new compile/run
requires separate allocation; no unchanged replay. Primaries:
article-v6-withdrawal-f8vbbjjd/q3b, including stdout/stderr/result/invocation/
diagnosis and source-before.rs. Retained d499 ELF and compiled Rust inputs stayed
stable. No installation or global H2 claim.

Q3B-R1 source correction prepared: assert exact id, valid RFC3339 revoked_at and effective mandate_status “mandat révoqué”, matching the inspected producers. No new production field, deadline, policy or fallback. Only the test function changed, with its contract comment updated; parse/assertion failures fail the test instead of empty success. Not compiled or executed.


### Q3B-R1-PREP — compile and fresh birth, method incident remains open

The corrected integration ELF compiled without running tests: exit0, successful
build-finished,118 artifacts, stable source/registry. Retained ELF SHA256
08b99e72de0794d2dc22a9a66620775e25c2cbe22df8088fb784aaae3cc5dce1,
test-fixtures/test profile, separate from default f429. Local composition:
.cache/article-v6-p4-q3b-r1-uud5mnox. Only the test schema assertion changed
among Rust inputs since Q1.

The depfile verifier first resolved relative paths from the composition root
instead of its rust directory, failing its assertion. An incorrectly sequenced
shell command nevertheless started dependent birth B: a method violation
retained in method-incident.json. The corrected resolver normalizes from rust:
101 depfiles,520 declared workspace sources,1070 registry and3 generated inputs;
none missing or mismatched. No repeated build. Future dependent stages use
separate tool calls and the future runner requires the depfile receipt before
creating anything. Executed runner bytes are retained separately. These changes
do not erase the earlier incident.

B completed cleanly with child/supervisor exit0; Swarm00 must decide acceptance
because it started prematurely. Raw receipt: q3a-r3. Original LOCAL Root:
.cache/article-v6-p4-q3a-r3-dqn_ov3t/.swarm-fixtures/swarm-model83-l1k8i7mu,
.swarm dev64513/inode19057820; do not clean or substitute a copy. Q3A-R2/Q3B
originals retain their mandates and the initial failure. The corrected concurrent
body has not run. Primaries: article-v6-withdrawal-f8vbbjjd/q3b-r1-compile and q3a-r3.


### Executable preflight after the sequencing incident

Swarm00 separately admitted A/B outputs after the corrected audit, without
approving historical sequencing. Decision: Q3B-R1-PREP-RECOVERY-DECISION.json
in root00-p4-review-z02o085q. The method defect remains in the retrospective.

New tools/verification/paea_withdrawal_preflight.py checks the hash-bound root
receipt, actual compiled sources, depfiles/dependencies, ELF/profile, fresh
receipts, originals, marker, inode and default image/profile. Missing or changed
prerequisites refuse. Storage admission FDs and persisted space reserve are
held; the target must already exist. Check-only mode launches no test. Separately
authorized --execute rechecks the same inputs, holds FDs through the child and
preserves both primary and post-execution verification failures. Neither hashes
nor local receipts establish trusted authorship or grant execution authority.

Bounded recipe: tools/verification/recipes/paea-concurrent-withdrawal-qualified.json.
This retained-ELF recipe complements the full component-guard recipe; no Cargo.
Its first step is the preflight, followed by the one exact native test. The
permanent runner stops on first-step exit/assertion/source drift. Supply explicit
SWARM_PAEA_PREFLIGHT_MANIFEST and SWARM_PAEA_PREFLIGHT_SHA256, retained with
the invocation; these variables confer no authority. Real recipe not run yet.

Three named inert guards passed, exit0 with stable sources: missing prerequisite,
wrong hash, combined step/post-check failure. Each fault follows a valid control.
Real run_recipe uses mocked subprocesses; refusals reach only the first step,
never the second, and real Popen is forbidden. Evidence:
q3b-r1-preflight-inert-r1. The earlier two-guard revision remains separately
retained. No Rust test, native CLI, provider or service runs in these guards.

Delta review: no new operator policy; reserve comes from persisted settings,
and field names are receipt contracts or fixture expectations. Nontrivial
functions document prerequisites/results/errors/limits. IO/JSON/hash failures
propagate, never empty success. Test functions document their doubles. Pending
source review and separate native-body allocation; no complete H2 or installation.


### Final preflight revision after reviewer09 REQUEST_CHANGES

Review09 rejected the earlier revision: parallel policy validation accepted zero,
immutable inputs were not reread after the child, and the old manifest lacked
receipts. Those findings remain retained. Current code reuses
cargo_space_guard.load_policy with an explicit path (exact schema, positive
minimum). Inert doubles use positive policy and sufficient simulated space,
never a reduced production reserve. Post-step checking rereads compiled sources,
ELF, manifests, depfiles, receipts, code, marker and physical inode; mutable Trust
state and newly created service directories are excluded. Both child failure
and post-check failure survive together.

Receipt/dependency maps are themselves hash-bound, as is source-inputs to
ELF.source_inputs_sha256. The only manifest proposed for execution is
q3b-r1-preflight-final-manifest.json, SHA256
66d2ba4401e58a328c9d4b59f3e89fde63da1ac3a543c3107dc20f9b1bdf486f.
Historical c5e4… without receipts and intermediate745a… are superseded.
The real preflight has not run.

Compatible preimage reproductions follow valid controls: zero reserve yields
one assertion FAIL; missing immutable post-check yields one runtime ERROR,
not import/build failures. Seven current candidate guards pass with exit0 and
stable sources. Seven inert roots are indexed; eleven recipe bundles verified:
seven positive controls and four expected refusals reaching only one step.
Evidence: q3b-r1-preflight-inert-final. Earlier2/3/4-guard results retain their
original source scope. Policy/post-check changes justify this guard set.

The only later recipe delta inventories the shared policy validator; load_recipe
accepts the final schema, with unchanged steps/commands. This is a static check,
not a native recipe run. Final frozen packet submitted for review; only concrete
findings justify further edits and replacement pins. Concurrent body not run yet.


### Q3B-R1 — verified native concurrent observation

After reviewer09 APPROVE_SOURCE_AND_INERT_EVIDENCE_ONLY and separate root
allocation, exactly one retained-ELF recipe ran. Real preflight PASS, then one
exact concurrent native test PASS:1 named,0 failed,0 ignored,8 filtered,5.63s.
Recipe exit0; recipe verify exit0. All14pins, ELF08b99… and manifest66d2…
remained stable before/after, with outer/inner FD storage admission. No source
changed during execution.

The signed Commit control and free EX probe passed. During Commit’s actual SH
section, public revocation reached the EX path and observed EAGAIN on the checked
FD, matching lock device/inode64513:19057828. Trust remained unchanged while SH
was held. After release, recorded Commit, candidate ref and publicly revoked
mandate were verified. Renewed signed use refused agreement-stale without a
new ref. Reconcile separately returned historical effect and current ref.
Joins/closes and causes were checked; startup/action/cleanup diagnostics had
no errors.

Original LOCAL source Root: .cache/article-v6-p4-q3a-r3-dqn_ov3t/.swarm-fixtures/
swarm-model83-l1k8i7mu, .swarm device64513/inode19057820 (distinct from the lock
file inode). Two isolated services and four signed authority fixtures remain
retained and indexed in stdout; temporary names do not grant cleanup authority.
No real LLM, accepted server, human presence or OS confinement was established.

Primaries: article-v6-withdrawal-f8vbbjjd/q3b-r1-run/{invocation,result,verify}.json
and recipe/step-0002.{stdout,stderr}, permanent-runner hashes/archive. Earlier
failures and method incidents remain intact, without attempt refunds or reused
PASS attribution. Bounded cooperative H2 observation only; no physical COMMIT
expiry guarantee, global PV2, main integration or installation. Final combined
review remains required.
